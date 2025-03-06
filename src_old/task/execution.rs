use crate::task::subtask::FilePathOrQuery;
use std::sync::Arc;
use async_trait::async_trait;
use cliclack::{self, confirm, log};
use serde::{Serialize, Deserialize};
use serde_json::json;

use crate::ai::{AiClient, Message, MessageRole};
use crate::error::{AgentError, AgentResult};
use crate::task::{
    SubTask, SubTaskType,    TaskStatus,  
};

/// Executor for subtasks
pub struct SubTaskExecutor {
 //   ai_client: Box<dyn AiClient>,
   // queue_manager: Arc<SubTaskQueueManager>,
   // user_confirmation_callback: Option<Box<dyn Fn(&SubTask) -> bool + Send + Sync>>,
}

impl SubTaskExecutor {
    pub fn new(ai_client: Box<dyn AiClient>) -> Self {
        Self {
            ai_client,
        // queue_manager: SubTaskQueueManager::global(),
            user_confirmation_callback: None,
        }
    }
    
    pub fn set_user_confirmation_callback(
        &mut self,
        callback: Box<dyn Fn(&SubTask) -> bool + Send + Sync>,
    ) {
        self.user_confirmation_callback = Some(callback);
    }
    
     

    
    /// Process user input to generate subtasks using the AI
   
        
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
                    
