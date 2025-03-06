use cliclack::{self, spinner, confirm, log};
 
use crate::SubTask;
use crate::AiClient;
use std::env::args;
use std::sync::Arc;
use crate::{Settings, SubTaskType};

#[derive(Default)] 
pub struct AgentEngine {


	pub current_subtask_depth: usize, 
	pub active_subtasks : Vec<  SubtaskSlot   > ,

	pub context_memory: ContextMemory , 


	 // pub user_confirmation_callback: Option<Box<dyn Fn(&SubTask) -> bool + Send + Sync>>,
}


#[derive(Default)]
pub struct ContextMemory {} 


pub struct SubtaskSlot { depth: usize, subtask:  SubTaskType } 


impl AgentEngine {

	pub fn push_subtask(&mut self, new_subtask: SubTaskType ){
		let current_depth = self.current_subtask_depth ;

		self.active_subtasks.push (  

			SubtaskSlot {
				depth: current_depth, 
				subtask: new_subtask, 
			}

		 ); 
	}

  /*  pub fn set_user_confirmation_callback(
        &mut self,
        callback: Box<dyn Fn(&SubTask) -> bool + Send + Sync>,
    ) {
        self.user_confirmation_callback = Some(callback);
    }*/
    
    /// Ask for user confirmation before executing a subtask
    pub async fn ask_user_confirmation(&self, subtask_type: SubTaskType) -> bool {
        // Display the subtask description
        cliclack::log::info(&format!("{} Operation: {}", 
            subtask_type.icon(),
            subtask_type.description()
        )).expect("Failed to log");
        
        // Create a temporary SubTask object for the callback
        let _subtask = SubTask::new(subtask_type, None);
        
        // If a custom callback is provided, use it
       
            // Otherwise use the default confirmation prompt
            confirm("Execute this operation?")
                .initial_value(true)
                .interact()
                .unwrap_or(false)
        
    }
     

    pub async fn perform_subtask(&self, subtask_type: SubTaskType, settings: Arc<Settings>) -> SubtaskOutput {
        // Get the appropriate tool for this subtask type
        let tool = subtask_type.get_tool();
        
        // Create AI client as needed
        let api_key = settings.claude_api_key.as_ref()
            .cloned()
            .unwrap_or_else(|| "dummy_key".to_string());
        let model = &settings.default_model;
        
        let ai_client = match crate::ai::claude::ClaudeClient::new(&api_key, model) {
            Ok(client) => Arc::new(client) as Arc<dyn AiClient>,
            Err(_) => {
                // Return early if we can't create a client
                return SubtaskOutput::SubtaskComplete();
            }
        };
        
        // Execute the subtask
        match tool.handle_subtask(ai_client).await {
            Some(output) => output,
            None => SubtaskOutput::SubtaskComplete()
        }
    }


 /*


 */

	pub async fn handle_subtasks( 
		&mut self, 

		_shared_state: Arc<SharedState>, 
		_context_memory: Arc< ContextMemory  >,
		settings: Arc<Settings> 

		){


		// execute_command("task", &task_description, settings.clone()).await?;


			//need to handle differently if we are popping up to next depth or not ! 

		loop {



			if let Some( _next_subtask)  = self.active_subtasks.last () {


			}



			let Some(next_subtask) = self.active_subtasks.pop() else {   //returns the last element ! 
				break;
			};

			let confirmed =  self.ask_user_confirmation( next_subtask.subtask.clone() )  .await ;

			match confirmed {

				true =>  {
					  cliclack::log::info("✓ Operation approved").expect("Failed to log");

					    let spin = spinner();
    					spin.start("Processing task...");
					  let subtask_output =  self.perform_subtask(  next_subtask.subtask , Arc::clone( &settings )  ).await; 
					   spin.stop("Task analyzed ✓");

					   	  cliclack::log::info(format!(" {:?}" , subtask_output ) ).expect("Failed to log");

				}
				false => {
					cliclack::log::info("⨯ Operation declined").expect("Failed to log");
					break
				}
			}

			//handle this next subtask 
		   


		} 


	}


 



}



#[derive(Debug)]
pub enum SubtaskOutput {
	PushSubtasksIncrementDepth(Vec<SubTaskType>),  // add subtasks in a deeper depth to try and grow context -- once those are all popped off and handled, we have more context to try again !  
	SubtaskComplete(),  //we have enough context to do an AI Query or to move on  
	//SubtaskFailed, // we are giving up . when would this happen ? 
}




pub struct SharedState {

    pub ai_client: Box<dyn AiClient>

}

