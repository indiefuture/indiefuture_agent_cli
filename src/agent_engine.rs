

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
        log::info(&format!("{} Operation: {}", 
            subtask_type.icon(),
            subtask_type.description()
        )).expect("Failed to log");
        
        // Create a temporary SubTask object for the callback
        let subtask = SubTask::new(subtask_type, None);
        
        // If a custom callback is provided, use it
       
            // Otherwise use the default confirmation prompt
            confirm("Execute this operation?")
                .initial_value(true)
                .interact()
                .unwrap_or(false)
        
    }
     

        pub async fn perform_subtask(&self, subtask_type: SubTaskType, settings: Arc<Settings>) -> SubtaskOutput {








        	


















        	





        }


 /*



 */

	pub async fn handle_subtasks( 
		&mut self, 

		shared_state: Arc<SharedState>, 
		context_memory: Arc< ContextMemory  >,
		settings: Arc<Settings> 

		){


		// execute_command("task", &task_description, settings.clone()).await?;


			//need to handle differently if we are popping up to next depth or not ! 

		loop {

			let Some(next_subtask) = self.active_subtasks.pop() else {
				break;
			};

			let confirmed =  self.ask_user_confirmation( next_subtask.subtask.clone() )  .await ;

			match confirmed {

				true =>  {
					  log::info("✓ Operation approved").expect("Failed to log");

					    let spin = spinner();
    					spin.start("Processing task...");
					   self.perform_subtask(  next_subtask.subtask , Arc::clone( &settings )  ).await; 
					   spin.stop("Task analyzed ✓");
				}
				false => {
					log::info("⨯ Operation declined").expect("Failed to log");
					break
				}
			}

			//handle this next subtask 
		   


		} 


	}


 



}



pub enum SubtaskOutput {


	PushSubtasksIncrementDepth,  // add subtasks in a deeper depth to try and grow context to try again 

	SubtaskComplete (   )



}




pub struct SharedState {

    pub ai_client: Box<dyn AiClient>

}



