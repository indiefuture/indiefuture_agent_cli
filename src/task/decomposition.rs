use crate::ai::{AiClient, Message, MessageRole};
use crate::ai::prompt::{create_subtask_functions, function_name_to_operation_type};
use crate::error::{AgentError, AgentResult};
use crate::task::{OperationType, Task, TaskStatus};
use crate::utils;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

const DECOMPOSITION_PROMPT: &str = r#"
You are an AI assistant that specializes in breaking down complex tasks into smaller, manageable subtasks.

IMPORTANT: Analyze this task and break it down into a sequence of subtasks using the function calls provided to you.

When creating your plan of subtasks, think about:
1. What information needs to be gathered first
2. What dependencies exist between subtasks
3. What discrete steps are needed to complete the task
4. What verification steps should be included

The functions provided allow you to create the following types of subtasks:
- execute_search: Search for files based on a query
- execute_read: Read specific file paths to analyze content
- execute_update: Update files with new content
- execute_bash: Execute bash commands on the system
- execute_task: Process input and respond with analysis

For each subtask:
- Make it specific and actionable
- Include all necessary context
- Make it self-contained so it can be executed independently
- Choose the appropriate function for the operation type

Each function call you make will result in a subtask added to the execution queue.
Subtasks will be executed in the order you create them, respecting any dependencies.

Main Task: {task_description}
"#;

#[derive(Debug, Serialize, Deserialize)]
struct SubtaskSpec {
    description: String,
    #[serde(deserialize_with = "deserialize_dependencies")]
    dependencies: Vec<usize>,
    estimated_time_seconds: u64,
    priority: u8,
    #[serde(default)]
    operation_type: Option<String>, // Keep as string for deserialization, we'll convert to enum later
}

/// Custom deserializer that can handle both strings and numbers for dependencies
fn deserialize_dependencies<'de, D>(deserializer: D) -> Result<Vec<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    
    // First, try to deserialize as a Vec<serde_json::Value>
    let values = Vec::<serde_json::Value>::deserialize(deserializer)?;
    
    // Convert each value to a usize
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        let idx = match value {
            serde_json::Value::Number(num) => {
                num.as_u64()
                    .ok_or_else(|| Error::custom("Invalid dependency index: not a positive number"))?
                    as usize
            }
            serde_json::Value::String(s) => {
                s.parse::<usize>()
                    .map_err(|_| Error::custom(format!("Invalid dependency index: '{}' is not a number", s)))?
            }
            _ => {
                return Err(Error::custom("Dependencies must be numbers or strings"));
            }
        };
        result.push(idx);
    }
    
    Ok(result)
}

#[derive(Debug, Serialize, Deserialize)]
struct DecompositionResponse {
    subtasks: Vec<SubtaskSpec>,
}

/// Handles breaking down complex tasks into subtasks
pub struct TaskDecomposer {
    ai_client: Arc<dyn AiClient>,
    default_timeout: Duration,
}

impl TaskDecomposer {
    pub fn new(ai_client: Box<dyn AiClient>, default_timeout_seconds: u64) -> Self {
        // Box to Arc conversion by boxing the client then wrapping in Arc
        let ai_client_arc: Arc<dyn AiClient> = match ai_client.clone_box() {
            boxed => Arc::from(boxed)
        };
        
        Self {
            ai_client: ai_client_arc,
            default_timeout: Duration::from_secs(default_timeout_seconds),
        }
    }

