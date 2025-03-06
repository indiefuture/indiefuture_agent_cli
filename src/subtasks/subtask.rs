use crate::ai::openai::GptToolCall;
use crate::memory::ContextMemory;
use tokio::sync::Mutex;
 
use cliclack::log as cliclack_log;
use cliclack::input;
use crate::ai::MessageRole;
use crate::AiClient;
use serde_json::json;
use crate::ai::Message;


use cliclack::log; 

use crate::AgentError;
use std::fmt;
use std::sync::Arc;
use serde::Serialize;
use serde::Deserialize;
use async_trait::async_trait;
use crate::agent_engine::SubtaskOutput;

#[ async_trait ] 
pub trait SubtaskTool {
	  async fn handle_subtask(&self, ai_client: &Box<dyn AiClient> , context_memory: Arc<Mutex<ContextMemory>>  ) -> Option<SubtaskOutput > ; 
}

 
/// Represents the type of subtask to perform
#[derive(Debug, Clone)]
pub enum SubTaskType {
    Task(String),
    
    
    Bash(String),

    FileReadTool(FileReadToolInputs),

     FileEditTool(FileEditToolInputs),

      LSTool(LSToolInputs),
     GlobTool(GlobToolInputs),
    GrepTool(GrepToolInputs),

}



impl SubTaskType {
    pub fn get_tool(&self) -> Arc<dyn SubtaskTool> {
        match self {
            Self::Task(query)  =>  Arc::new(TaskTool (query.to_string()) ),

             Self::FileReadTool( input  )  =>  Arc::new(FileReadTool (  input .clone() )),

             Self::FileEditTool( input ) => Arc::new( FileEditTool( input.clone() ) ),

             Self::Bash( input ) => Arc::new( BashTool( input.to_string()  ) ),

         
             Self::LSTool( input)  => Arc::new(  LSTool ( input.clone()  )   ),

              Self::GlobTool( input)  => Arc::new(  GlobTool ( input.clone()  )   ),
            Self::GrepTool( input)  => Arc::new(  GrepTool ( input.clone()  )   ) ,
            
         
            // Other cases should return their respective tool implementations



        //    _ => unimplemented!("Tool not implemented"),
        }
    }
}








impl SubTaskType {





    pub fn from_tool_call( tool_call: GptToolCall ) -> Option<Self> {




          let function_name = tool_call.function.name.as_str();
          let args :serde_json::Value  = serde_json::from_str(  tool_call.function.arguments.as_str()? ).unwrap_or_default();


       /*  let args: serde_json::Value = match &tool_call.function.arguments.as_array() {
            Some(args) => args,
            None  => {
                         println!("WARN no function match  ");
                    return None
                },
        };*/
            
        // Log that we got a function call
        let _ = cliclack::log::info(format!("Processing function: {} {:?}", function_name, args  ) );
            
        // Create the appropriate subtask based on the function call
        let subtask = match function_name {
            "create_task" => {
                // Extract description parameter
                let description = match args["description"].as_str() {
                    Some(desc) => desc.to_string(),
                    None => return None,
                };
                
                // Log it
             
                let _ = cliclack::log::info(format!("Adding task: {}", description) );
                
                SubTaskType::Task(description) 
            },
            
            "read_file_at_path" => {
                // Extract file_path parameter
                let file_path = match args["file_path"].as_str() {
                    Some(path) => path.to_string(),
                    None => return None,
                };
                
                // Log it
                let _ = cliclack::log::info(format!("Adding read file subtask: {}", file_path));
                
                 SubTaskType::Read(FilePathOrQuery::FilePath(file_path)) 
            },
            
            "read_file_from_lookup" => {
                // Extract lookup_query parameter
                let lookup_query = match args["lookup_query"].as_str() {
                    Some(query) => query.to_string(),
                    None => return None,
                };
                
                // Log it
                let _ = cliclack::log::info(format!("Adding read lookup subtask: {}", lookup_query));
                
                 SubTaskType::Read(FilePathOrQuery::FileQuery(lookup_query)) 
            },
            
            "update_file_at_path" => {
                // Extract file_path parameter
                let file_path = match args["file_path"].as_str() {
                    Some(path) => path.to_string(),
                    None => return None,
                };
                
                // Log it
                let _ = cliclack::log::info(format!("Adding update file subtask: {}", file_path));
                
                 SubTaskType::Update(FilePathOrQuery::FilePath(file_path)) 
            },
            
            "update_file_from_lookup" => {
                // Extract lookup_query parameter
                let lookup_query = match args["lookup_query"].as_str() {
                    Some(query) => query.to_string(),
                    None => return None,
                };
                
                // Log it
                let _ = cliclack::log::info(format!("Adding update lookup subtask: {}", lookup_query));
                
                SubTaskType::Update(FilePathOrQuery::FileQuery(lookup_query))
            },
            
            "search_for_file" => {
                // Extract query parameter
                let query = match args["query"].as_str() {
                    Some(q) => q.to_string(),
                    None => return None,
                };
                
                // Log it
                let _ = cliclack::log::info(format!("Adding search subtask: {}", query));
                
                 SubTaskType::Search(query)
            },
            
            "create_bash" => {
                // Extract command parameter
                let command = match args["command"].as_str() {
                    Some(cmd) => cmd.to_string(),
                    None => return None,
                };
                
                // Log it
                let _ = cliclack::log::info(format!("Adding bash subtask: {}", command));
                
                 SubTaskType::Bash(command) 
            },
            
            // Support legacy function names for backward compatibility
            "create_read" => {
                let description = match args["description"].as_str() {
                    Some(desc) => desc.to_string(),
                    None => return None,
                };
                
                if description.starts_with('/') || description.starts_with("./") {
                    let _ = cliclack::log::info(format!("Adding legacy read file subtask: {}", description));
                    SubTaskType::Read(FilePathOrQuery::FilePath(description)) 
                } else {
                    let _ = cliclack::log::info(format!("Adding legacy read lookup subtask: {}", description));
                     SubTaskType::Read(FilePathOrQuery::FileQuery(description)) 
                }
            },
            
            "create_update" => {
                let description = match args["description"].as_str() {
                    Some(desc) => desc.to_string(),
                    None => return None,
                };
                
                let _ = cliclack::log::info(format!("Adding legacy update subtask: {}", description));
                
                if description.starts_with('/') || description.starts_with("./") {
                    SubTaskType::Update(FilePathOrQuery::FilePath(description))
                } else {
                     SubTaskType::Update(FilePathOrQuery::FileQuery(description))
                }
            },
            
            "create_search" => {
                let description = match args["description"].as_str() {
                    Some(desc) => desc.to_string(),
                    None => return None,
                };
                
                let _ = cliclack::log::info(format!("Adding legacy search subtask: {}", description));
                
                 SubTaskType::Search(description) 
            },
            
            _ => return  None ,
        };
        
        // Extract the SubTaskType from the SubTask
       // let subtask_type = subtask.task_type;


        return Some( subtask ) 








    }





}