                    SubTaskType::Read(path_or_query) => {
                        match path_or_query {
                            FilePathOrQuery::FilePath(path) => {
                                // It's a file path, read the file
                                log::info(&format!("📖 Reading file: {}", path)).expect("Failed to log");
                                
                                use std::fs;
                                use std::path::Path;
                                
                                let file_path = Path::new(&path);
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
                            FilePathOrQuery::FileQuery(query) => {
                                // Search for files matching the query
                                log::info(&format!("🔍 Looking for file matching: {}", query)).expect("Failed to log");
                                
                                // Use find to search for files that might match
                                use tokio::process::Command;
                                
                                // Convert query to search terms
                                let search_terms: Vec<&str> = query.split_whitespace().collect();
                                
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
                                        log::info(&format!("📁 Files matching query '{}':", query)).expect("Failed to log");
                                        
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
                                        log::info(&format!("❌ No files found matching query: {}", query)).expect("Failed to log");
                                        
                                        // Update the subtask result
                                        let mut updated_subtask = subtask.clone();
                                        updated_subtask.status = TaskStatus::Completed;
                                        updated_subtask.result = Some("No matching files found".to_string());
                                    }
                                } else {
                                    log::info("❌ Invalid file query").expect("Failed to log");
                                }
                            }
                        }
                    },
                    
                    SubTaskType::Search( query) => {
                        use tokio::process::Command;
                        
                  //      match path_or_query {
                          /*  FilePathOrQuery::FilePath(path) => {
                                log::info(&format!("🔍 Searching specific file: {}", path)).expect("Failed to log");
                                
                                // In this case, we'll just read the file to examine it
                                use std::fs;
                                use std::path::Path;
                                
                                let file_path = Path::new(&path);
                                if file_path.exists() {
                                    // Read the file content to examine
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
                            },*/
                           // FilePathOrQuery::FileQuery(query) => {


                            //lets change this so it pops down a depth and   adds SUB-SUB tasks to help build our context ! 

                                log::info(&format!("🔍 Searching for content: {}", query)).expect("Failed to log");
                                
                                // First, search for files containing the query text
                                let grep_output = Command::new("sh")
                                    .arg("-c")
                                    .arg(format!("grep -r \"{}\" . --include=\"*.rs\" --include=\"*.toml\" 2>/dev/null | head -n 20", query))
                                    .output()
                                    .await
                                    .map_err(|e| AgentError::TaskExecution(format!("Failed to execute search: {}", e)))?;
                                
                                // Process the grep output
                                let grep_stdout = String::from_utf8_lossy(&grep_output.stdout).to_string();
                                
                                // Second, search for files with names matching the query
                                let find_output = Command::new("sh")
                                    .arg("-c")
                                    .arg(format!("find . -type f -name \"*{}*\" | grep -v \"target/\" | grep -v \"node_modules/\" | head -n 10", query))
                                    .output()
                                    .await
                                    .map_err(|e| AgentError::TaskExecution(format!("Failed to execute file search: {}", e)))?;
                                
                                let find_stdout = String::from_utf8_lossy(&find_output.stdout).to_string();
                                
                                // Combine results
                                let mut results = String::new();
                                let mut found_something = false;
                                
                                // Print content search results if any
                                if !grep_stdout.is_empty() {
                                    found_something = true;
                                    results.push_str(&format!("Content matching '{}':\n", query));
                                    results.push_str(&grep_stdout);
                                }
                                
                                // Print filename search results if any
                                if !find_stdout.is_empty() {
                                    if found_something {
                                        results.push_str("\n\n");
                                    }
                                    found_something = true;
                                    results.push_str(&format!("Files matching '{}':\n", query));
                                    results.push_str(&find_stdout);
                                }
                                
                                // Print the results
                                if found_something {
                                    println!();
                                    log::info(&format!("🔍 Search results for '{}':", query)).expect("Failed to log");
                                    
                                    // Print the content with some formatting
                                    let width = 100;
                                    let separator = "─".repeat(width);
                                    println!("┌{}┐", separator);
                                    
                                    for line in results.lines() {
                                        println!("│ {:<width$} │", line, width=width-2);
                                    }
                                    
                                    println!("└{}┘", separator);
                                    
                                    // Update the subtask result
                                    let mut updated_subtask = subtask.clone();
                                    updated_subtask.status = TaskStatus::Completed;
                                    updated_subtask.result = Some(results);
                                } else {
                                    log::info(&format!("❌ No matches found for '{}'", query)).expect("Failed to log");
                                    
                                    // Update the subtask result
                                    let mut updated_subtask = subtask.clone();
                                    updated_subtask.status = TaskStatus::Completed;
                                    updated_subtask.result = Some("No matches found".to_string());
                                }
                          //  }
                      //  }
                    },
                    
                    SubTaskType::Update(path_or_query) => {
                        match path_or_query {
                            FilePathOrQuery::FilePath(path) => {
                                log::info(&format!("✏️ Updating file: {}", path)).expect("Failed to log");
                                
                                use std::fs;
                                use std::path::Path;
                                
                                let file_path = Path::new(&path);
                                if file_path.exists() {
                                    // For safety, just show the update information for now
                                    println!();
                                    log::info(&format!("Would update file: {}", path)).expect("Failed to log");
                                    log::info("Update functionality not fully implemented yet").expect("Failed to log");
                                    
                                    // Read current content
                                    let content = fs::read_to_string(file_path)
                                        .map_err(|e| AgentError::TaskExecution(format!("Failed to read file: {}", e)))?;
                                    
                                    // Print the current content preview
                                    println!();
                                    log::info("📝 Current file content:").expect("Failed to log");
                                    
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
                            FilePathOrQuery::FileQuery(query) => {
                                log::info(&format!("🔍 Looking for file to update: {}", query)).expect("Failed to log");
                                
                                // Use tokio::process::Command to search for the file
                                use tokio::process::Command;
                                
                                // Search for files with names matching the query
                                let output = Command::new("sh")
                                    .arg("-c")
                                    .arg(format!("find . -type f -name \"*{}*\" | grep -v \"target/\" | grep -v \"node_modules/\" | head -n 5", query))
                                    .output()
                                    .await
                                    .map_err(|e| AgentError::TaskExecution(format!("Failed to search files: {}", e)))?;
                                
                                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                                
                                if !stdout.is_empty() {
                                    println!();
                                    log::info(&format!("📁 Found files matching '{}' that could be updated:", query)).expect("Failed to log");
                                    
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
                                    log::info(&format!("❌ No files found matching '{}'", query)).expect("Failed to log");
                                    
                                    // Update the subtask result
                                    let mut updated_subtask = subtask.clone();
                                    updated_subtask.status = TaskStatus::Completed;
                                    updated_subtask.result = Some("No matching files found".to_string());
                                }
                                
                                log::info("Update by query not fully implemented yet").expect("Failed to log");
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