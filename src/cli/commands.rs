use crate::agent_engine::AgentEngine;
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
use crate::task::{
    SubTask, SubTaskType,   SubTaskExecutor,
    // Legacy imports
    OperationType, Task,  TaskExecutor, TaskStatus
};

/// Execute a CLI command
/*
pub async fn execute_command(command: &str, args: &str, settings: Arc<Settings>) -> AgentResult<()> {
    match command {
        "task" => execute_task(args, settings).await,
        "scan" => scan_codebase(args, settings).await,
        "config" => show_config(settings).await,
        _ => Err(AgentError::Cli(format!("Unknown command: {}", command))),
    }
}*/

/// Add a subtask to the queue
/*pub fn add_queued_subtask(subtask: SubTask) -> AgentResult<()> {
    let queue_manager = SubTaskQueueManager::global();
    queue_manager.add_queued_subtask(subtask);
    Ok(())
}*/

/// Execute a complex task by breaking it down and handling subtasks
pub async fn ask_confirmation_for_subtask(

    mut agent_engine : &mut AgentEngine , 

    subtask: SubTaskType,

     settings: Arc<Settings>

     ) -> AgentResult<()> {
    // Initialize components
   
   
    
    
    // Set up the user confirmation callback
    agent_engine.set_user_confirmation_callback(Box::new(|subtask: &SubTask| {
        // Get confirmation from the user
        println!("\n");
        
        log::info(&format!("{} AI wants to execute subtask:", 
            subtask.subtask_type.icon()
        )).expect("Failed to log");
        
        log::info(&format!("  {}", subtask.subtask_type.description())).expect("Failed to log");
        
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
    }))  ;
    
    // Process user input to generate subtasks
    log::info("🔄 Analyzing task...").expect("Failed to log");
    let spin = spinner();
    spin.start("Processing task...");
    
    // Process the user input and generate subtasks
    agent_engine.process_user_input(task_description).await?;
    
    spin.stop("Task analyzed ✓");
    
  /*

    let queue_size = queue_manager.queue_length();
    
    if queue_size > 0 {
        log::info(&format!("🔍 Generated {} subtasks", queue_size)).expect("Failed to log");
        
        // List the subtasks in the queue
        let subtasks = queue_manager.list_subtasks();
        println!();
        log::info("📋 Subtasks:").expect("Failed to log");
        
        for (i, subtask) in subtasks.iter().enumerate() {
            log::info(&format!(" {}. {} {}", 
                style(i + 1).bold(), 
                subtask.subtask_type.icon(),
                subtask.subtask_type.description()
            )).expect("Failed to log");
        }
        
        // Ask for confirmation before executing tasks
        println!();
        let confirmed = confirm("Do you want to execute these subtasks?")
            .initial_value(true)
            .interact()
            .unwrap_or(false);
            
        if confirmed {
            // Process all subtasks in the queue
            log::info("▶️ Executing subtasks...").expect("Failed to log");
            subtask_executor.process_all_subtasks().await?;
            
            // Display the results
            println!();
            log::info("🎉 Task execution complete!").expect("Failed to log");
        } else {
            // User cancelled task execution
            log::info("⨯ Task execution cancelled by user").expect("Failed to log");
        }
    } else {
        log::info("⚠️ No subtasks were generated").expect("Failed to log");
    }*/
    
    Ok(())
}


/*
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

*/