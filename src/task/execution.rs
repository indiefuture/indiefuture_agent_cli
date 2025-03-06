use std::sync::Arc;
use async_trait::async_trait;
use cliclack::{self, confirm, log};
use serde::{Serialize, Deserialize};
use serde_json::json;

use crate::ai::{AiClient, Message, MessageRole};
use crate::error::{AgentError, AgentResult};
use crate::task::{
    SubTask, SubTaskType, SubTaskQueueManager, TaskStatus,
    ReadAction, UpdateAction, SearchAction
};

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
                "name": "create_read_file",
                "description": "Create a subtask to read a specific file by path",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Full path to the file to read"
                        }
                    },
                    "required": ["file_path"]
                }
            },
            {
                "name": "create_read_lookup",
                "description": "Create a subtask to find and read a file matching a description",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "Description of the file to find and read"
                        }
                    },
                    "required": ["description"]
                }
            },
            {
                "name": "create_update_file",
                "description": "Create a subtask to update a specific file by path",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Full path to the file to update"
                        },
                        "content": {
                            "type": "string",
                            "description": "New content or changes to make to the file"
                        }
                    },
                    "required": ["file_path", "content"]
                }
            },
            {
                "name": "create_update_lookup",
                "description": "Create a subtask to find and update a file matching a description",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "Description of the file to find and update"
                        },
                        "content": {
                            "type": "string",
                            "description": "New content or changes to make to the file"
                        }
                    },
                    "required": ["description", "content"]
                }
            },
            {
                "name": "create_search_content",
                "description": "Create a subtask to search for content in files",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Content to search for in files"
                        }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "create_search_filename",
                "description": "Create a subtask to search for files by name pattern",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Filename pattern to search for"
                        }
                    },
                    "required": ["pattern"]
                }
            },
            {
                "name": "create_bash",
                "description": "Create a subtask to execute a bash command",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The bash command to execute"
                        }
                    },
                    "required": ["command"]
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
            
            // We're not using this variable directly, just checking if parameter exists in function args
            let _description = args["description"].as_str()
                .ok_or_else(|| AgentError::AiApi("Missing description parameter".to_string()))?
                .to_string();
            
            // Log that we got a function call
            log::info(&format!("Processing function: {}", function_name)).expect("Failed to log");
            
            // Create the appropriate subtask based on the function call
            let subtask = match function_name {
                "create_task" => {
                    // Extract description parameter
                    let description = args["description"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing description parameter".to_string()))?
                        .to_string();
                    
                    // Log it
                    log::info(&format!("Adding task: {}", description)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Task(description), None)
                },
                
                "create_read_file" => {
                    // Extract file_path parameter
                    let file_path = args["file_path"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing file_path parameter".to_string()))?
                        .to_string();
                    
                    // Log it
                    log::info(&format!("Adding read file subtask: {}", file_path)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Read(ReadAction::FilePath(file_path)), None)
                },
                
                "create_read_lookup" => {
                    // Extract description parameter
                    let description = args["description"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing description parameter".to_string()))?
                        .to_string();
                    
                    // Log it
                    log::info(&format!("Adding read lookup subtask: {}", description)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Read(ReadAction::LookForFile(description)), None)
                },
                
                "create_update_file" => {
                    // Extract file_path and content parameters
                    let file_path = args["file_path"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing file_path parameter".to_string()))?
                        .to_string();
                    
                    let content = args["content"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing content parameter".to_string()))?
                        .to_string();
                    
                    // Log it
                    log::info(&format!("Adding update file subtask: {}", file_path)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Update(UpdateAction::FilePath(file_path, content)), None)
                },
                
                "create_update_lookup" => {
                    // Extract description and content parameters
                    let description = args["description"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing description parameter".to_string()))?
                        .to_string();
                    
                    let content = args["content"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing content parameter".to_string()))?
                        .to_string();
                    
                    // Log it
                    log::info(&format!("Adding update lookup subtask: {}", description)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Update(UpdateAction::LookForFile(description, content)), None)
                },
                
                "create_search_content" => {
                    // Extract query parameter
                    let query = args["query"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing query parameter".to_string()))?
                        .to_string();
                    
                    // Log it
                    log::info(&format!("Adding content search subtask: {}", query)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Search(SearchAction::Content(query)), None)
                },
                
                "create_search_filename" => {
                    // Extract pattern parameter
                    let pattern = args["pattern"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing pattern parameter".to_string()))?
                        .to_string();
                    
                    // Log it
                    log::info(&format!("Adding filename search subtask: {}", pattern)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Search(SearchAction::FileName(pattern)), None)
                },
                
                "create_bash" => {
                    // Extract command parameter
                    let command = args["command"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing command parameter".to_string()))?
                        .to_string();
                    
                    // Log it
                    log::info(&format!("Adding bash subtask: {}", command)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Bash(command), None)
                },
                
                // Support legacy function names for backward compatibility
                "create_read" => {
                    let description = args["description"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing description parameter".to_string()))?
                        .to_string();
                    
                    if description.starts_with('/') || description.starts_with("./") {
                        log::info(&format!("Adding legacy read file subtask: {}", description)).expect("Failed to log");
                        SubTask::new(SubTaskType::Read(ReadAction::FilePath(description)), None)
                    } else {
                        log::info(&format!("Adding legacy read lookup subtask: {}", description)).expect("Failed to log");
                        SubTask::new(SubTaskType::Read(ReadAction::LookForFile(description)), None)
                    }
                },
                
                "create_update" => {
                    let description = args["description"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing description parameter".to_string()))?
                        .to_string();
                    
                    log::info(&format!("Adding legacy update subtask: {}", description)).expect("Failed to log");
                    
                    if description.starts_with('/') || description.starts_with("./") {
                        SubTask::new(SubTaskType::Update(UpdateAction::FilePath(
                            description, 
                            "// Placeholder content for update".to_string()
                        )), None)
                    } else {
                        SubTask::new(SubTaskType::Update(UpdateAction::LookForFile(
                            description, 
                            "// Placeholder content for update".to_string()
                        )), None)
                    }
                },
                
                "create_search" => {
                    let description = args["description"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing description parameter".to_string()))?
                        .to_string();
                    
                    log::info(&format!("Adding legacy search subtask: {}", description)).expect("Failed to log");
                    
                    if description.starts_with("file:") {
                        let pattern = description.trim_start_matches("file:").trim().to_string();
                        SubTask::new(SubTaskType::Search(SearchAction::FileName(pattern)), None)
                    } else {
                        SubTask::new(SubTaskType::Search(SearchAction::Content(description)), None)
                    }
                },
                
                _ => return Err(AgentError::AiApi(format!("Unknown function: {}", function_name))),
            };
            
            // Add the subtask to the queue
            self.add_queued_subtask(subtask);
        } else {
            // No function call, create a generic task
            let subtask = SubTask::new(SubTaskType::Task(input.to_string()), None);
            self.add_queued_subtask(subtask);
            
            log::info(&format!("Added generic task: {}", input)).expect("Failed to log");
            
            // Try to generate a search subtask for any key terms in the input
            // This makes the system more responsive to general queries
            let key_terms = extract_key_terms(input);
            
            if !key_terms.is_empty() {
                log::info(&format!("Automatically adding search subtasks for key terms")).expect("Failed to log");
                
                for term in key_terms {
                    // Create a search task for each key term
                    let search_subtask = SubTask::new(
                        SubTaskType::Search(SearchAction::Content(term.clone())), 
                        None
                    );
                    self.add_queued_subtask(search_subtask);
                    log::info(&format!("  - Added content search for: {}", term)).expect("Failed to log");
                }
            }
        }
        
        // Helper function to extract potential key search terms from input
        fn extract_key_terms(input: &str) -> Vec<String> {
            let mut terms = Vec::new();
            
            // Simple, naive approach - just look for words longer than 5 chars
            // In a real system, this would use NLP to extract actual key entities
            let words: Vec<&str> = input.split_whitespace()
                .filter(|w| w.len() >= 5)
                .collect();
            
            // Only take up to 3 search terms to avoid too many auto-generated tasks
            for word in words.iter().take(3) {
                // Remove any punctuation
                let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric()).to_string();
                if !clean_word.is_empty() {
                    terms.push(clean_word);
                }
            }
            
            terms
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
                
                // Execute the subtask based on its type
                match &subtask.subtask_type {
                    SubTaskType::Bash(command) => {
                        // Execute Bash command
                        log::info(&format!("🔧 Executing command: {}", command)).expect("Failed to log");
                        
                        // Use tokio::process::Command to execute the command
                        use tokio::process::Command;
                        let output = Command::new("sh")
                            .arg("-c")
                            .arg(command)
                            .output()
                            .await
                            .map_err(|e| AgentError::TaskExecution(format!("Failed to execute command: {}", e)))?;
                        
                        // Process the output
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        
                        // Print the results in a nice format
                        if !stdout.is_empty() {
                            println!();
                            log::info("📄 Command output:").expect("Failed to log");
                            
                            // Print the output with some formatting
                            let width = 80;
                            let separator = "─".repeat(width);
                            println!("┌{}┐", separator);
                            
                            // Split and limit output lines if too long
                            let max_lines = 20;
                            let lines: Vec<&str> = stdout.lines().collect();
                            let display_lines = if lines.len() > max_lines {
                                let mut truncated = lines[0..max_lines].to_vec();
                                truncated.push("... (output truncated)");
                                truncated
                            } else {
                                lines
                            };
                            
                            for line in display_lines {
                                println!("│ {:<width$} │", line, width=width-2);
                            }
                            
                            println!("└{}┘", separator);
                        }
                        
                        if !stderr.is_empty() {
                            println!();
                            log::info("⚠️ Command errors:").expect("Failed to log");
                            println!("{}", stderr);
                        }
                        
                        // Update the subtask result
                        let mut updated_subtask = subtask.clone();
                        updated_subtask.status = TaskStatus::Completed;
                        updated_subtask.result = Some(stdout);
                    },
                    
                    SubTaskType::Read(action) => {
                        match action {
                            ReadAction::FilePath(path) => {
                                // It's a file path, read the file
                                log::info(&format!("📖 Reading file: {}", path)).expect("Failed to log");
                                
                                use std::fs;
                                use std::path::Path;
                                
                                let file_path = Path::new(path);
                                if file_path.exists() {
                                    // Read the file content
                                    let content = fs::read_to_string(file_path)
                                        .map_err(|e| AgentError::TaskExecution(format!("Failed to read file: {}", e)))?;
                                    
                                    // Print the content
                                    println!();
                                    log::info(&format!("📄 Content of {}:", path)).expect("Failed to log");
                                    
                                    // Print the content with some formatting
                                    let width = 80;
                                    let separator = "─".repeat(width);
                                    println!("┌{}┐", separator);
                                    
                                    // Split and limit output lines if too long
                                    let max_lines = 30;
                                    let lines: Vec<&str> = content.lines().collect();
                                    let display_lines = if lines.len() > max_lines {
                                        let mut truncated = lines[0..max_lines].to_vec();
                                        truncated.push("... (content truncated)");
                                        truncated
                                    } else {
                                        lines
                                    };
                                    
                                    for line in display_lines {
                                        println!("│ {:<width$} │", line, width=width-2);
                                    }
                                    
                                    println!("└{}┘", separator);
                                    
                                    // Update the subtask result
                                    let mut updated_subtask = subtask.clone();
                                    updated_subtask.status = TaskStatus::Completed;
                                    updated_subtask.result = Some(content);
                                } else {
                                    log::info(&format!("❌ File not found: {}", path)).expect("Failed to log");
                                    
                                    // Update the subtask result with error
                                    let mut updated_subtask = subtask.clone();
                                    updated_subtask.status = TaskStatus::Failed;
                                    updated_subtask.result = Some(format!("File not found: {}", path));
                                }
                            },
                            ReadAction::LookForFile(description) => {
                                // Search for files matching the description
                                log::info(&format!("🔍 Looking for file matching: {}", description)).expect("Failed to log");
                                
                                // Use find to search for files that might match
                                use tokio::process::Command;
                                
                                // Convert description to search terms
                                let search_terms: Vec<&str> = description.split_whitespace().collect();
                                
                                if !search_terms.is_empty() {
                                    // Build a find command with multiple -name patterns
                                    let mut find_cmd = String::from("find . -type f");
                                    
                                    // Add name patterns for each term
                                    for term in &search_terms {
                                        find_cmd.push_str(&format!(" -o -name \"*{}*\"", term));
                                    }
                                    
                                    // Exclude common directories and limit results
                                    find_cmd.push_str(" | grep -v \"target/\" | grep -v \"node_modules/\" | head -n 10");
                                    
                                    // Execute the search
                                    let output = Command::new("sh")
                                        .arg("-c")
                                        .arg(&find_cmd)
                                        .output()
                                        .await
                                        .map_err(|e| AgentError::TaskExecution(format!("Failed to search files: {}", e)))?;
                                    
                                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                                    
                                    if !stdout.is_empty() {
                                        println!();
                                        log::info(&format!("📁 Files matching description '{}':", description)).expect("Failed to log");
                                        
                                        // Print the content with some formatting
                                        let width = 100;
                                        let separator = "─".repeat(width);
                                        println!("┌{}┐", separator);
                                        
                                        for line in stdout.lines() {
                                            println!("│ {:<width$} │", line, width=width-2);
                                        }
                                        
                                        println!("└{}┘", separator);
                                        
                                        // Update the subtask result
                                        let mut updated_subtask = subtask.clone();
                                        updated_subtask.status = TaskStatus::Completed;
                                        updated_subtask.result = Some(stdout);
                                    } else {
                                        log::info(&format!("❌ No files found matching description: {}", description)).expect("Failed to log");
                                        
                                        // Update the subtask result
                                        let mut updated_subtask = subtask.clone();
                                        updated_subtask.status = TaskStatus::Completed;
                                        updated_subtask.result = Some("No matching files found".to_string());
                                    }
                                } else {
                                    log::info("❌ Invalid file description").expect("Failed to log");
                                }
                            }
                        }
                    },
                    
                    SubTaskType::Search(action) => {
                        use tokio::process::Command;
                        
                        match action {
                            SearchAction::Content(query) => {
                                log::info(&format!("🔍 Searching for content: {}", query)).expect("Failed to log");
                                
                                // Search for files containing the query
                                let output = Command::new("sh")
                                    .arg("-c")
                                    .arg(format!("grep -r \"{}\" . --include=\"*.rs\" --include=\"*.toml\" 2>/dev/null | head -n 20", query))
                                    .output()
                                    .await
                                    .map_err(|e| AgentError::TaskExecution(format!("Failed to execute search: {}", e)))?;
                                
                                // Process the output
                                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                                
                                // Print the results
                                if !stdout.is_empty() {
                                    println!();
                                    log::info(&format!("🔍 Content search results for '{}':", query)).expect("Failed to log");
                                    
                                    // Print the content with some formatting
                                    let width = 100;
                                    let separator = "─".repeat(width);
                                    println!("┌{}┐", separator);
                                    
                                    for line in stdout.lines() {
                                        println!("│ {:<width$} │", line, width=width-2);
                                    }
                                    
                                    println!("└{}┘", separator);
                                    
                                    // Update the subtask result
                                    let mut updated_subtask = subtask.clone();
                                    updated_subtask.status = TaskStatus::Completed;
                                    updated_subtask.result = Some(stdout);
                                } else {
                                    log::info(&format!("❌ No content matches found for '{}'", query)).expect("Failed to log");
                                    
                                    // Update the subtask result
                                    let mut updated_subtask = subtask.clone();
                                    updated_subtask.status = TaskStatus::Completed;
                                    updated_subtask.result = Some("No content matches found".to_string());
                                }
                            },
                            SearchAction::FileName(pattern) => {
                                log::info(&format!("🔍 Searching for files: {}", pattern)).expect("Failed to log");
                                
                                // Search for files with names matching the pattern
                                let output = Command::new("sh")
                                    .arg("-c")
                                    .arg(format!("find . -type f -name \"*{}*\" | grep -v \"target/\" | grep -v \"node_modules/\" | head -n 10", pattern))
                                    .output()
                                    .await
                                    .map_err(|e| AgentError::TaskExecution(format!("Failed to execute file search: {}", e)))?;
                                
                                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                                
                                if !stdout.is_empty() {
                                    println!();
                                    log::info(&format!("📁 Files matching '{}':", pattern)).expect("Failed to log");
                                    
                                    // Print the content with some formatting
                                    let width = 100;
                                    let separator = "─".repeat(width);
                                    println!("┌{}┐", separator);
                                    
                                    for line in stdout.lines() {
                                        println!("│ {:<width$} │", line, width=width-2);
                                    }
                                    
                                    println!("└{}┘", separator);
                                    
                                    // Update the subtask result
                                    let mut updated_subtask = subtask.clone();
                                    updated_subtask.status = TaskStatus::Completed;
                                    updated_subtask.result = Some(stdout);
                                } else {
                                    log::info(&format!("❌ No files found matching '{}'", pattern)).expect("Failed to log");
                                    
                                    // Update the subtask result
                                    let mut updated_subtask = subtask.clone();
                                    updated_subtask.status = TaskStatus::Completed;
                                    updated_subtask.result = Some("No matching files found".to_string());
                                }
                            }
                        }
                    },
                    
                    SubTaskType::Update(action) => {
                        match action {
                            UpdateAction::FilePath(path, content) => {
                                log::info(&format!("✏️ Updating file: {}", path)).expect("Failed to log");
                                
                                use std::fs;
                                use std::path::Path;
                                
                                let file_path = Path::new(path);
                                if file_path.exists() {
                                    // For safety, just show the update information for now
                                    println!();
                                    log::info(&format!("Would update file: {}", path)).expect("Failed to log");
                                    log::info("Update functionality not fully implemented yet").expect("Failed to log");
                                    
                                    // Print the update content preview
                                    println!();
                                    log::info("📝 Update content preview:").expect("Failed to log");
                                    
                                    // Print the content with some formatting
                                    let width = 100;
                                    let separator = "─".repeat(width);
                                    println!("┌{}┐", separator);
                                    
                                    // Show the content preview
                                    let lines: Vec<&str> = content.lines().collect();
                                    let max_lines = 20;
                                    let display_lines = if lines.len() > max_lines {
                                        let mut truncated = lines[0..max_lines].to_vec();
                                        truncated.push("... (content truncated)");
                                        truncated
                                    } else {
                                        lines
                                    };
                                    
                                    for line in display_lines {
                                        println!("│ {:<width$} │", line, width=width-2);
                                    }
                                    
                                    println!("└{}┘", separator);
                                    
                                    // Update the subtask result
                                    let mut updated_subtask = subtask.clone();
                                    updated_subtask.status = TaskStatus::Completed;
                                    updated_subtask.result = Some("File update preview completed".to_string());
                                } else {
                                    log::info(&format!("❌ File not found: {}", path)).expect("Failed to log");
                                    
                                    // Update the subtask result with error
                                    let mut updated_subtask = subtask.clone();
                                    updated_subtask.status = TaskStatus::Failed;
                                    updated_subtask.result = Some(format!("File not found: {}", path));
                                }
                            },
                            UpdateAction::LookForFile(description, _content) => {
                                log::info(&format!("🔍 Looking for file to update: {}", description)).expect("Failed to log");
                                log::info("Update by description not fully implemented yet").expect("Failed to log");
                                
                                // Update the subtask result
                                let mut updated_subtask = subtask.clone();
                                updated_subtask.status = TaskStatus::Completed;
                                updated_subtask.result = Some("Update by description not implemented yet".to_string());
                            }
                        }
                    },
                    
                    // Handle other subtask types
                    _ => {
                        log::info(&format!("Subtask type {} not yet implemented", 
                                          subtask.subtask_type.icon())).expect("Failed to log");
                    }
                }
                
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