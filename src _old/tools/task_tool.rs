use std::collections::HashMap;
use std::sync::Arc;
use serde_json::json;

use crate::ai::AiClient;
use crate::error::{AgentError, AgentResult};
use crate::task::{Task, TaskDecomposer, TaskExecutor};
use crate::tools::{Capability, Tool, ToolArgs, ToolOutput};
use crate::utils;

/// Tool for processing user queries, generating tasks, and managing task flow
pub struct TaskProcessingTool {
    ai_client: Arc<dyn AiClient>,
    default_timeout_seconds: u64,
}

impl TaskProcessingTool {
    pub fn new(ai_client: Arc<dyn AiClient>, default_timeout_seconds: u64) -> Self {
        Self {
            ai_client,
            default_timeout_seconds,
        }
    }
    
    /// Process a user query and break it down into subtasks
    async fn process_query(&self, query: &str) -> AgentResult<Vec<Task>> {
        // Create a task decomposer with our AI client
        let decomposer = TaskDecomposer::new(
            self.ai_client.clone_box(),
            self.default_timeout_seconds,
        );
        
        // Decompose the query into subtasks
        decomposer.decompose(query).await
    }
}

impl Tool for TaskProcessingTool {
    fn name(&self) -> &str {
        "task_processor"
    }
    
    fn description(&self) -> &str {
        "Processes user queries and creates subtasks for execution"
    }
    
    fn execute(&self, args: &ToolArgs) -> AgentResult<ToolOutput> {
        let query = &args.command;
        
        // Process the query
        let tasks = tokio::task::block_in_place(|| {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(self.process_query(query))
        })?;
        
        // Get the current task ID for parent reference
        let current_task_id = args.context.get_variable("current_task_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        // Add the tasks to the task queue if provided
        let mut added_tasks = Vec::new();
        if let Some(task_queue) = &args.task_queue {
            // Track which tasks were actually added (not duplicates)
            for task in &tasks {
                if let Ok(mut queue) = task_queue.lock() {
                    // Check for duplicates in the task queue
                    let is_duplicate = queue.iter().any(|queued_task| {
                        // Simple description similarity check
                        let desc_similarity = self.description_similarity(&task.description, &queued_task.description);
                        
                        // Check if operations match and descriptions are similar enough
                        task.operation_type == queued_task.operation_type && desc_similarity > 0.7
                    });
                    
                    if !is_duplicate {
                        // Add the task to the queue
                        added_tasks.push(task.clone());
                        queue.push_back(task.clone());
                        log::info!("Added new task: {} ({})", 
                            task.description, 
                            task.operation_type.clone().unwrap_or_default());
                    } else {
                        log::info!("🔍 TaskTool skipped duplicate task: {} ({})", 
                            task.description.chars().take(50).collect::<String>(), 
                            task.operation_type.clone().unwrap_or_default());
                    }
                }
            }
        } else {
            // If no queue provided, just return all tasks
            added_tasks = tasks;
        }
            
        // Generate a list of task descriptors for the response
        let task_descriptors: Vec<serde_json::Value> = added_tasks.iter()
            .map(|task| {
                json!({
                    "id": task.id,
                    "description": task.description,
                    "operation_type": task.operation_type,
                    "priority": task.priority
                })
            })
            .collect();
        
        // Return the result
        Ok(ToolOutput {
            success: true,
            result: json!({
                "query": query,
                "tasks": task_descriptors,
                "count": added_tasks.len(),
                "task_ids": added_tasks.iter().map(|t| t.id.clone()).collect::<Vec<String>>()
            }),
            message: None,
            artifacts: HashMap::new(),
        })
    }
    
    /// Calculate similarity between two task descriptions
    /// Returns a value between 0.0 and 1.0, where 1.0 is an exact match
    fn description_similarity(&self, desc1: &str, desc2: &str) -> f64 {
        // Convert to lowercase
        let desc1 = desc1.to_lowercase();
        let desc2 = desc2.to_lowercase();
        
        // Split into words
        let words1: Vec<&str> = desc1.split_whitespace().collect();
        let words2: Vec<&str> = desc2.split_whitespace().collect();
        
        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }
        
        // Count matching words
        let mut matches = 0;
        for word in &words1 {
            if words2.contains(word) {
                matches += 1;
            }
        }
        
        // Calculate Jaccard similarity
        let total_words = words1.len() + words2.len() - matches;
        if total_words == 0 {
            return 1.0;
        }
        
        matches as f64 / total_words as f64
    }
    }
    
    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::TaskProcessing]
    }
}