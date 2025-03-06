use std::sync::Arc;
use async_trait::async_trait;
use cliclack::{self, confirm, log};
use serde::{Serialize, Deserialize};
use serde_json::json;

use crate::ai::{AiClient, Message, MessageRole};
use crate::error::{AgentError, AgentResult};
use crate::task::{SubTask, SubTaskType, SubTaskQueueManager};

/// Executor for subtasks
pub struct SubTaskExecutor {
    ai_client: Box<dyn AiClient>,
    queue_manager: Arc<SubTaskQueueManager>,
    user_confirmation_callback: Option<Box<dyn Fn(&SubTask) -> bool + Send + Sync>>,
}

impl SubTaskExecutor {
    pub fn new(ai_client: Box<dyn AiClient>) -> Self {
        Self {
            ai_client,
            queue_manager: SubTaskQueueManager::global(),
            user_confirmation_callback: None,
        }
    }
    
    pub fn set_user_confirmation_callback(
        &mut self,
        callback: Box<dyn Fn(&SubTask) -> bool + Send + Sync>,
    ) {
        self.user_confirmation_callback = Some(callback);
    }
    
    /// Add a new subtask to the queue
    pub fn add_queued_subtask(&self, subtask: SubTask) {
        self.queue_manager.add_queued_subtask(subtask);
    }
    
    /// Process user input to generate subtasks using the AI
    pub async fn process_user_input(&self, input: &str) -> AgentResult<()> {
        // Create system prompt for function calling
        let system_prompt = r#"
You are an expert AI assistant for a command-line tool that can help with various tasks.
Your job is to analyze user requests and determine what operations to perform.
You must select the most appropriate operations to complete a user's request.
"#;

        // Define the SubTaskType function schema for OpenAI function calling
        let functions = json!([
            {
                "name": "create_task",
                "description": "Create a new task based on user input",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "Description of the task to be performed"
                        }
                    },
                    "required": ["description"]
                }
            },
            {
                "name": "create_read",
                "description": "Create a subtask to read a file or content",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "Description of what needs to be read and why"
                        }
                    },
                    "required": ["description"]
                }
            },
            {
                "name": "create_update",
                "description": "Create a subtask to update or modify content",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "Description of what needs to be updated and how"
                        }
                    },
                    "required": ["description"]
                }
            },
            {
                "name": "create_search",
                "description": "Create a subtask to search for information",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "Description of what to search for"
                        }
                    },
                    "required": ["description"]
                }
            },
            {
                "name": "create_bash",
                "description": "Create a subtask to execute a bash command",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "Description of the bash command to execute"
                        }
                    },
                    "required": ["description"]
                }
            }
        ]);

        // Create messages for the AI
        let messages = vec![
            Message {
                role: MessageRole::System,
                content: system_prompt.to_string(),
                name: None,
            },
            Message {
                role: MessageRole::User,
                content: input.to_string(),
                name: None,
            },
        ];

        // Call AI with function calling enabled
        let response = self.ai_client
            .chat_completion_with_functions(messages, functions)
            .await?;
        
        // Process function calls if any
        if let Some(function_call) = response.function_call {
            let function_name = function_call.name.as_str();
            let args: serde_json::Value = serde_json::from_str(&function_call.arguments)?;
            
            let description = args["description"].as_str()
                .ok_or_else(|| AgentError::AiApi("Missing description parameter".to_string()))?
                .to_string();
            
            // Log before moving the description
            log::info(&format!("Added subtask: {}", description)).expect("Failed to log");
            
            // Create the appropriate subtask based on the function call
            let subtask = match function_name {
                "create_task" => SubTask::new(SubTaskType::Task(description), None),
                "create_read" => SubTask::new(SubTaskType::Read(description.clone()), None),
                "create_update" => SubTask::new(SubTaskType::Update(description.clone()), None),
                "create_search" => SubTask::new(SubTaskType::Search(description.clone()), None),
                "create_bash" => SubTask::new(SubTaskType::Bash(description.clone()), None),
                _ => return Err(AgentError::AiApi(format!("Unknown function: {}", function_name))),
            };
            
            // Add the subtask to the queue
            self.add_queued_subtask(subtask);
        } else {
            // No function call, create a generic task
            let subtask = SubTask::new(SubTaskType::Task(input.to_string()), None);
            self.add_queued_subtask(subtask);
            
            log::info(&format!("Added generic task: {}", input)).expect("Failed to log");
        }
        
        Ok(())
    }
    
    /// Process the next subtask in the queue
    pub async fn process_next_subtask(&self) -> AgentResult<bool> {
        if self.queue_manager.is_queue_empty() {
            return Ok(false);
        }
        
        // Get the next subtask
        if let Some(subtask) = self.queue_manager.next_subtask() {
            // Display the subtask
            log::info(&format!("{} Subtask: {}", 
                subtask.subtask_type.icon(),
                subtask.subtask_type.description()
            )).expect("Failed to log");
            
            // Ask for confirmation
            let confirmed = if let Some(callback) = &self.user_confirmation_callback {
                callback(&subtask)
            } else {
                confirm("Execute this subtask?")
                    .initial_value(true)
                    .interact()
                    .unwrap_or(false)
            };
            
            if confirmed {
                log::info("✓ Subtask approved").expect("Failed to log");
                // TODO: Execute the subtask based on its type
                // This would be implemented based on the specific subtask types
                // For now, we'll just return success
                return Ok(true);
            } else {
                log::info("⨯ Subtask declined").expect("Failed to log");
                return Ok(true); // Return true to continue processing the queue
            }
        }
        
        Ok(false)
    }
    
    /// Process all subtasks in the queue
    pub async fn process_all_subtasks(&self) -> AgentResult<()> {
        while !self.queue_manager.is_queue_empty() {
            self.process_next_subtask().await?;
        }
        
        Ok(())
    }
}