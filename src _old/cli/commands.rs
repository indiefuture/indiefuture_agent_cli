use cliclack::{self, spinner, confirm, log};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::ai::{create_ai_client, Message, MessageRole};
use crate::codebase::CodebaseScanner;
use crate::config::Settings;
use crate::error::{AgentError, AgentResult};
use crate::memory::{MemoryManager, ShortTermMemory, VectorStore};
use crate::storage::SledStore;
use crate::task::{OperationType, Task, TaskDecomposer, TaskExecutor, TaskStatus};

/// Execute a CLI command
pub async fn execute_command(command: &str, args: &str, settings: Arc<Settings>) -> AgentResult<()> {
    match command {
        "task" => execute_task(args, settings).await,
        "scan" => scan_codebase(args, settings).await,
        "config" => show_config(settings).await,
        _ => Err(AgentError::Cli(format!("Unknown command: {}", command))),
    }
}

/// Execute a complex task by breaking it down and handling subtasks
async fn execute_task(task_description: &str, settings: Arc<Settings>) -> AgentResult<()> {
    // Initialize components
    let ai_client = create_ai_client(
        &settings.default_ai_provider,
        &settings.default_model,
        settings.openai_api_key.as_deref().unwrap_or(""),
    )?;
    
    // Initialize vector store
    let vector_store = VectorStore::new(settings.vector_store_path.to_str().unwrap_or("."), &settings.collection_name).await;
    
    // Initialize short-term memory
    let short_term_memory = ShortTermMemory::new(None);
    
    // Create memory manager
    let memory_manager = Arc::new(MemoryManager::new(short_term_memory, vector_store));
    
    // Create codebase scanner
    let codebase_scanner = Arc::new(CodebaseScanner::new(
        &settings.default_scan_path,
        settings.ignore_patterns.clone(),
        settings.supported_extensions.clone(),
    ));
    
    // Create task decomposer
    let decomposer = TaskDecomposer::new(
        ai_client.clone_box(),
        settings.default_timeout_seconds,
    );
    
    // Show spinner while decomposing the task
    log::info("🔄 Breaking down task into subtasks...").expect("Failed to log");
    let mut spin = spinner();
    spin.start("Analyzing task requirements...");
    
    // Decompose the task into subtasks and mark the main task as a TASK operation
    let mut tasks = decomposer.decompose(task_description).await?;
    
    // The first task should already be marked as a TASK operation in the decomposer
    if let Some(task) = tasks.first() {
        log::info(&format!("Created main TASK with ID: {}", task.id)).expect("Failed to log");
    }
    
    spin.stop("Task breakdown completed ✓");
    
    // Create a progress bar for task execution
    let total_tasks = tasks.len() as u64;
    let pb = ProgressBar::new(total_tasks);
    let pb_clone = pb.clone();
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {wide_msg}")
            .expect("Failed to set progress bar style")
            .progress_chars("█▓▒░"),
    );
    
    // Display the task breakdown with nicer formatting
    let parent_task = tasks.first().expect("No parent task found");
    // Get parent task operation icon
    let parent_icon = match &parent_task.operation_type {
        Some(op_type) => op_type.icon(),
        None => "📋",
    };
    
    log::info(&format!("{} Main task: {}", parent_icon, parent_task.description)).expect("Failed to log");
    println!();
    log::info("✅ Task successfully broken down into the following subtasks:").expect("Failed to log");
    
    for (i, task) in tasks.iter().skip(1).enumerate() {
        // Get operation icon based on operation type
        let op_icon = match &task.operation_type {
            Some(op_type) => op_type.icon(),
            None => "➡️",
        };
        
        log::info(&format!(" {}. {} {}", style(i + 1).bold(), op_icon, task.description)).expect("Failed to log");
    }
    println!();
    
    // Create task executor with the workspace root
    let mut executor = TaskExecutor::new(
        memory_manager.clone(),
        ai_client.clone_box(),
        codebase_scanner.clone(),
        settings.default_scan_path.clone(), // Use the scan path as workspace root
        settings.max_concurrent_tasks,
    );
    
    // Set up the status callback to update the progress bar
    executor.set_status_callback(Box::new(move |task: &Task| {
        match task.status {
            TaskStatus::Completed => {
                pb_clone.inc(1);
                pb_clone.set_message(format!("Completed: {}", task.description));
            }
            TaskStatus::Failed => {
                pb_clone.set_message(format!("Failed: {}", task.description));
            }
            TaskStatus::InProgress => {
                pb_clone.set_message(format!("In progress: {}", task.description));
            }
            _ => {}
        }
    }));
    
    // Set up the user confirmation callback
    executor.set_user_confirmation_callback(Box::new(|tool: &str, command: &str| {
        // Get confirmation from the user - one at a time
        println!("\n");
        
        if tool == "question" {
            log::info(&format!("AI is asking for your decision:\n{}", command)).expect("Failed to log");
            let result = confirm("Continue with this step?")
                .initial_value(true)
                .interact()
                .unwrap_or(false);
                
            if result {
                log::info("✓ Continuing with the next step").expect("Failed to log");
            } else {
                log::info("⨯ Stopping execution").expect("Failed to log");
            }
            result
        } else {
            // Tool execution
            if tool == "bash" {
                log::info(&format!("🔧 AI wants to run command: {}", style(command).bold().cyan())).expect("Failed to log");
            } else {
                log::info(&format!("🔧 AI wants to use tool: {}", style(tool).bold())).expect("Failed to log");
                log::info(&format!("  Command: {}", command)).expect("Failed to log");
            }
            
            let result = confirm("Allow this operation?")
                .initial_value(true)
                .interact()
                .unwrap_or(false);
                
            if result {
                log::info("✓ Operation approved").expect("Failed to log");
            } else {
                log::info("⨯ Operation declined").expect("Failed to log");
            }
            result
        }
    }));
    
    // Deduplicate tasks before queueing
    let mut deduped_tasks = tasks;
    deduplicate_bash_tasks(&mut deduped_tasks);
    
    // Queue the tasks for execution
    executor.queue_tasks(deduped_tasks)?;
    
    // Ask for confirmation before executing tasks
    println!();
    let confirmed = confirm("Do you want to execute these subtasks?")
        .initial_value(true)
        .interact()
        .unwrap_or(false);
        
    if confirmed {
            // Execute all tasks
            log::info("▶️ Executing tasks...").expect("Failed to log");
            let task_results = executor.execute_tasks().await?;
            
            // Finish the progress bar
            pb.finish_with_message("✨ All tasks completed");
            
            // Display the results
            println!();
            log::info("🎉 Task execution complete!").expect("Failed to log");
            
            // Find the parent task result
            if let Some(parent) = task_results.values().find(|t| t.parent_id.is_none()) {
                if let Some(result) = &parent.result {
                    log::info("🔍 Final result:").expect("Failed to log");
                    println!();
                    
                    // Print the result with a nice box around it
                    let width = 100;
                    let separator = "─".repeat(width);
                    println!("┌{}┐", separator);
                    
                    // Split the result into lines and print with box borders
                    for line in result.lines() {
                        println!("│ {:<width$} │", line, width=width-2);
                    }
                    
                    println!("└{}┘", separator);
                }
                
                // Print a summary of completed subtasks with their operation types
                let completed_subtasks: Vec<_> = task_results.values()
                    .filter(|t| t.parent_id.is_some() && t.status == TaskStatus::Completed)
                    .collect();
                
                if !completed_subtasks.is_empty() {
                    println!();
                    log::info("📊 Completed subtasks summary:").expect("Failed to log");
                    
                    for (i, task) in completed_subtasks.iter().enumerate() {
                        // Get operation icon based on operation type
                        let op_icon = match &task.operation_type {
                            Some(op_type) => op_type.icon(),
                            None => "✅",
                        };
                        
                        log::info(&format!(" {}. {} {}", i + 1, op_icon, task.description))
                            .expect("Failed to log");
                    }
                }
            }
    } else {
        // User cancelled task execution
        log::info("⨯ Task execution cancelled by user").expect("Failed to log");
        pb.finish_with_message("❌ Task execution cancelled");
        return Ok(());
    }
    
    Ok(())
}