#[derive(Debug, Clone)]
pub enum FilePathOrQuery {
    FilePath(String),
    FileQuery(String),
}

impl SubTaskType {
    pub fn description(&self) -> String {
        match self {
            SubTaskType::Task(desc) => format!("Task: {}", desc),
             
            SubTaskType::Bash(cmd) => format!("Execute: {}", cmd),
        }
    }
    
    pub fn icon(&self) -> &'static str {
        match self {
            SubTaskType::Task(_) => "🧠",
           
            SubTaskType::Bash(_) => "🖥️",
        }
    }


    pub fn requires_user_permission(&self) -> bool {
        match self {
            SubTaskType::Bash(_) => true,
            
            SubTaskType::FileEditTool(_) => true,

            _ => false 
        }
    }

}

/// Represents a subtask to be executed
pub struct SubTask {
    pub task_type: SubTaskType,
    pub metadata: Option<serde_json::Value>,
}

impl SubTask {
    pub fn new(task_type: SubTaskType, metadata: Option<serde_json::Value>) -> Self {
        Self { task_type, metadata }
    }













}


 pub struct TaskTool(String);  //query 
 
#[async_trait] 
impl SubtaskTool for TaskTool {
    async fn handle_subtask(&self, ai_client: &Box<dyn AiClient> , context_memory: Arc<Mutex<ContextMemory>> ) -> Option<SubtaskOutput> {
        let input = &self.0;

        let system_prompt = r#"
You are an expert AI assistant for a command-line tool that can help with various tasks.
Your job is to analyze user requests and determine what operations to perform.
Provide a list of the most appropriate operations to complete the input request, in the order in which they should be performed.
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
                "name": "read_file_at_path",
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
                "name": "read_file_from_lookup",
                "description": "Create a subtask to find and read a file matching a description",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "lookup_query": {
                            "type": "string",
                            "description": "Description of the file to find and read"
                        }
                    },
                    "required": ["lookup_query"]
                }
            },
            {
                "name": "update_file_at_path",
                "description": "Create a subtask to update a specific file by path",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Full path to the file to update"
                        }
                    },
                    "required": ["file_path"]
                }
            },
            {
                "name": "update_file_from_lookup",
                "description": "Create a subtask to find and update a file matching a description",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "lookup_query": {
                            "type": "string",
                            "description": "Description of the file to find and update"
                        }
                    },
                    "required": ["lookup_query"]
                }
            },
            {
                "name": "search_for_file",
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
        let response = match ai_client
            .chat_completion_with_functions(messages, functions)
            .await {
                Ok(resp) => resp,
                Err( e ) => {
                         println!("WARN no  chat_completion_with_functions {:?}", e );
                    return None
                },  
            };
        

        if let Some(content) = response.content {

             println!("{}",   content );


        }
        
        // Process function calls if any
        let Some(tool_calls) = response.tool_calls else {

            println!("WARN no fn call chosen ");
            return None;
        };



        let mut built_sub_tasks = Vec::new();


        for tool_call in &tool_calls {

            println!("got tool call {:?}", tool_call);


            if let Some( sub_task_type ) = SubTaskType::from_tool_call(tool_call.clone()){

                built_sub_tasks.push(sub_task_type);

            } 

        }


        if tool_calls.is_empty(){
            println!("wARN no tool calls ! ");
            return None 
        }

        
            //this pushes the new subtasks onto the queue 
        Some(SubtaskOutput::PushSubtasksIncrementDepth(  built_sub_tasks  ))
    }
}