    /// Break down a task into smaller subtasks using OpenAI functions
    pub async fn decompose(&self, task_description: &str) -> AgentResult<Vec<Task>> {
        // Check if this is a simple task that doesn't need decomposition
        if let Some(simple_tasks) = self.handle_simple_task(task_description) {
            log::info!("🧠 Detected simple task, skipping decomposition");
            return Ok(simple_tasks);
        }
        
        // Prepare the prompt
        let prompt = DECOMPOSITION_PROMPT.replace("{task_description}", task_description);
        
        // Get the function definitions
        let functions = create_subtask_functions();
        
        // Call the AI service with function calling enabled
        let messages = vec![
            Message {
                role: MessageRole::System,
                content: prompt.to_string(),
            }
        ];
        
        // Get the response with function calls
        // TODO: Update the AiClient trait to support functions
        let response = self.ai_client.generate_text_with_functions(
            messages, 
            functions,
            Some("auto")
        ).await?;
        
        // Parse the function calls from the response
        // Each function call represents a subtask with a specific operation type
        let parent_id = utils::generate_id();
        let subtasks = self.parse_function_calls(response, parent_id, task_description)?;
        
        Ok(subtasks)
    }
    
    /// Handle simple tasks directly without calling the AI
    fn handle_simple_task(&self, task_description: &str) -> Option<Vec<Task>> {
        // Check for common Rust/Cargo commands
        let task_lower = task_description.to_lowercase();
        
        // Create a parent task ID
        let parent_id = utils::generate_id();
        
        // Direct cargo commands don't need decomposition
        if task_lower.contains("cargo check") || 
           task_lower.contains("cargo build") || 
           task_lower.contains("cargo run") || 
           task_lower.contains("cargo test") ||
           task_lower.contains("cargo fmt") ||
           task_lower.contains("cargo clippy") {
            
            // Extract the command
            let command = if task_lower.contains("cargo check") {
                "cargo check"
            } else if task_lower.contains("cargo build") {
                "cargo build"
            } else if task_lower.contains("cargo run") {
                "cargo run"
            } else if task_lower.contains("cargo test") {
                "cargo test"
            } else if task_lower.contains("cargo fmt") {
                "cargo fmt"
            } else if task_lower.contains("cargo clippy") {
                "cargo clippy"
            } else {
                return None;
            };
            
            // Create main task
            let main_task = Task::new(
                parent_id.clone(),
                task_description.to_string(),
                None,
                Vec::new(),
                self.default_timeout,
                1,
            ).with_operation(OperationType::TASK);
            
            // Create the BASH task
            let bash_task = Task::new(
                utils::generate_id(),
                format!("Execute command: {}", command),
                Some(parent_id.clone()),
                Vec::new(),
                self.default_timeout,
                1,
            ).with_operation(OperationType::BASH);
            
            // Return the tasks
            return Some(vec![main_task, bash_task]);
        }
        
        // Other kinds of simple tasks could be handled here
        
        // Not a simple task
        None
    }
    
    /// Parse function calls from AI response into Task objects
    fn parse_function_calls(
        &self, 
        response: Value,
        parent_id: String,
        parent_description: &str
    ) -> AgentResult<Vec<Task>> {
        // Create main task
        let mut tasks = Vec::new();
        let parent_task = Task::new(
            parent_id.clone(),
            parent_description.to_string(),
            None,
            Vec::new(),
            self.default_timeout,
            1,
        ).with_operation(OperationType::TASK);
        
        tasks.push(parent_task);
        
        // Extract function calls from response
        if let Some(function_calls) = response.get("function_calls").and_then(|c| c.as_array()) {
            for call in function_calls {
                if let (Some(name), Some(args)) = (
                    call.get("name").and_then(|n| n.as_str()),
                    call.get("arguments").and_then(|a| a.as_object())
                ) {
                    // Map function name to operation type
                    let operation_type = function_name_to_operation_type(name);
                    
                    // Extract common parameters
                    let priority = args.get("priority")
                        .and_then(|p| p.as_u64())
                        .unwrap_or(3) as u8;
                    
                    // Create task based on operation type
                    let (task_id, description) = match operation_type {
                        OperationType::SEARCH => {
                            if let Some(query) = args.get("query").and_then(|q| q.as_str()) {
                                (
                                    utils::generate_id(),
                                    format!("Search for: {}", query)
                                )
                            } else {
                                continue; // Skip if required param missing
                            }
                        },
                        OperationType::READ => {
                            if let Some(file_path) = args.get("file_path").and_then(|f| f.as_str()) {
                                (
                                    utils::generate_id(),
                                    format!("Read and analyze file: {}", file_path)
                                )
                            } else {
                                continue; // Skip if required param missing
                            }
                        },
                        OperationType::UPDATE => {
                            if let (Some(file_path), Some(changes)) = (
                                args.get("file_path").and_then(|f| f.as_str()),
                                args.get("changes").and_then(|c| c.as_str())
                            ) {
                                (
                                    utils::generate_id(),
                                    format!("Update file {}: {}", file_path, changes)
                                )
                            } else {
                                continue; // Skip if required param missing
                            }
                        },
                        OperationType::BASH => {
                            if let Some(command) = args.get("command").and_then(|c| c.as_str()) {
                                (
                                    utils::generate_id(),
                                    format!("Execute command: {}", command)
                                )
                            } else {
                                continue; // Skip if required param missing
                            }
                        },
                        OperationType::TASK => {
                            if let Some(desc) = args.get("description").and_then(|d| d.as_str()) {
                                (
                                    utils::generate_id(),
                                    desc.to_string()
                                )
                            } else {
                                continue; // Skip if required param missing
                            }
                        },
                        _ => continue, // Skip unknown operation types
                    };
                    
                    // Create the task
                    let task = Task::new(
                        task_id,
                        description,
                        Some(parent_id.clone()),
                        Vec::new(), // No dependencies for now - we'll process in order
                        self.default_timeout,
                        priority
                    ).with_operation(operation_type);
                    
                    tasks.push(task);
                }
            }
        }
        
        Ok(tasks)
    }
    
