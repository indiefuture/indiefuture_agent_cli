use std::collections::HashMap;
use std::sync::Arc;
use serde_json::{json, Value};

use crate::codebase::CodebaseScanner;
use crate::error::{AgentError, AgentResult};
use crate::memory::{MemoryManager, MemoryMetadata};
use crate::task::{OperationType, Task, TaskExecutor};
use crate::tools::{Capability, EventType, Tool, ToolArgs, ToolOutput};

/// Tool for searching codebase content
pub struct CodeSearchTool {
    codebase_scanner: Arc<CodebaseScanner>,
    memory_manager: Arc<MemoryManager>,
}

impl CodeSearchTool {
    pub fn new(codebase_scanner: Arc<CodebaseScanner>, memory_manager: Arc<MemoryManager>) -> Self {
        Self {
            codebase_scanner,
            memory_manager,
        }
    }
}

impl CodeSearchTool {
    // Create a READ task for a file that was found during search
    fn create_read_task_for_file(&self, file_path: &str, parent_task_id: Option<&str>) -> AgentResult<Task> {
        // Create a new task to read this file
        let task_id = crate::utils::generate_id();
        let task = Task::new(
            task_id,
            format!("Read and analyze file: {}", file_path),
            parent_task_id.map(|id| id.to_string()),
            Vec::new(), // No dependencies
            std::time::Duration::from_secs(60),
            1, // High priority
        ).with_operation(OperationType::READ);
        
        // Log that we're creating a task
        log::info!("📖 Created READ task for file: {}", file_path);
        
        Ok(task)
    }
}

impl Tool for CodeSearchTool {
    fn name(&self) -> &str {
        "code_search"
    }
    
    fn description(&self) -> &str {
        "Searches codebase for relevant files and content"
    }
    
    fn execute(&self, args: &ToolArgs) -> AgentResult<ToolOutput> {
        let query = &args.command;
        
        // Search parameters
        let limit = args.parameters.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;
            
        let store_in_memory = args.parameters.get("store_in_memory")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        
        // Run the search
        let relevant_files = tokio::task::block_in_place(|| {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(self.codebase_scanner.find_relevant_files(query))
        })?;
        
        // Store results in memory if requested
        if store_in_memory {
            for file_info in &relevant_files {
                let metadata = MemoryMetadata {
                    source: "code_search_tool".to_string(),
                    file_path: Some(file_info.path.clone()),
                    language: file_info.language.clone(),
                    tags: vec!["code".to_string(), "search_result".to_string()],
                    task_id: None,
                };
                
                tokio::task::block_in_place(|| {
                    tokio::runtime::Runtime::new()
                        .unwrap()
                        .block_on(self.memory_manager.add_memory(&file_info.content, metadata))
                })?;
            }
        }
        
        // In a real implementation, we would update the shared context
        // For now, just log the search
        log::info!("🔍 Searched code for: {} (found {} results)", query, relevant_files.len());
        
        // Check if we need to create READ tasks for each file
        // If task_queue is provided, we'll automatically create tasks
        // Otherwise, we'll only create tasks if explicitly requested
        let auto_create_tasks = args.task_queue.is_some();
        let create_tasks = args.parameters.get("create_read_tasks")
            .and_then(|v| v.as_bool())
            .unwrap_or(auto_create_tasks);
            
        let current_task_id = args.context.get_variable("current_task_id")
            .and_then(|v| v.as_str());
            
        // Create READ tasks for each file if requested
        // Limit the number of tasks to create to avoid overwhelming the system
        let max_tasks = args.parameters.get("max_tasks")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize; // Default to max 5 tasks at once
            
        let mut generated_tasks = Vec::new();
        if create_tasks && !relevant_files.is_empty() {
            // Check if we have a task queue to add tasks to
            if let Some(task_queue) = &args.task_queue {
                // Create READ tasks and add them to the queue (limited to max_tasks)
                for (i, file) in relevant_files.iter().enumerate() {
                    // Limit the number of tasks created
                    if i >= max_tasks {
                        break;
                    }
                    let new_task = self.create_read_task_for_file(&file.path, current_task_id)?;
                    
                    // Add the task to the queue (at the front for high priority)
                    if let Ok(mut queue) = task_queue.lock() {
                        // Add to front of queue for high priority
                        queue.push_front(new_task.clone());
                        
                        // Log that we've added the task
                        log::info!("📖 Added READ task to queue: {}", new_task.description);
                    }
                    
                    // Also store the task info for response
                    generated_tasks.push(serde_json::to_value(&new_task).unwrap_or_default());
                }
            } else {
                // Just create tasks but don't add them anywhere (limited to max_tasks)
                for (i, file) in relevant_files.iter().enumerate() {
                    // Limit the number of tasks created
                    if i >= max_tasks {
                        break;
                    }
                    let new_task = self.create_read_task_for_file(&file.path, current_task_id)?;
                    generated_tasks.push(serde_json::to_value(&new_task).unwrap_or_default());
                    
                    // Log that we've created the task but couldn't add it
                    log::info!("📖 Created READ task (not added to queue): {}", new_task.description);
                }
            }
        }
        
        // Format the results - include partial content for display but not the full file
        let results: Vec<Value> = relevant_files.iter().map(|file| {
            // Truncate content for display to avoid massive JSON responses
            let preview_content = if file.content.len() > 500 {
                format!("{}... [truncated, {} bytes total]", &file.content[..500], file.content.len())
            } else {
                file.content.clone()
            };
            
            json!({
                "path": file.path,
                "language": file.language,
                "relevance": file.relevance,
                "content": preview_content,
                "size": file.content.len()
            })
        }).collect();
        
        Ok(ToolOutput {
            success: true,
            result: json!({
                "files": results,
                "count": results.len(),
                "query": query,
                "created_read_tasks": create_tasks,
                "generated_tasks": generated_tasks
            }),
            message: None,
            artifacts: HashMap::new(),
        })
    }
    
    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::CodeSearch]
    }
}