/// Scan the codebase to build the vector index
async fn scan_codebase(path_str: &str, settings: Arc<Settings>) -> AgentResult<()> {
    let scan_path = if path_str.is_empty() {
        settings.default_scan_path.clone()
    } else {
        PathBuf::from(path_str)
    };
    
    if !scan_path.exists() {
        return Err(AgentError::Cli(format!(
            "Path does not exist: {:?}",
            scan_path
        )));
    }
    
    // Initialize vector store
    let vector_store = VectorStore::new(settings.vector_store_path.to_str().unwrap_or("."), &settings.collection_name).await;
    
    // Initialize short-term memory
    let short_term_memory = ShortTermMemory::new(None);
    
    // Create memory manager
    let memory_manager = Arc::new(MemoryManager::new(short_term_memory, vector_store));
    
    // Create codebase scanner
    let codebase_scanner = Arc::new(CodebaseScanner::new(
        &scan_path,
        settings.ignore_patterns.clone(),
        settings.supported_extensions.clone(),
    ));
    
    // Show status while scanning
    println!("Scanning codebase at {:?}...", scan_path);
    
    // Scan the directory
    let files = codebase_scanner.scan_directory(&scan_path).await?;
    
    println!("Scanned {} files", files.len());
    
    // Create a progress bar for storing embeddings
    let total_files = files.len() as u64;
    let pb = ProgressBar::new(total_files);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .expect("Failed to set progress bar style")
            .progress_chars("#>-"),
    );
    
    // Store files in memory
    for file in files {
        use crate::memory::MemoryMetadata;
        
        // Only process if the file has content and is not too large
        if !file.content.is_empty() && file.content.len() < 100000 {
            let metadata = MemoryMetadata {
                source: "codebase".to_string(),
                file_path: Some(file.path.clone()),
                language: file.language.clone(),
                tags: vec!["code".to_string()],
                task_id: None,
            };
            
            if memory_manager.vector_store.is_available() {
                let _ = memory_manager.add_memory(&file.content, metadata).await;
            }
            
            pb.inc(1);
            pb.set_message(file.path.clone());
        }
    }
    
    pb.finish_with_message("All files indexed");
    cliclack::log::info("Codebase scanning complete!").expect("Failed to log");
    
    Ok(())
}

