

 
use cliclack::log::info;
use cliclack::input;
use crate::ai::MessageRole;
use crate::AiClient;
use serde_json::json;
use crate::ai::Message;
 
use crate::AgentError;
use std::fmt;
use std::sync::Arc;
use serde::Serialize;
use serde::Deserialize;
use async_trait::async_trait;
use crate::agent_engine::SubtaskOutput;

#[ async_trait ] 
pub trait SubtaskTool {


	  async fn handle_subtask(&self, ai_client: Arc<AiClient>) -> Option<SubtaskOutput > ; 


}


 

pub struct TaskTool(String);  //query 
 
#[ async_trait ] 
impl SubtaskTool for TaskTool {

 


		async fn handle_subtask(&self, ai_client: Arc<AiClient>) -> Option<SubtaskOutput>  { 






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
        let response = ai_client
            .chat_completion_with_functions(messages, functions)
            .await?;
        
        // Process function calls if any
        let  Some(function_call) = response.function_call else {

            return None 
        };



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
                        .ok_or_else(|| AgentError::AiApi("Missing description parameter".to_string())).ok()?
                        .to_string();
                    
                    // Log it
                    log::info(&format!("Adding task: {}", description)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Task(description), None)
                },
                
                "read_file_at_path" => {
                    // Extract file_path parameter
                    let file_path = args["file_path"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing file_path parameter".to_string())).ok()?
                        .to_string();
                    
                    // Log it
                    log::info(&format!("Adding read file subtask: {}", file_path)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Read(FilePathOrQuery::FilePath(file_path)), None)
                },
                
                "read_file_from_lookup" => {
                    // Extract lookup_query parameter
                    let lookup_query = args["lookup_query"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing lookup_query parameter".to_string())).ok()?
                        .to_string();
                    
                    // Log it
                    log::info(&format!("Adding read lookup subtask: {}", lookup_query)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Read(FilePathOrQuery::FileQuery(lookup_query)), None)
                },
                
                "update_file_at_path" => {
                    // Extract file_path parameter
                    let file_path = args["file_path"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing file_path parameter".to_string())).ok()?
                        .to_string();
                    
                    // Log it
                    log::info(&format!("Adding update file subtask: {}", file_path)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Update(FilePathOrQuery::FilePath(file_path)), None)
                },
                
                "update_file_from_lookup" => {
                    // Extract lookup_query parameter
                    let lookup_query = args["lookup_query"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing lookup_query parameter".to_string())).ok()?
                        .to_string();
                    
                    // Log it
                    log::info(&format!("Adding update lookup subtask: {}", lookup_query)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Update(FilePathOrQuery::FileQuery(lookup_query)), None)
                },
                
                "search_for_file" => {
                    // Extract query parameter
                    let query = args["query"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing query parameter".to_string())).ok()?
                        .to_string();
                    
                    // Log it
                    log::info(&format!("Adding search subtask: {}", query)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Search(query.to_string()), None)
                },
                
                "create_bash" => {
                    // Extract command parameter
                    let command = args["command"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing command parameter".to_string())).ok()?
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
                        SubTask::new(SubTaskType::Read(FilePathOrQuery::FilePath(description)), None)
                    } else {
                        log::info(&format!("Adding legacy read lookup subtask: {}", description)).expect("Failed to log");
                        SubTask::new(SubTaskType::Read(FilePathOrQuery::FileQuery(description)), None)
                    }
                },
                
                "create_update" => {
                    let description = args["description"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing description parameter".to_string()))?
                        .to_string();
                    
                    log::info(&format!("Adding legacy update subtask: {}", description)).expect("Failed to log");
                    
                    if description.starts_with('/') || description.starts_with("./") {
                        SubTask::new(SubTaskType::Update(FilePathOrQuery::FilePath(description)), None)
                    } else {
                        SubTask::new(SubTaskType::Update(FilePathOrQuery::FileQuery(description)), None)
                    }
                },
                
                "create_search" => {
                    let description = args["description"].as_str()
                        .ok_or_else(|| AgentError::AiApi("Missing description parameter".to_string()))?
                        .to_string();
                    
                    log::info(&format!("Adding legacy search subtask: {}", description)).expect("Failed to log");
                    
                    SubTask::new(SubTaskType::Search( description ), None)
                },
                
                _ => return Err(AgentError::AiApi(format!("Unknown function: {}", function_name))),
            };
            
            // Add the subtask to the queue
          //  self.add_queued_subtask(subtask);




            return SubtaskOutput::PushSubtasksIncrementDepth (  vec![ subtask  ]    )


  





		 }


}






pub struct ReadTool(String);  //query 
 
#[ async_trait ] 
impl SubtaskTool for ReadTool {

 


		async fn handle_subtask(&self, ai_client: Arc<AiClient>) -> Option<SubtaskOutput >  { 









		 }


}










pub struct BashTool(String);  //query 
 
#[ async_trait ] 
impl SubtaskTool for BashTool {

 


		async fn handle_subtask(&self, ai_client: Arc<AiClient>) -> Option<SubtaskOutput > { 









		 }


}





pub struct UpdateTool(String);  //query 
 
#[ async_trait ] 
impl SubtaskTool for UpdateTool {

 


		async fn handle_subtask(&self, ai_client: Arc<AiClient>) -> Option<SubtaskOutput >  { 









		 }


}





pub struct SearchTool(String);  //query 
 
#[ async_trait ] 
impl SubtaskTool for SearchTool {

 


		async fn handle_subtask(&self, ai_client: Arc<AiClient>) -> Option<SubtaskOutput >  { 









		 }


}


// --------------


/// Read subtask variants
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilePathOrQuery {
    /// Read a specific file path
    FilePath(String),
    /// Look for a file matching a description
    FileQuery(String),
}

impl fmt::Display for FilePathOrQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilePathOrQuery::FilePath(path) => write!(f, "{}", path),
            FilePathOrQuery::FileQuery(query) => write!(f, "query: {}", query),
        }
    }
}
 
/// The type of subtask
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubTaskType {
    Task(String),   // ???? 
    Read(FilePathOrQuery),
    Update(FilePathOrQuery),
    Search(String),
    Bash(String),
}

impl SubTaskType {
    pub fn description(&self) -> String {
        match self {
            SubTaskType::Task(desc) => desc.clone(),

            SubTaskType::Read(path_or_query) => format!("Read file: {}", path_or_query) ,
            SubTaskType::Update(path_or_query) => format!("Update file: {}", path_or_query) ,
            SubTaskType::Search(desc) => format!("Search file: {}", desc) ,
          
            SubTaskType::Bash(cmd) => cmd.clone(),
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            SubTaskType::Task(_) => "📋",
            SubTaskType::Read(_) => "👁️",
            SubTaskType::Update(_) => "✏️",
            SubTaskType::Search(_) => "🔎",
            SubTaskType::Bash(_) => "🔧",
        }
    }
}




impl SubTaskType {
    pub fn get_tool(&self) -> Arc<dyn SubtaskTool> {
        match self {
            Self::Task(query)  =>  Arc::new(TaskTool (query.to_string()) ),

             Self::Read(path_or_query)  =>  Arc::new(ReadTool ( path_or_query.to_string() )),


             Self::Bash( input ) => Arc::new( BashTool( input.to_string()  ) ),

             Self::Update(path_or_query) => Arc::new( UpdateTool( path_or_query.to_string() ) ),

             Self::Search( input)  => Arc::new(  SearchTool ( input.to_string()  )   )
            
         
            // Other cases should return their respective tool implementations



        //    _ => unimplemented!("Tool not implemented"),
        }
    }
}
