

use std::fmt;
use std::sync::Arc;
use serde::Serialize;
use serde::Deserialize;
use async_trait::async_trait;
use crate::agent_engine::SubtaskOutput;

#[ async_trait ] 
pub trait SubtaskTool {


	  async fn handle_subtask(&self) -> SubtaskOutput ; 


}


 

pub struct TaskTool(String);  //query 
 
#[ async_trait ] 
impl SubtaskTool for TaskTool {

 


		async fn handle_subtask(&self) -> SubtaskOutput  { 





		 }


}






pub struct ReadTool(String);  //query 
 
#[ async_trait ] 
impl SubtaskTool for ReadTool {

 


		async fn handle_subtask(&self) -> SubtaskOutput  { 









		 }


}










pub struct BashTool(String);  //query 
 
#[ async_trait ] 
impl SubtaskTool for BashTool {

 


		async fn handle_subtask(&self) -> SubtaskOutput  { 









		 }


}





pub struct UpdateTool(String);  //query 
 
#[ async_trait ] 
impl SubtaskTool for UpdateTool {

 


		async fn handle_subtask(&self) -> SubtaskOutput  { 









		 }


}





pub struct SearchTool(String);  //query 
 
#[ async_trait ] 
impl SubtaskTool for SearchTool {

 


		async fn handle_subtask(&self) -> SubtaskOutput  { 









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
