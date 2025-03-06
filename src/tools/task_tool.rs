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
        let _current_task_id = args.context.get_variable("current_task_id")
            .and_then(|v| v.as_str());
            
        // Generate a list of task descriptors for the response
        let task_descriptors: Vec<serde_json::Value> = tasks.iter()
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
                "count": tasks.len(),
                "task_ids": tasks.iter().map(|t| t.id.clone()).collect::<Vec<String>>()
            }),
            message: None,
            artifacts: HashMap::new(),
        })
    }
    
    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::TaskProcessing]
    }
}