    /// Create Task objects from the AI decomposition
    fn create_tasks(
        &self,
        parent_description: &str, 
        decomposition: &DecompositionResponse,
        parent_id: String,
    ) -> AgentResult<Vec<Task>> {
        // Create the parent task
        let mut tasks = Vec::new();
        let parent_task = Task::new(
            parent_id.clone(),
            parent_description.to_string(),
            None,
            Vec::new(),
            self.default_timeout,
            1,
        ).with_operation(OperationType::TASK); // Mark the parent task as the main TASK operation
        tasks.push(parent_task);
        
        // Create all subtasks
        let mut subtask_ids = Vec::with_capacity(decomposition.subtasks.len());
        for (_i, subtask) in decomposition.subtasks.iter().enumerate() {
            let task_id = utils::generate_id();
            subtask_ids.push(task_id.clone());
            
            // Convert 1-based indices to task IDs
            let dependencies = subtask
                .dependencies
                .iter()
                .filter_map(|&idx| {
                    if idx > 0 && idx <= subtask_ids.len() {
                        Some(subtask_ids[idx - 1].clone())
                    } else {
                        None
                    }
                })
                .collect();
            
            let timeout = Duration::from_secs(subtask.estimated_time_seconds);
            
            // Map the operation type string to SubtaskOperation enum if provided
            let operation_type = subtask.operation_type.clone().unwrap_or_default();
            
            let mut task = Task::new(
                task_id,
                subtask.description.clone(),
                Some(parent_id.clone()),
                dependencies,
                timeout,
                subtask.priority,
            );
            
            // Convert string operation type to enum and set it
            if !operation_type.is_empty() {
                let op_type = match operation_type.as_str() {
                    "TASK" => OperationType::TASK,
                    "READ" => OperationType::READ,
                    "UPDATE" => OperationType::UPDATE,
                    "SEARCH" => OperationType::SEARCH,
                    "BASH" => OperationType::BASH,
                    _ => OperationType::UNKNOWN,
                };
                task = task.with_operation(op_type);
            }
            
            tasks.push(task);
        }
        
        Ok(tasks)
    }
}