/// Display and edit configuration
async fn show_config(settings: Arc<Settings>) -> AgentResult<()> {
    cliclack::log::info("Current Configuration:").expect("Failed to log");
    println!();
    cliclack::log::info(&format!("AI Provider: {}", settings.default_ai_provider)).expect("Failed to log");
    cliclack::log::info(&format!("Default Model: {}", settings.default_model)).expect("Failed to log");
    cliclack::log::info(&format!("Vector Store Path: {:?}", settings.vector_store_path)).expect("Failed to log");
    cliclack::log::info(&format!("Sled Path: {:?}", settings.sled_path)).expect("Failed to log");
    cliclack::log::info(&format!("Default Scan Path: {:?}", settings.default_scan_path)).expect("Failed to log");
    cliclack::log::info(&format!("Max Concurrent Tasks: {}", settings.max_concurrent_tasks)).expect("Failed to log");
    cliclack::log::info(&format!(
        "Default Timeout: {} seconds",
        settings.default_timeout_seconds
    )).expect("Failed to log");
    println!();
    cliclack::log::info(&format!("OpenAI API Key: {}", 
        if settings.openai_api_key.is_some() { "[Set]" } else { "[Not Set]" }
    )).expect("Failed to log");
    
    cliclack::log::info(&format!("Claude API Key: {}", 
        if settings.claude_api_key.is_some() { "[Set]" } else { "[Not Set]" }
    )).expect("Failed to log");
    println!();
    
    // In a more complete implementation, we would allow editing the configuration here
    cliclack::log::info("To modify settings, edit the .env file or set environment variables.").expect("Failed to log");
    
    Ok(())
}

/// Helper function to deduplicate BASH tasks to prevent multiple identical commands
fn deduplicate_bash_tasks(tasks: &mut Vec<Task>) {
    if tasks.len() <= 2 {
        // Not enough tasks to deduplicate
        return;
    }
    
    // Find the parent task
    let parent_id = tasks[0].id.clone();
    
    // Keep track of seen BASH commands
    let mut seen_commands = HashSet::new();
    let mut indices_to_remove = Vec::new();
    
    // First pass: collect all commands
    for (i, task) in tasks.iter().enumerate() {
        // Skip the parent task
        if i == 0 {
            continue;
        }
        
        // Check if it's a BASH task
        if let Some(op_type) = &task.operation_type {
            if *op_type == OperationType::BASH {
                // Extract the command
                let command = if task.description.contains("Execute command:") {
                    task.description.replace("Execute command:", "").trim().to_string()
                } else {
                    task.description.clone()
                };
                
                // Note which indices are duplicates
                if !seen_commands.insert(command.clone()) {
                    log::info(&format!("🧹 Removing duplicate task: {}", command)).expect("Failed to log");
                    indices_to_remove.push(i);
                }
            }
        }
    }
    
    // Sort indices in reverse order to avoid invalidating indices
    indices_to_remove.sort_unstable_by(|a, b| b.cmp(a));
    
    // Remove duplicates
    for index in indices_to_remove {
        tasks.remove(index);
    }
    
    if !indices_to_remove.is_empty() {
        log::info(&format!("🧹 Removed {} duplicate tasks", indices_to_remove.len())).expect("Failed to log");
    }
}