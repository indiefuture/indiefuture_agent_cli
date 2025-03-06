use crate::memory::ContextMemory;
use crate::memory::MemoryFragment;
use crate::subtasks::subtask::SubTask;
use crate::subtasks::subtask::SubTaskType;
use cliclack::{self, confirm, log, spinner};
use tokio::sync::Mutex;

use crate::AiClient;
use crate::Settings;
use std::env::args;
use std::sync::Arc;

#[derive(Default)]
pub struct AgentEngine {
    pub current_subtask_depth: usize,
    pub active_subtasks: Vec<SubtaskSlot>,

    pub context_memory: ContextMemory,
    // pub user_confirmation_callback: Option<Box<dyn Fn(&SubTask) -> bool + Send + Sync>>,
}

#[derive(Clone, Debug)]
pub struct SubtaskSlot {
    depth: usize,
    subtask: SubTaskType,
}

impl AgentEngine {
    pub fn push_subtask(&mut self, new_subtask: SubTaskType) {
        let current_depth = self.current_subtask_depth;

        self.active_subtasks.push(SubtaskSlot {
            depth: current_depth,
            subtask: new_subtask,
        });
    }

    pub fn increment_subtask_depth(&mut self) {
        self.current_subtask_depth += 1;

        println!("increment_subtask_depth {}", self.current_subtask_depth);
    }

    pub fn set_subtask_depth(&mut self, new_depth: usize) {
        self.current_subtask_depth = new_depth;

        println!("set task depth {}", new_depth);
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
        cliclack::log::info(&format!(
            "{} Operation: {}",
            subtask_type.icon(),
            subtask_type.description()
        ))
        .expect("Failed to log");

        // Create a temporary SubTask object for the callback
        let _subtask = SubTask::new(subtask_type, None);

        // If a custom callback is provided, use it

        // Otherwise use the default confirmation prompt
        confirm("Execute this operation?")
            .initial_value(true)
            .interact()
            .unwrap_or(false)
    }

    pub async fn perform_subtask(
        &self,
        subtask_type: SubTaskType,
        context_memory: Arc<Mutex<ContextMemory>>,
        shared_state: Arc<SharedState>,
        settings: Arc<Settings>,
    ) -> SubtaskOutput {
        // Get the appropriate tool for this subtask type
        let tool = subtask_type.get_tool();

        let ai_client = &shared_state.ai_client;

        // Execute the subtask
        match tool.handle_subtask(ai_client, context_memory).await {
            Some(output) => output,
            None => SubtaskOutput::SubtaskComplete(),
        }
    }

    /*


    */

    pub async fn handle_subtasks(
        &mut self,

        shared_state: Arc<SharedState>,
        context_memory: Arc<Mutex<ContextMemory>>,
        settings: Arc<Settings>,
    ) {
        // execute_command("task", &task_description, settings.clone()).await?;

        //need to handle differently if we are popping up to next depth or not !

        loop {
            if let Some(next_subtask) = self.active_subtasks.last() {
                if next_subtask.depth != self.current_subtask_depth {
                    self.set_subtask_depth(next_subtask.depth);
                }
            }


 




            let Some(next_subtask) = self.active_subtasks.pop() .clone() else {

 


                //returns the last element !
                break;
            };

            let confirmed = match next_subtask.subtask.requires_user_permission() {
                true => {
                    self.ask_user_confirmation(next_subtask.subtask.clone())
                        .await
                }
                false => true,
            };

            match confirmed {
                true => {
                    cliclack::log::info("✓ Operation approved").expect("Failed to log");

                    let spin = spinner();
                    spin.start("Processing task... ");

                    cliclack::log::info(format!(" TASK {:?}", next_subtask.subtask))
                        .expect("Failed to log");

                    let subtask_output = self
                        .perform_subtask(
                            next_subtask.subtask.clone(),
                            Arc::clone(&context_memory),
                            Arc::clone(&shared_state),
                            Arc::clone(&settings),
                        )
                        .await;

                    spin.stop("Task analyzed ✓");












                    let task_is_completed = match   next_subtask.subtask .clone() {

                    	SubTaskType::Task( _ ) => false, // for now 
                    	_ => true 
                    }; 

                    if !task_is_completed {  //re push self to stack 
                    	 cliclack::log::info("⨯ Task not yet complete - retrying with more context ").expect("Failed to log");
                    	self.push_subtask(next_subtask.subtask.clone() );
                    }






                    match subtask_output {
                        SubtaskOutput::AddToContextMemory(ref memory_fragment) => {
                            context_memory
                                .lock()
                                .await
                                .add_frag(memory_fragment.clone());
                        }

                        SubtaskOutput::PushSubtasks(ref new_tasks_array) => {
                            //self.push_subtask( next_subtask.subtask.clone() );  //lets come back to this after we handle the lower depth tasks we are about to create !

                            for new_subtask in new_tasks_array {
                                //increment depth here !?

                                self.push_subtask(new_subtask.clone());
                            }
                        }

                        // we are going to try and do a bunch of subtasks and then come back and re-attempt this with better context
                        SubtaskOutput::PushSubtasksIncrementDepth(ref new_tasks_array) => {
                            self.push_subtask(next_subtask.subtask.clone()); //lets come back to this after we handle the lower depth tasks we are about to create !

                            for new_subtask in new_tasks_array {
                                //increment depth here !?

                                self.increment_subtask_depth();
                                self.push_subtask(new_subtask.clone());
                            }
                        }

                        _ => {
                            // ???
                        }
                    }





 

                   // cliclack::log::info(format!(" {:?}", subtask_output)).expect("Failed to log");
                }
                false => {


                 

                    cliclack::log::info("⨯ Operation declined").expect("Failed to log");
                    break;
                }
            }

            //handle this next subtask
        }
    }
}

#[derive(Debug)]
pub enum SubtaskOutput {
    PushSubtasksIncrementDepth(Vec<SubTaskType>), // add subtasks in a deeper depth to try and grow context -- once those are all popped off and handled, we have more context to try again !
    PushSubtasks(Vec<SubTaskType>),
    AddToContextMemory(MemoryFragment),

    SubtaskComplete(), //we have enough context to do an AI Query or to move on
                       //SubtaskFailed, // we are giving up . when would this happen ?
}

pub struct SharedState {
    pub ai_client: Box<dyn AiClient>,
}
