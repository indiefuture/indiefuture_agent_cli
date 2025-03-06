use crate::memory::ContextMemory;
use crate::agent_engine::SharedState;
use crate::subtasks::SubTaskType;
 
use tokio::sync::Mutex;
use cliclack::{self, intro, outro, select, input};
use console::style;
use crate::agent_engine::AgentEngine;
//use crate::cli::commands::execute_command;
use crate::error::AgentResult;
use crate::config::Settings;
use std::io;
use std::sync::Arc;




/// Main CLI entry point
pub async fn run_cli(

    shared_state: Arc<SharedState>, 
    context_memory: Arc< Mutex<ContextMemory > >,

    settings: Arc<Settings>, 

    agent_engine: Arc<Mutex<AgentEngine>>


    ) -> AgentResult<()> {
    // Welcome message
    intro("IndieFuture Agent CLI").expect("Failed to show intro");
    cliclack::log::info("Your AI-powered assistant for complex tasks").expect("Failed to show info");
    
    // Main loop
    loop {
        let select_result = select("What would you like to do?")
            .item("task", "Execute a task", "Break down and execute a complex task")
           // .item("scan", "Scan codebase", "Scan and index your codebase for future tasks")
            .item("config", "Configure", "View or modify settings")
            .item("quit", "Quit", "Exit the application")
            .interact();
            
        let selected = match select_result {
            Ok(val) => val.to_owned(),
            Err(_) => "quit".to_owned()
        };
        
        match selected.as_str() {
            "task" => {
                let input_result = input("What task would you like to execute?")
                    .placeholder("Describe your task in detail...")
                    .interact();
                
                let task_description = match input_result {
                    Ok(val) => val,
                    Err(_) => String::new(),
                };
                
                if !task_description.is_empty() {

                    agent_engine.lock().await.push_subtask(
                          SubTaskType::Task(task_description.clone())
                        );
                   // execute_command("task", &task_description, settings.clone()).await?;
                }
            }
           // "scan" => {
                //execute_command("scan", "", settings.clone()).await?;
            //}
            "config" => {
               // execute_command("config", "", settings.clone()).await?;
            }
            "quit" | _ => {
                outro("Goodbye!").expect("Failed to show outro");
                break;
            }
        }


        // handle ALL subtasks until the entire stack (queue) is empty and then we loop again 
        agent_engine.lock().await.handle_subtasks( 


            Arc::clone( &shared_state ),
            Arc::clone( &context_memory  ),
            Arc::clone( &settings )

          ).await 





    }
    
    Ok(())
}