pub struct BashTool(String);  //query 
 
#[ async_trait ] 
impl SubtaskTool for BashTool {
    async fn handle_subtask(&self, ai_client: &Box<dyn AiClient> , context_memory: Arc<Mutex<ContextMemory>> ) -> Option<SubtaskOutput > { 
        let command = &self.0;

        // Execute Bash command
        let _ = cliclack::log::info(format!("🔧 Executing command: {}", command));
        
        // Use tokio::process::Command to execute the command
        use tokio::process::Command;
        let output = match Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await {
                Ok(out) => out,
                Err(e) => {
                    let _ = cliclack::log::info(format!("Failed to execute command: {}", e));
                    return None;
                }
            };
        
        // Process the output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        // Print the results in a nice format
        if !stdout.is_empty() {
            println!();
            let _ = cliclack::log::info(format!("📄 Command output:"));
            
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
            let _ = cliclack::log::info(format!("⚠️ Command errors:"));
            println!("{}", stderr);
        }

        Some( SubtaskOutput::SubtaskComplete() )
    }
}






#[derive(Serialize,Deserialize,Clone,Debug)]
pub struct LSToolInputs  {
   file_path: String, 
}
pub struct LSTool( LSToolInputs ) ;




#[ async_trait ] 
impl SubtaskTool for LSTool {

   
    async fn handle_subtask(&self, ai_client: &Box<dyn AiClient> , context_memory: Arc<Mutex<ContextMemory>> ) -> Option<SubtaskOutput >  { 





            None


     }


}










#[derive(Serialize,Deserialize,Clone,Debug)]
pub struct GlobToolInputs  {
   file_path: String, 
}

pub struct GlobTool( GlobToolInputs );





#[ async_trait ] 
impl SubtaskTool for GlobTool {

   
    async fn handle_subtask(&self, ai_client: &Box<dyn AiClient> , context_memory: Arc<Mutex<ContextMemory>> ) -> Option<SubtaskOutput >  { 





            None


     }


}










#[derive(Serialize,Deserialize,Clone,Debug)]
pub struct GrepToolInputs  {
   file_path: String, 
}

pub struct GrepTool( GrepToolInputs ); 






#[ async_trait ] 
impl SubtaskTool for GrepTool {

 


    async fn handle_subtask(&self, ai_client: &Box<dyn AiClient> , context_memory: Arc<Mutex<ContextMemory>> ) -> Option<SubtaskOutput >  { 





            None


     }


}










#[derive(Serialize,Deserialize,Clone,Debug)]
pub struct FileReadToolInputs  {
   file_path: String, 
}



pub struct FileReadTool( FileReadToolInputs );  //query 
 
#[ async_trait ] 
impl SubtaskTool for FileReadTool {

 


    async fn handle_subtask(&self, ai_client: &Box<dyn AiClient> , context_memory: Arc<Mutex<ContextMemory>> ) -> Option<SubtaskOutput >  { 





            None


     }


}




#[derive(Serialize,Deserialize, Clone , Debug )]
pub struct FileEditToolInputs  {
   file_path: String, 
}


pub struct FileEditTool( FileEditToolInputs );   
 
#[ async_trait ] 
impl SubtaskTool for FileEditTool {

 


	async fn handle_subtask(&self, ai_client: &Box<dyn AiClient> , context_memory: Arc<Mutex<ContextMemory>> ) -> Option<SubtaskOutput >  { 








            None
	 }


}


 

// --------------
 
 


