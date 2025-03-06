use crate::ai::{AiClient, Message, MessageRole};
use crate::codebase::CodebaseScanner;
use crate::error::{AgentError, AgentResult};
use crate::memory::{MemoryManager, MemoryMetadata};
use crate::task::{OperationType, Task, TaskStatus};
use crate::tools::{
    BashExecutionTool, CodeModificationTool, CodeSearchTool, DocumentationTool,
    TaskProcessingTool, SharedContext, Tool, ToolArgs
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde_json::json;
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Callback type for reporting task status changes
pub type TaskStatusCallback = Box<dyn Fn(&Task) + Send + Sync>;

/// Callback type for requesting user confirmation
#[allow(clippy::type_complexity)]
pub type UserConfirmationCallback = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;

/// Executes subtasks in the correct order based on dependencies
#[derive(Clone)]
pub struct TaskExecutor {
    memory_manager: Arc<MemoryManager>,
    ai_client: Arc<dyn AiClient>,
    codebase_scanner: Arc<CodebaseScanner>,
    workspace_root: PathBuf,
    max_concurrent_tasks: usize,
    task_queue: Arc<Mutex<VecDeque<Task>>>,
    task_map: Arc<Mutex<HashMap<String, Task>>>,
    completed_tasks: Arc<Mutex<HashSet<String>>>,
    status_callback: Option<Arc<TaskStatusCallback>>,
    user_confirmation_callback: Option<UserConfirmationCallback>,
    tools: Arc<HashMap<String, Box<dyn Tool>>>,
}

impl TaskExecutor {
    pub fn new(
        memory_manager: Arc<MemoryManager>,
        ai_client: Box<dyn AiClient>,
        codebase_scanner: Arc<CodebaseScanner>,
        workspace_root: PathBuf,
        max_concurrent_tasks: usize,
    ) -> Self {
        // Box to Arc conversion by boxing the client then wrapping in Arc
        let ai_client_arc: Arc<dyn AiClient> = match ai_client.clone_box() {
            boxed => Arc::from(boxed)
        };
        
        // Create tools map
        let mut tools = HashMap::new();
        
        // Bash execution tool
        let bash_tool = BashExecutionTool::new(workspace_root.clone());
        tools.insert(bash_tool.name().to_string(), Box::new(bash_tool) as Box<dyn Tool>);
        
        // Code search tool
        let code_search_tool = CodeSearchTool::new(codebase_scanner.clone(), memory_manager.clone());
        tools.insert(code_search_tool.name().to_string(), Box::new(code_search_tool) as Box<dyn Tool>);
        
        // Code modification tool
        let code_mod_tool = CodeModificationTool::new(workspace_root.clone(), memory_manager.clone(), true);
        tools.insert(code_mod_tool.name().to_string(), Box::new(code_mod_tool) as Box<dyn Tool>);
        
        // Documentation tool
        let docs_path = workspace_root.join("docs");
        let docs_tool = DocumentationTool::new(docs_path);
        tools.insert(docs_tool.name().to_string(), Box::new(docs_tool) as Box<dyn Tool>);
        
        // Task processing tool
        let task_tool = TaskProcessingTool::new(ai_client_arc.clone(), 300); // 5 minute timeout for task processing
        tools.insert(task_tool.name().to_string(), Box::new(task_tool) as Box<dyn Tool>);
        
        Self {
            memory_manager,
            ai_client: ai_client_arc,
            codebase_scanner,
            workspace_root,
            max_concurrent_tasks,
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            task_map: Arc::new(Mutex::new(HashMap::new())),
            completed_tasks: Arc::new(Mutex::new(HashSet::new())),
            status_callback: None,
            user_confirmation_callback: None,
            tools: Arc::new(tools),
        }
    }

    /// Set a callback function to receive task status updates
    pub fn set_status_callback(&mut self, callback: TaskStatusCallback) {
        self.status_callback = Some(Arc::new(callback));
    }
    
    /// Set a callback function for requesting user confirmation
    pub fn set_user_confirmation_callback(&mut self, callback: impl Fn(&str, &str) -> bool + Send + Sync + 'static) {
        self.user_confirmation_callback = Some(Arc::new(callback));
    }

    /// Queue tasks for execution with deduplication
    pub fn queue_tasks(&self, tasks: Vec<Task>) -> AgentResult<()> {
        let mut task_queue = self.task_queue.lock().map_err(|e| {
            AgentError::TaskExecution(format!("Failed to lock task queue: {}", e))
        })?;

        let mut task_map = self.task_map.lock().map_err(|e| {
            AgentError::TaskExecution(format!("Failed to lock task map: {}", e))
        })?;

        // Map to store seen descriptions for deduplication
        let mut seen_descriptions = std::collections::HashMap::new();
        let mut dupes_removed = 0;
        
        // Add tasks to the queue and map (with deduplication)
        for task in tasks {
            // For BASH tasks, deduplicate based on description
            if let Some(op_type) = &task.operation_type {
                if *op_type == OperationType::BASH {
                    // Extract command from description if possible
                    let command = if task.description.contains("Execute command:") {
                        task.description.replace("Execute command:", "").trim().to_string()
                    } else {
                        task.description.clone()
                    };
                    
                    // Skip if we've seen this command before
                    if let Some(existing_id) = seen_descriptions.get(&command) {
                        // Check if the existing task is the parent task
                        if task.parent_id.as_deref() != Some(existing_id) {
                            log::info!("🔍 Skipping duplicate BASH task: {}", command);
                            dupes_removed += 1;
                            continue;
                        }
                    }
                    
                    // Add to seen commands
                    seen_descriptions.insert(command, task.id.clone());
                }
            }
            
            // Add task to queue and map
            task_map.insert(task.id.clone(), task.clone());
            task_queue.push_back(task);
        }

        if dupes_removed > 0 {
            log::info!("🧹 Removed {} duplicate tasks", dupes_removed);
        }

        Ok(())
    }
    
    /// Add a new task to the front of the queue with higher priority
    pub fn add_high_priority_task(&self, task: Task) -> AgentResult<()> {
        let mut task_queue = self.task_queue.lock().map_err(|e| {
            AgentError::TaskExecution(format!("Failed to lock task queue: {}", e))
        })?;

        let mut task_map = self.task_map.lock().map_err(|e| {
            AgentError::TaskExecution(format!("Failed to lock task map: {}", e))
        })?;

        // Check for duplicates before adding (especially for BASH tasks)
        let should_add = if let Some(op_type) = &task.operation_type {
            if *op_type == OperationType::BASH {
                // Extract command
                let command = if task.description.contains("Execute command:") {
                    task.description.replace("Execute command:", "").trim().to_string()
                } else {
                    task.description.clone()
                };
                
                // Check if this command already exists in any queued task
                let duplicate = task_queue.iter().any(|t| {
                    if let Some(t_op) = &t.operation_type {
                        if *t_op == OperationType::BASH {
                            let t_desc = if t.description.contains("Execute command:") {
                                t.description.replace("Execute command:", "").trim().to_string()
                            } else {
                                t.description.clone()
                            };
                            
                            // Compare commands
                            t_desc == command
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                });
                
                if duplicate {
                    log::info!("🔍 Skipping duplicate BASH task: {}", command);
                    false
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };
        
        // Add task to the map and front of the queue if not a duplicate
        if should_add {
            task_map.insert(task.id.clone(), task.clone());
            task_queue.push_front(task);
        }

        Ok(())
    }
    
    /// Add a new task to the back of the queue
    pub fn add_task(&self, task: Task) -> AgentResult<()> {
        let mut task_queue = self.task_queue.lock().map_err(|e| {
            AgentError::TaskExecution(format!("Failed to lock task queue: {}", e))
        })?;

        let mut task_map = self.task_map.lock().map_err(|e| {
            AgentError::TaskExecution(format!("Failed to lock task map: {}", e))
        })?;

        // Check for duplicates before adding (especially for BASH tasks)
        let should_add = if let Some(op_type) = &task.operation_type {
            if *op_type == OperationType::BASH {
                // Extract command
                let command = if task.description.contains("Execute command:") {
                    task.description.replace("Execute command:", "").trim().to_string()
                } else {
                    task.description.clone()
                };
                
                // Check if this command already exists in any queued task
                let duplicate = task_queue.iter().any(|t| {
                    if let Some(t_op) = &t.operation_type {
                        if *t_op == OperationType::BASH {
                            let t_desc = if t.description.contains("Execute command:") {
                                t.description.replace("Execute command:", "").trim().to_string()
                            } else {
                                t.description.clone()
                            };
                            
                            // Compare commands
                            t_desc == command
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                });
                
                if duplicate {
                    log::info!("🔍 Skipping duplicate BASH task: {}", command);
                    false
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };
        
        // Add task to the map and back of the queue if not a duplicate
        if should_add {
            task_map.insert(task.id.clone(), task.clone());
            task_queue.push_back(task);
        }

        Ok(())
    }

    /// Start executing tasks
    pub async fn execute_tasks(&self) -> AgentResult<HashMap<String, Task>> {
        let (tx, mut rx) = mpsc::channel(self.max_concurrent_tasks);

        // Initial number of active tasks
        let mut active_count = 0;
        
        // Flag to trigger checking for new tasks periodically
        let mut needs_task_check = true;
        
        // Count iterations to log status periodically
        let mut iterations = 0;

        // Main execution loop - continues as long as there are active tasks OR new tasks to start
        while active_count > 0 || needs_task_check {
            iterations += 1;
            
            // Reset the flag - we'll set it again if we need another check
            needs_task_check = false;
            
            // Check if there are any tasks in the queue (every 10 iterations)
            if iterations % 10 == 0 {
                let queue_size = {
                    let task_queue = self.task_queue.lock().map_err(|e| {
                        AgentError::TaskExecution(format!("Failed to lock task queue: {}", e))
                    })?;
                    task_queue.len()
                };
                
                if queue_size > 0 {
                    log::info!("Task status: {} in queue, {} active", queue_size, active_count);
                }
            }
            
            // Check for new tasks if we have capacity
            if active_count < self.max_concurrent_tasks {
                // Extract tasks that are ready to execute
                let mut ready_tasks = self.get_ready_tasks()?;
                
                // Prioritize READ operations first, then others
                ready_tasks.sort_by(|a, b| {
                    // First sort by priority (lower number is higher priority)
                    let priority_cmp = a.priority.cmp(&b.priority);
                    if priority_cmp != std::cmp::Ordering::Equal {
                        return priority_cmp;
                    }
                    
                    // Then by operation type (READ first)
                    let a_is_read = a.operation_type == Some(OperationType::READ);
                    let b_is_read = b.operation_type == Some(OperationType::READ);
                    
                    if a_is_read && !b_is_read {
                        return std::cmp::Ordering::Less;
                    } else if !a_is_read && b_is_read {
                        return std::cmp::Ordering::Greater;
                    }
                    
                    // Finally by creation time
                    a.created_at.cmp(&b.created_at)
                });
                
                if !ready_tasks.is_empty() {
                    log::info!("Found {} ready tasks to execute", ready_tasks.len());
                }
                
                // Start new tasks up to the max concurrent limit
                for task in ready_tasks {
                    let task_id = task.id.clone();
                    let description = task.description.clone();
                    
                    // Log that we're starting a new task with operation type
                    let op_type = task.operation_type.clone().unwrap_or(OperationType::UNKNOWN);
                    let icon = op_type.icon();
                    log::info!("{} Starting task: {} ({})", icon, description, task_id);
                    
                    // Now start the task
                    self.start_task(task, tx.clone()).await?;
                    active_count += 1;

                    if active_count >= self.max_concurrent_tasks {
                        break;
                    }
                }
            }
            
            // If we have active tasks, wait for one to complete
            if active_count > 0 {
                // Set a timeout to periodically check for new tasks
                let timeout_duration = std::time::Duration::from_millis(100);
                match tokio::time::timeout(timeout_duration, rx.recv()).await {
                    Ok(Some((task_id, result))) => {
                        active_count -= 1;
                        self.handle_completed_task(&task_id, result).await?;
                        
                        // Always check for new tasks after a task completes
                        needs_task_check = true;
                    }
                    Ok(None) => {
                        // Channel closed, no more tasks will be coming
                        break;
                    }
                    Err(_) => {
                        // Timeout occurred, check for new tasks
                        needs_task_check = true;
                    }
                }
            }
        }

        // Return all tasks with their final states
        let task_map = self.task_map.lock().map_err(|e| {
            AgentError::TaskExecution(format!("Failed to lock task map: {}", e))
        })?;

        Ok(task_map.clone())
    }

    /// Get all tasks that are ready to execute (no pending dependencies)
    fn get_ready_tasks(&self) -> AgentResult<Vec<Task>> {
        let task_queue = self.task_queue.lock().map_err(|e| {
            AgentError::TaskExecution(format!("Failed to lock task queue: {}", e))
        })?;

        let completed_tasks = self.completed_tasks.lock().map_err(|e| {
            AgentError::TaskExecution(format!("Failed to lock completed tasks: {}", e))
        })?;

        let completed_vec: Vec<String> = completed_tasks.iter().cloned().collect();

        // Get all tasks that are ready to execute
        let ready_tasks: Vec<Task> = task_queue
            .iter()
            .filter(|task| task.is_executable(&completed_vec))
            .cloned()
            .collect();
            
        // Log queue state if there are pending tasks
        if !task_queue.is_empty() {
            log::debug!("Task queue status: {} total tasks, {} ready to execute",
                task_queue.len(), ready_tasks.len());
                
            // Log operation types of ready tasks
            if !ready_tasks.is_empty() {
                for task in &ready_tasks {
                    let op_type = task.operation_type.clone().unwrap_or(OperationType::UNKNOWN);
                    let icon = op_type.icon();
                    log::info!("{} Ready to execute: {} ({})", icon, task.description, task.id);
                }
            }
        }

        Ok(ready_tasks)
    }

    /// Get the next task that's ready to execute
    fn get_next_ready_task(&self) -> AgentResult<Option<Task>> {
        let mut task_queue = self.task_queue.lock().map_err(|e| {
            AgentError::TaskExecution(format!("Failed to lock task queue: {}", e))
        })?;

        let completed_tasks = self.completed_tasks.lock().map_err(|e| {
            AgentError::TaskExecution(format!("Failed to lock completed tasks: {}", e))
        })?;

        let completed_vec: Vec<String> = completed_tasks.iter().cloned().collect();

        // Find the first task that's ready to execute
        let position = task_queue
            .iter()
            .position(|task| task.is_executable(&completed_vec));

        // Remove and return the task if found
        if let Some(pos) = position {
            Ok(Some(task_queue.remove(pos).unwrap()))
        } else {
            Ok(None)
        }
    }

    /// Start a task execution in a new tokio task
    async fn start_task(
        &self,
        mut task: Task,
        tx: mpsc::Sender<(String, AgentResult<String>)>,
    ) -> AgentResult<()> {
        // Mark task as in progress
        task.mark_in_progress();
        self.update_task(&task)?;

        // Notify of status change
        self.notify_status_change(&task);

        // Clone necessary resources for the task
        let task_id = task.id.clone();
        let description = task.description.clone();
        let timeout_duration = task.timeout;
        let memory_manager = self.memory_manager.clone();
        let ai_client = self.ai_client.clone();
        let codebase_scanner = self.codebase_scanner.clone();
        let tools = self.tools.clone();
        let workspace_root = self.workspace_root.clone();
        let user_confirmation_callback = self.user_confirmation_callback.clone();
        
        // Make clones of the task queue and map to pass to the task
        let task_queue_clone = self.task_queue.clone();
        let task_map_clone = self.task_map.clone();

        // Create a working context for this task if it doesn't exist yet
        let parent_id = task.parent_id.clone();
        if memory_manager.get_working_context(&task_id).is_none() {
            memory_manager.create_working_context(
                &task_id, 
                parent_id.as_deref()
            )?;
        }

        // Spawn a new tokio task
        tokio::spawn(async move {
            let result = timeout(timeout_duration, async {
                Self::execute_single_task(
                    &task_id,
                    &description,
                    memory_manager,
                    ai_client,
                    codebase_scanner,
                    tools,
                    workspace_root,
                    user_confirmation_callback,
                    task_queue_clone,
                    task_map_clone,
                )
                .await
            })
            .await;

            // Send result back to the coordinator
            let task_result = match result {
                Ok(inner_result) => inner_result,
                Err(_) => Err(AgentError::TaskExecution(format!(
                    "Task timed out after {:?}",
                    timeout_duration
                ))),
            };

            let _ = tx.send((task_id, task_result)).await;
        });

        Ok(())
    }

    /// Handle a completed task
    async fn handle_completed_task(
        &self,
        task_id: &str,
        result: AgentResult<String>,
    ) -> AgentResult<()> {
        let mut task_map = self.task_map.lock().map_err(|e| {
            AgentError::TaskExecution(format!("Failed to lock task map: {}", e))
        })?;

        let mut completed_tasks = self.completed_tasks.lock().map_err(|e| {
            AgentError::TaskExecution(format!("Failed to lock completed tasks: {}", e))
        })?;

        // Update the task with the result
        if let Some(task) = task_map.get_mut(task_id) {
            match result {
                Ok(output) => {
                    // Get the task operation type and description for logging
                    let op_type = task.operation_type.clone().unwrap_or(OperationType::UNKNOWN);
                    let icon = op_type.icon();
                    
                    // Clone the output for validation and logging
                    let output_clone = output.clone();
                    
                    // Check if the response fits the operation type
                    let is_response_valid = match op_type {
                        OperationType::READ => true, // READ operations always return content which is valid
                        OperationType::SEARCH => output_clone.contains("file") || output_clone.contains("path") || output_clone.contains("found"),
                        OperationType::UPDATE => output_clone.contains("update") || output_clone.contains("modified") || output_clone.contains("changed"),
                        OperationType::BASH => true, // BASH outputs vary widely
                        OperationType::TASK => true, // TASK responses are general and flexible
                        OperationType::UNKNOWN => true, // Default to accepting
                    };
                    
                    if is_response_valid {
                        // Clone task description for logging
                        let task_description = task.description.clone();
                        
                        // Mark as completed and add to completed tasks
                        task.mark_completed(Some(output));
                        completed_tasks.insert(task_id.to_string());
                        
                        // Log completion with task type icon
                        if op_type == OperationType::READ {
                            // For READ tasks, show a snippet of the file content
                            let preview = if output_clone.len() > 100 {
                                format!("{}...", &output_clone[0..100])
                            } else {
                                output_clone
                            };
                            log::info!("{} Completed READ task: {} - Content: {}", icon, task_description, preview);
                        } else {
                            log::info!("{} Completed {} task: {}", icon, op_type, task_description);
                        }
                    } else {
                        // Response doesn't fit the expected operation type
                        log::warn!("⚠️ Response for {} operation doesn't match expected format", op_type);
                        log::warn!("⚠️ Discarding response for task: {}", task.description);
                        
                        // Mark as failed instead of completed
                        task.mark_failed(format!("Response doesn't match {} operation type", op_type));
                    }
                }
                Err(e) => {
                    task.mark_failed(format!("{}", e));
                    log::error!("❌ Failed task: {} - {}", task.description, e);
                }
            }

            // Notify of status change
            self.notify_status_change(task);
        }

        Ok(())
    }

    /// Execute a single task using the enhanced execution engine with tools
    async fn execute_single_task(
        task_id: &str,
        description: &str,
        memory_manager: Arc<MemoryManager>,
        ai_client: Arc<dyn AiClient>,
        codebase_scanner: Arc<CodebaseScanner>,
        tools: Arc<HashMap<String, Box<dyn Tool>>>,
        workspace_root: PathBuf,
        user_confirmation_callback: Option<UserConfirmationCallback>,
        task_queue: Arc<Mutex<VecDeque<Task>>>,
        task_map: Arc<Mutex<HashMap<String, Task>>>,
    ) -> AgentResult<String> {
        // Check operation type
        if let Ok(task_map_lock) = task_map.lock() {
            if let Some(task) = task_map_lock.get(task_id) {
                if let Some(op_type) = &task.operation_type {
                    // Log operation beginning with proper icon
                    log::info!("{} Executing {} task: {}", op_type.icon(), op_type, description);
                    
                    // For SEARCH and READ operations, ensure we're not using the AI API unnecessarily
                    if *op_type == OperationType::SEARCH || *op_type == OperationType::READ {
                        log::info!("This is a filesystem operation - will use local scanning only");
                    }
                }
            }
        }
        // Create a shared context for the tools
        let mut context = crate::tools::SharedContext::new();
        
        // Store the task_id in the context
        // This allows tools to create related subtasks
        context.add_variable("current_task_id", serde_json::Value::String(task_id.to_string()));
        
        let shared_context = Arc::new(context);
        
        // Get the operation type if available, for specialized handling
        let op_type = {
            if let Ok(task_map_lock) = task_map.lock() {
                task_map_lock.get(task_id)
                    .and_then(|t| t.operation_type.clone())
                    .unwrap_or(OperationType::UNKNOWN)
            } else {
                OperationType::UNKNOWN
            }
        };
        
        // 1. Check if we need to scan code for this task
        let needs_code_context = op_type == OperationType::READ || 
                               op_type == OperationType::SEARCH || 
                               description.to_lowercase().contains("code") ||
                               description.to_lowercase().contains("file") ||
                               description.to_lowercase().contains("directory");

        // 2. Gather context if needed
        let mut context = String::new();
        if needs_code_context {
            // Use the code search tool to find relevant files
            if let Some(code_search_tool) = tools.get("code_search") {
                let mut tool_args = crate::tools::ToolArgs {
                    command: description.to_string(),
                    parameters: HashMap::new(),
                    context: shared_context.clone(),
                    task_queue: Some(task_queue.clone()),
                };
                
                let search_result = code_search_tool.execute(&tool_args)?;
                
                if search_result.success {
                    if let Some(files) = search_result.result.get("files") {
                        if let Some(files_array) = files.as_array() {
                            for file in files_array {
                                if let (Some(path), Some(content)) = (file.get("path"), file.get("content")) {
                                    let file_path = path.as_str().unwrap_or("unknown");
                                    let file_content = content.as_str().unwrap_or("");
                                    
                                    // Log that we're reading a file
                                    if let Ok(task_map_lock) = task_map.lock() {
                                        if let Some(task) = task_map_lock.get(task_id) {
                                            if let Some(op_type) = &task.operation_type {
                                                if *op_type == OperationType::READ {
                                                    // Log first 100 chars of file content
                                                    let preview = if file_content.len() > 100 {
                                                        format!("{}...", &file_content[0..100])
                                                    } else {
                                                        file_content.to_string()
                                                    };
                                                    log::info!("📖 Reading file: {} - {}", file_path, preview);
                                                }
                                            }
                                        }
                                    }
                                    
                                    context.push_str(&format!(
                                        "File: {}\nContent:\n{}\n\n",
                                        file_path,
                                        file_content,
                                    ));
                                }
                            }
                        }
                    }
                }
            } else {
                // Fall back to codebase scanner if tool not available
                let relevant_files = codebase_scanner.find_relevant_files(description).await?;
                for file_info in relevant_files {
                    let file_path = &file_info.path;
                    let file_content = &file_info.content;
                    
                    // Log that we're reading a file
                    if let Ok(task_map_lock) = task_map.lock() {
                        if let Some(task) = task_map_lock.get(task_id) {
                            if let Some(op_type) = &task.operation_type {
                                if *op_type == OperationType::READ {
                                    // Log first 100 chars of file content
                                    let preview = if file_content.len() > 100 {
                                        format!("{}...", &file_content[0..100])
                                    } else {
                                        file_content.to_string()
                                    };
                                    log::info!("📖 Reading file: {} - {}", file_path, preview);
                                }
                            }
                        }
                    }
                    
                    context.push_str(&format!(
                        "File: {}\nContent:\n{}\n\n",
                        file_path, file_content
                    ));
                }
            }
        }

        // Special handling for direct operations that don't need AI
        match op_type {
            // READ tasks - just read file content and return
            OperationType::READ => {
                log::info!("📖 Processing READ operation directly without AI");
                
                // Extract the file path from the description if possible
                let file_path = if description.contains("Read and analyze file:") {
                    description.replace("Read and analyze file:", "").trim().to_string()
                } else {
                    // Try to parse from context
                    match context.lines().find(|line| line.starts_with("File:")) {
                        Some(line) => line.replace("File:", "").trim().to_string(),
                        None => description.to_string()
                    }
                };
                
                log::info!("📖 Reading file: {}", file_path);
                
                // The result is the content we already gathered in the context
                if !context.is_empty() {
                    return Ok(context);
                } else {
                    return Ok(format!("Read operation completed, but no content found for: {}", file_path));
                }
            },
            
            // SEARCH tasks - just return summary of files found
            OperationType::SEARCH => {
                log::info!("🔍 Processing SEARCH operation directly without AI");
                
                // If we have context, summarize the files found
                if !context.is_empty() {
                    let file_count = context.matches("File:").count();
                    return Ok(format!("Search operation found {} relevant files. See context for details.", file_count));
                } else {
                    return Ok(format!("Search operation completed, but no files found matching: {}", description));
                }
            },
            
            // BASH tasks - execute locally without AI
            OperationType::BASH => {
                log::info!("💻 Processing BASH operation directly without AI");
                
                // Extract the command from the description
                let command = if description.contains("Execute command:") {
                    description.replace("Execute command:", "").trim().to_string()
                } else {
                    description.to_string()
                };
                
                // Use the bash tool to execute the command
                if let Some(bash_tool) = tools.get("bash") {
                    let mut tool_args = ToolArgs {
                        command: command.clone(),
                        parameters: HashMap::new(),
                        context: shared_context.clone(),
                        task_queue: Some(task_queue.clone()),
                    };
                    
                    log::info!("💻 Executing command: {}", command);
                    let result = bash_tool.execute(&tool_args)?;
                    
                    if result.success {
                        // Format the result
                        let output = serde_json::to_string_pretty(&result.result).unwrap_or_default();
                        return Ok(format!("Command executed: {}\nOutput:\n{}", command, output));
                    } else {
                        return Ok(format!("Command execution failed: {}\nError: {}", command, 
                            result.message.unwrap_or_else(|| "Unknown error".to_string())));
                    }
                } else {
                    return Ok(format!("BASH tool not available to execute: {}", command));
                }
            },
            
            // UPDATE operations - use specialized AI prompt with file editing functions
            OperationType::UPDATE => {
                log::info!("✏️ Processing UPDATE operation with specialized AI prompt");
                
                // Extract the file path from the description
                let file_path = if description.contains("Update file") {
                    let parts: Vec<&str> = description.split(":").collect();
                    if parts.len() >= 2 {
                        parts[0].replace("Update file", "").trim().to_string()
                    } else {
                        description.to_string()
                    }
                } else {
                    // Try to parse from context
                    match context.lines().find(|line| line.starts_with("File:")) {
                        Some(line) => line.replace("File:", "").trim().to_string(),
                        None => description.to_string()
                    }
                };
                
                // Get the update goal/description
                let update_goal = if description.contains(":") {
                    let parts: Vec<&str> = description.splitn(2, ":").collect();
                    if parts.len() >= 2 {
                        parts[1].trim().to_string()
                    } else {
                        "Update the file as needed".to_string()
                    }
                } else {
                    description.to_string()
                };
                
                log::info!("✏️ Updating file: {} - Goal: {}", file_path, update_goal);
                
                // Continue with the AI processing but use a specialized prompt
                // This is handled in the regular flow below
            },
                
            // TASK and other operations will use the default AI processing
            _ => {}
        }

        // For other operations, continue with the normal AI-based processing
        
        // 3. Get relevant memories
        let memories = memory_manager.retrieve_relevant(description, 5).await?;
        for memory in memories {
            context.push_str(&format!(
                "Memory: {}\nContent:\n{}\n\n",
                memory.metadata.source, memory.content
            ));
        }
        
        // 4. Get the task's working context if it exists
        if let Some(working_context) = memory_manager.get_working_context(task_id) {
            context.push_str("Working Context Variables:\n");
            for (key, value) in &working_context.variables {
                context.push_str(&format!("{}: {}\n", key, value));
            }
            context.push_str("\n");
        }

        // 5. Create a system prompt with available tools
        let mut tool_descriptions = String::new();
        for tool in tools.values() {
            tool_descriptions.push_str(&format!("- {}: {}\n", tool.name(), tool.description()));
        }
        
        let system_prompt = if op_type == OperationType::UPDATE {
            // Special system prompt for UPDATE operations with file edit functions
            format!(
                "You are an AI assistant that helps with updating source code files.\n\n\
                You're working on updating a file with specific changes.\n\n\
                TASK INFORMATION:\n\
                - File to update: {}\n\
                - Update goal: {}\n\n\
                To make changes to the file, use one of the following edit functions:\n\n\
                1. delete_lines - Delete lines from a file (specify start_line and end_line)\n\
                2. add_lines - Add lines to a file at a specific position (specify line_number and content)\n\
                3. edit_lines - Edit specific lines in a file (specify start_line, end_line, and new_content)\n\
                4. replace_text - Replace specific text in a file (specify old_text and new_text)\n\n\
                IMPORTANT INSTRUCTIONS:\n\
                1. When using edit_lines or replace_text, make sure to include sufficient context.\n\
                2. When deleting or editing, be precise with line numbers.\n\
                3. Line numbers start at 1 (not 0).\n\
                4. When replacing text, make sure old_text is unique and exists in the file.\n\
                5. Explain your changes and reasoning.\n\n\
                You should respond in JSON format with your edits as an array of operations. Example:\n\n\
                ```json\n\
                {{\n\
                  \"operations\": [\n\
                    {{\n\
                      \"type\": \"edit_lines\",\n\
                      \"start_line\": 10,\n\
                      \"end_line\": 12,\n\
                      \"new_content\": \"// New content here\\nfunction updatedCode() {{\\n  return true;\\n}}\"\n\
                    }},\n\
                    {{\n\
                      \"type\": \"add_lines\",\n\
                      \"line_number\": 15,\n\
                      \"content\": \"// Adding this new function\\nfunction newFeature() {{}}\"\n\
                    }}\n\
                  ],\n\
                  \"explanation\": \"Updated the existing function to fix the bug and added a new function for the new feature.\"\n\
                }}\n\
                ```\n\n\
                Review the file content carefully and make only the necessary changes to accomplish the goal.",
                description.split(":").next().unwrap_or(description),
                description.split(":").nth(1).unwrap_or("")
            )
        } else {
            // Standard system prompt for other operations
            format!(
                "You are an AI assistant that helps with software engineering tasks.\n\n\
                You have the following tools available:\n\
                {}\n\n\
                To use a tool, you should generate a response in the following format:\n\n\
                ```tool-call\n\
                {{\"tool\": \"<tool_name>\", \"command\": \"<command>\", \"parameters\": {{\"param1\": \"value1\", ...}}}}\n\
                ```\n\n\
                IMPORTANT INSTRUCTIONS:\n\
                1. You have been given a list of subtasks to complete. Process each subtask in sequence.\n\
                2. For each subtask, indicate when you've completed it by saying \"Subtask completed\".\n\
                3. Then, move on to the next subtask without waiting for further instructions.\n\
                4. Complete ALL subtasks before providing a final summary.\n\
                5. Ask for confirmation before making any changes to code or files.\n\
                6. After completing all subtasks, provide a summary of what was accomplished.\n\n\
                Complete the given task with the provided context and tools.",
                tool_descriptions
            )
        };

        // 6. Create a user prompt with the task and context
        let user_prompt = if !context.is_empty() {
            format!(
                "Task: {}\n\nRelevant Context:\n{}\n\nThis task has been broken down into subtasks. Please complete ALL subtasks in sequence, indicating when each is complete. After each subtask, immediately proceed to the next one without waiting for instructions. After all subtasks are complete, provide a final summary.",
                description, context
            )
        } else {
            format!(
                "Task: {}\n\nThis task has been broken down into subtasks. Please complete ALL subtasks in sequence, indicating when each is complete. After each subtask, immediately proceed to the next one without waiting for instructions. After all subtasks are complete, provide a final summary.",
                description
            )
        };

        // 7. Start conversation with the AI
        let mut messages = vec![
            Message {
                role: MessageRole::System,
                content: system_prompt,
            },
            Message {
                role: MessageRole::User,
                content: user_prompt,
            },
        ];
        
        // 8. Execute iteratively with the AI, processing one operation at a time
        let mut final_response = String::new();
        let max_iterations = 20; // Increased to allow for more interactions
        let mut user_asked_question = false;
        let mut subtask_completed = false;
        // Tracking to ensure we only process one operation per iteration
        let mut processing_operation = false;
        
        for iteration in 0..max_iterations {
            // Call the AI service
            let response = ai_client.generate_text(messages.clone()).await?;
            
            // Check if response indicates task completion
            if response.to_lowercase().contains("task completed") || 
               response.to_lowercase().contains("subtask completed") ||
               response.to_lowercase().contains("successfully completed") {
                // Mark as a completed subtask
                subtask_completed = true;
                
                // Store this as a potential final response
                if final_response.is_empty() {
                    final_response = response.clone();
                }
                
                // Add a message to continue to the next subtask
                messages.push(Message {
                    role: MessageRole::User,
                    content: "Great work! Please proceed with the next subtask.".to_string(),
                });
                
                // Reset the flags for the next subtask
                user_asked_question = false;
                processing_operation = false;
                
                // Continue to the next iteration
                continue;
            }
            
            // Check if the response is asking a yes/no question - process one question at a time
            else if !user_asked_question && !processing_operation && (
                response.contains("Shall I proceed") || 
                response.contains("shall I proceed") ||
                response.contains("Do you want me to") ||
                response.contains("do you want me to") ||
                response.contains("Should I") ||
                response.contains("should I") ||
                response.contains("Would you like me to") ||
                response.contains("would you like me to")
            ) {
                // This is a question that needs user confirmation
                user_asked_question = true;
                processing_operation = true;
                
                // Store this response even if it's a question - we'll return to it if needed
                if final_response.is_empty() || subtask_completed {
                    final_response = response.clone();
                }
                
                // Get user confirmation for this question - one at a time sequentially
                let should_proceed = match &user_confirmation_callback {
                    Some(callback) => callback("question", &response),
                    None => true, // Default to proceed if no callback
                };
                
                if should_proceed {
                    // Add a message to continue
                    messages.push(Message {
                        role: MessageRole::User,
                        content: "I've reviewed your question. Please proceed with the next step.".to_string(),
                    });
                    
                    // Continue to the next iteration with the updated messages
                    continue;
                } else {
                    // User declined to continue, but don't immediately return.
                    // Instead, add a message to tell the AI to skip to the next subtask
                    messages.push(Message {
                        role: MessageRole::User,
                        content: "Skip this step and move on to the next subtask.".to_string(),
                    });
                    continue;
                }
            }
            
            // Check for tool calls in the response - process ONE at a time
            else if !processing_operation {
                if let Some(tool_call_start) = response.find("```tool-call") {
                    if tool_call_start + 12 < response.len() {
                        let tool_call_remaining = &response[tool_call_start + 12..];
                        if let Some(tool_call_relative_end) = tool_call_remaining.find("```") {
                            // Extract the tool call JSON - just the first one
                            let tool_call_json = &tool_call_remaining[..tool_call_relative_end];
                            
                            // Try to parse the tool call
                            match serde_json::from_str::<serde_json::Value>(tool_call_json) {
                                Ok(tool_call) => {
                                    let tool_name = tool_call["tool"].as_str().unwrap_or("");
                                    let command = tool_call["command"].as_str().unwrap_or("");
                                    
                                    // Get confirmation if needed - one at a time sequentially
                                    processing_operation = true;
                                    let should_proceed = match &user_confirmation_callback {
                                        Some(callback) => callback(tool_name, command),
                                        None => true, // Default to proceed if no callback is set
                                    };
                                    
                                    if should_proceed {
                                        // Execute the tool
                                        if let Some(tool) = tools.get(tool_name) {
                                            let parameters = match tool_call.get("parameters") {
                                                Some(params) if params.is_object() => {
                                                    let map: HashMap<String, serde_json::Value> = params.as_object().unwrap()
                                                        .iter()
                                                        .map(|(k, v)| (k.clone(), v.clone()))
                                                        .collect();
                                                    map
                                                },
                                                _ => HashMap::new(),
                                            };
                                            
                                            let tool_args = ToolArgs {
                                                command: command.to_string(),
                                                parameters,
                                                context: shared_context.clone(),
                                                task_queue: Some(task_queue.clone()),
                                            };
                                        
                                            // Execute the tool and get the result
                                            match tool.execute(&tool_args) {
                                                Ok(result) => {
                                                    // Check if this is a search tool that found files 
                                                    if tool_name == "code_search" {
                                                        // Log details about any files found and tasks created
                                                        if let Some(count) = result.result.get("count") {
                                                            if let Some(count_num) = count.as_u64() {
                                                                if count_num > 0 {
                                                                    // Check if we created tasks from the search results
                                                                    if let Some(tasks) = result.result.get("generated_tasks") {
                                                                        if let Some(tasks_array) = tasks.as_array() {
                                                                            if !tasks_array.is_empty() {
                                                                                log::info!("🔍 Search found {} relevant files and created {} READ tasks", 
                                                                                    count_num, tasks_array.len());
                                                                                    
                                                                                // Log a notice that READ tasks have been created
                                                                                // We'll need to implement a better mechanism to force task execution
                                                                            } else {
                                                                                log::info!("🔍 Search found {} relevant files", count_num);
                                                                            }
                                                                        }
                                                                    } else {
                                                                        log::info!("🔍 Search found {} relevant files", count_num);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    
                                                    // Add the tool call and result to the conversation
                                                    messages.push(Message {
                                                        role: MessageRole::User,
                                                        content: format!("I executed the tool '{}' with command '{}'. Here is the result:\n\n```json\n{}\n```",
                                                            tool_name, command, serde_json::to_string_pretty(&result.result).unwrap_or_default()
                                                        ),
                                                    });
                                                    
                                                    // Update the working context with the result - one operation at a time
                                                    if let Some(mut working_context) = memory_manager.get_working_context(task_id) {
                                                        // Just store a simple status for now
                                                        working_context.set_variable(
                                                            &format!("tool_result_{}", iteration),
                                                            json!({
                                                                "success": result.success,
                                                                "message": result.message
                                                            })
                                                        );
                                                        
                                                        memory_manager.update_working_context(working_context)?;
                                                    }
                                                },
                                                Err(e) => {
                                                    // Add the error to the conversation
                                                    messages.push(Message {
                                                        role: MessageRole::User,
                                                        content: format!("There was an error executing the tool '{}' with command '{}': {}", 
                                                            tool_name, command, e
                                                        ),
                                                    });
                                                }
                                        }
                                        } else {
                                            // Tool not found
                                            messages.push(Message {
                                                role: MessageRole::User,
                                                content: format!("The tool '{}' was not found. Please use one of the available tools.", tool_name),
                                            });
                                        }
                                    } else {
                                        // User denied the operation
                                        messages.push(Message {
                                            role: MessageRole::User,
                                            content: format!("The operation with tool '{}' and command '{}' was not approved. Please suggest an alternative approach.", 
                                                tool_name, command
                                            ),
                                        });
                                    }
                                },
                                Err(e) => {
                                    // Error parsing tool call
                                    messages.push(Message {
                                        role: MessageRole::User,
                                        content: format!("There was an error parsing your tool call: {}. Please use the correct format.", e),
                                    });
                                }
                            }
                        } else {
                            // Malformed tool call - couldn't find closing code block
                            messages.push(Message {
                                role: MessageRole::User,
                                content: "Your tool call was malformed. Couldn't find the closing code block. Please use the correct format.".to_string(),
                            });
                        }
                    } else {
                        // Tool call marker found but not enough characters after it
                        messages.push(Message {
                            role: MessageRole::User,
                            content: "Your tool call was malformed. The tool-call block appears to be cut off. Please use the correct format.".to_string(),
                        });
                    }
                }
            } else {
                // No tool call, check if it looks like a final summary
                if (response.to_lowercase().contains("all subtasks") && response.to_lowercase().contains("complete")) || 
                   (response.to_lowercase().contains("summary") && response.to_lowercase().contains("task")) ||
                   (response.to_lowercase().contains("final result")) {
                    // This looks like a final summary, save it and finish
                    final_response = response.clone();
                    messages.push(Message {
                        role: MessageRole::Assistant,
                        content: response,
                    });
                    break;
                } else {
                    // This is a partial response, save it but keep going
                    if final_response.is_empty() || subtask_completed {
                        final_response = response.clone();
                    }
                    
                    // Add the response to the message history
                    messages.push(Message {
                        role: MessageRole::Assistant,
                        content: response,
                    });
                    
                    // Ask AI to continue with next subtask
                    messages.push(Message {
                        role: MessageRole::User,
                        content: "Please continue with the next subtask.".to_string(),
                    });
                    
                    // Reset flags for next subtask
                    user_asked_question = false;
                    subtask_completed = false;
                    processing_operation = false;
                }
            }
        }
        
        // 9. Store the final result in memory
        let metadata = MemoryMetadata {
            source: "task_execution".to_string(),
            file_path: None,
            language: None,
            tags: vec!["task_result".to_string()],
            task_id: Some(task_id.to_string()),
        };

        memory_manager.add_memory(&final_response, metadata).await?;
        
        // 10. Store final shared context
        if let Some(mut working_context) = memory_manager.get_working_context(task_id) {
            working_context.set_variable("final_result", json!(final_response));
            memory_manager.update_working_context(working_context)?;
        }

        Ok(final_response)
    }

    /// Update a task in the task map
    fn update_task(&self, task: &Task) -> AgentResult<()> {
        let mut task_map = self.task_map.lock().map_err(|e| {
            AgentError::TaskExecution(format!("Failed to lock task map: {}", e))
        })?;

        task_map.insert(task.id.clone(), task.clone());
        Ok(())
    }

    /// Notify the callback of a task status change
    fn notify_status_change(&self, task: &Task) {
        if let Some(callback) = &self.status_callback {
            callback(task);
        }
    }
    
    /// Returns a reference to self wrapped in Arc
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}