/// Parse the JSON response from the AI service
fn parse_decomposition_response(response: &str) -> AgentResult<DecompositionResponse> {
    // Try to extract JSON from the response
    // This handles cases where the AI might return text before/after the JSON
    let json_start = match response.find('{') {
        Some(pos) => pos,
        None => return Err(AgentError::AiApi(
            "Failed to find JSON object in AI response".to_string(),
        )),
    };
    
    let json_end = match response.rfind('}') {
        Some(pos) => pos + 1, // include the closing brace
        None => return Err(AgentError::AiApi(
            "Failed to find end of JSON object in AI response".to_string(),
        )),
    };
    
    if json_start >= json_end {
        return Err(AgentError::AiApi(
            "Invalid JSON structure in AI response".to_string(),
        ));
    }
    
    let json_str = &response[json_start..json_end];
    
    // Log the JSON we're trying to parse in debug mode
    log::debug!("Attempting to parse AI response JSON: {}", json_str);
    
    // Try to parse the JSON
    match serde_json::from_str::<DecompositionResponse>(json_str) {
        Ok(decomposition) => {
            // Validate the decomposition
            if decomposition.subtasks.is_empty() {
                return Err(AgentError::AiApi(
                    "AI response contained no subtasks".to_string(),
                ));
            }
            Ok(decomposition)
        },
        Err(e) => {
            // If parsing failed, log the issue and try to display a helpful error
            log::error!("Failed to parse AI response: {}", e);
            log::debug!("Response JSON: {}", json_str);
            
            // Try to parse as a generic Value to give a more helpful error
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                if let Some(subtasks) = value.get("subtasks") {
                    if !subtasks.is_array() {
                        return Err(AgentError::AiApi(
                            "The 'subtasks' field must be an array".to_string(),
                        ));
                    }
                    
                    // Try a manual conversion as a fallback
                    let mut manual_subtasks = Vec::new();
                    for (i, task) in subtasks.as_array().unwrap().iter().enumerate() {
                        if let Some(task_obj) = task.as_object() {
                            // Extract description
                            let description = match task_obj.get("description") {
                                Some(d) if d.is_string() => d.as_str().unwrap().to_string(),
                                _ => format!("Task {}", i + 1),
                            };
                            
                            // Extract dependencies
                            let mut dependencies = Vec::new();
                            if let Some(deps) = task_obj.get("dependencies") {
                                if let Some(deps_array) = deps.as_array() {
                                    for dep in deps_array {
                                        let idx = match dep {
                                            serde_json::Value::Number(n) => n.as_u64().unwrap_or(0) as usize,
                                            serde_json::Value::String(s) => s.parse::<usize>().unwrap_or(0),
                                            _ => 0,
                                        };
                                        if idx > 0 {
                                            dependencies.push(idx);
                                        }
                                    }
                                }
                            }
                            
                            // Extract estimated time
                            let estimated_time = match task_obj.get("estimated_time_seconds") {
                                Some(t) if t.is_number() => t.as_u64().unwrap_or(60),
                                Some(t) if t.is_string() => t.as_str().unwrap().parse::<u64>().unwrap_or(60),
                                _ => 60,
                            };
                            
                            // Extract priority
                            let priority = match task_obj.get("priority") {
                                Some(p) if p.is_number() => p.as_u64().unwrap_or(3) as u8,
                                Some(p) if p.is_string() => p.as_str().unwrap().parse::<u8>().unwrap_or(3),
                                _ => 3,
                            };
                            
                            // Create a subtask
                            manual_subtasks.push(SubtaskSpec {
                                description,
                                dependencies,
                                estimated_time_seconds: estimated_time,
                                priority,
                                operation_type: None,
                            });
                        }
                    }
                    
                    // If we were able to extract at least one task, use that
                    if !manual_subtasks.is_empty() {
                        log::warn!("Used fallback JSON parsing to extract {} subtasks", manual_subtasks.len());
                        return Ok(DecompositionResponse { subtasks: manual_subtasks });
                    }
                } else {
                    return Err(AgentError::AiApi(
                        "Missing 'subtasks' field in JSON response".to_string(),
                    ));
                }
            }
            
            // If all else fails, return the original error
            Err(AgentError::AiApi(format!("Failed to parse AI response as JSON: {}", e)))
        }
    }
}