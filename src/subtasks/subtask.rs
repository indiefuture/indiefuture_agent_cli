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
    
    ExplainTool(String), // Takes a string query to explain using accumulated context
}



impl SubTaskType {
    pub fn get_tool(&self) -> Arc<dyn SubtaskTool> {
        match self {
            Self::Task(query) => Arc::new(TaskTool(query.to_string())),

            Self::FileReadTool(input) => Arc::new(FileReadTool(input.clone())),

            Self::FileEditTool(input) => Arc::new(FileEditTool(input.clone())),

            Self::Bash(input) => Arc::new(BashTool(input.to_string())),

            Self::LSTool(input) => Arc::new(LSTool(input.clone())),

            Self::GlobTool(input) => Arc::new(GlobTool { inputs: input.clone() }),
            
            Self::GrepTool(input) => Arc::new(GrepTool(input.clone())),
            
            Self::ExplainTool(query) => Arc::new(ExplainTool(query.to_string())),
        }
    }
}







impl SubTaskType {




    pub fn from_tool_call(tool_call: GptToolCall) -> Option<Self> {
        let function_name = tool_call.function.name.as_str();
        let args: serde_json::Value = serde_json::from_str(tool_call.function.arguments.as_str()?).unwrap_or_default();
            
        // Log that we got a function call
        let _ = cliclack::log::info(format!("Processing function: {} {:?}", function_name, args));
            
        // Create the appropriate subtask based on the function call
        let subtask = match function_name {
            "Task" => {
                let description = match args["description"].as_str() {
                    Some(desc) => desc.to_string(),
                    None => return None,
                };
                
                let _ = cliclack::log::info(format!("Adding task: {}", description));
                SubTaskType::Task(description) 
            },
            
            "Bash" => {
                let command = match args["command"].as_str() {
                    Some(cmd) => cmd.to_string(),
                    None => return None,
                };
                
                let _ = cliclack::log::info(format!("Adding bash subtask: {}", command));
                SubTaskType::Bash(command) 
            },
            
            "FileReadTool" => {
                let file_path = match args["file_path"].as_str() {
                    Some(path) => path.to_string(),
                    None => return None,
                };
                
                let limit = args["limit"].as_u64().map(|v| v as u32);
                let offset = args["offset"].as_u64().map(|v| v as u32);
                
                let _ = cliclack::log::info(format!("Adding file read subtask: {}", file_path));
                
                SubTaskType::FileReadTool(FileReadToolInputs {
                    file_path,
                    limit,
                    offset
                })
            },
            
            "FileEditTool" => {
                let file_path = match args["file_path"].as_str() {
                    Some(path) => path.to_string(),
                    None => return None,
                };
                
                let old_string = match args["old_string"].as_str() {
                    Some(s) => s.to_string(),
                    None => return None,
                };
                
                let new_string = match args["new_string"].as_str() {
                    Some(s) => s.to_string(),
                    None => return None,
                };
                
                let _ = cliclack::log::info(format!("Adding file edit subtask: {}", file_path));
                
                SubTaskType::FileEditTool(FileEditToolInputs {
                    file_path,
                    old_string,
                    new_string
                })
            },
            
            "LSTool" => {
                let file_path = match args["path"].as_str() {
                    Some(path) => path.to_string(),
                    None => return None,
                };
                
                let _ = cliclack::log::info(format!("Adding LS subtask: {}", file_path));
                
                SubTaskType::LSTool(LSToolInputs {
                    file_path
                })
            },
            
            "GlobTool" => {
                let pattern = match args["pattern"].as_str() {
                    Some(p) => p.to_string(),
                    None => return None,
                };
                
                let path = args["path"].as_str().map(|s| s.to_string());
                
                let _ = cliclack::log::info(format!("Adding glob search subtask: {}", pattern));
                
                SubTaskType::GlobTool(GlobToolInputs {
                    pattern,
                    path
                })
            },
            
            "GrepTool" => {
                let pattern = match args["pattern"].as_str() {
                    Some(p) => p.to_string(),
                    None => return None,
                };
                
                let include = args["include"].as_str().map(|s| s.to_string());
                let path = args["path"].as_str().map(|s| s.to_string());
                
                let _ = cliclack::log::info(format!("Adding grep search subtask: {}", pattern));
                
                SubTaskType::GrepTool(GrepToolInputs {
                    pattern,
                    include,
                    path
                })
            },
            
            "Explain" => {
                let query = match args["query"].as_str() {
                    Some(q) => q.to_string(),
                    None =>   "".into(),
                };
                
                let _ = cliclack::log::info(format!("Adding explain subtask: {}", query));
                
                SubTaskType::ExplainTool(query)
            },
            
            
            
            "View" | "ReadFile"   => {
                let file_path = match args["file_path"].as_str() {
                    Some(path) => path.to_string(),
                    None => return None,
                };
                
                let limit = args["limit"].as_u64().map(|v| v as u32);
                let offset = args["offset"].as_u64().map(|v| v as u32);
                
                let _ = cliclack::log::info(format!("Legacy function: {} -> FileReadTool: {}", function_name, file_path));
                
                SubTaskType::FileReadTool(FileReadToolInputs {
                    file_path,
                    limit,
                    offset
                })
            },
            
            "Edit" => {
                let file_path = match args["file_path"].as_str() {
                    Some(path) => path.to_string(),
                    None => return None,
                };
                
                let old_string = match args["old_string"].as_str() {
                    Some(s) => s.to_string(),
                    None => return None,
                };
                
                let new_string = match args["new_string"].as_str() {
                    Some(s) => s.to_string(),
                    None => return None,
                };
                
                let _ = cliclack::log::info(format!("Legacy function: Edit -> FileEditTool: {}", file_path));
                
                SubTaskType::FileEditTool(FileEditToolInputs {
                    file_path,
                    old_string,
                    new_string
                })
            },
            
            "LS" => {
                let file_path = match args["path"].as_str() {
                    Some(path) => path.to_string(),
                    None => return None,
                };
                
                let _ = cliclack::log::info(format!("Legacy function: LS -> LSTool: {}", file_path));
                
                SubTaskType::LSTool(LSToolInputs {
                    file_path
                })
            },
            
         
            
            _ => {
                let _ = cliclack::log::info(format!("Unknown function: {}", function_name));
                return None;
            },
        };
        
        return Some(subtask);
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
            SubTaskType::FileReadTool(inputs) => format!("Read File: {}", inputs.file_path),
            SubTaskType::FileEditTool(inputs) => format!("Edit File: {}", inputs.file_path),
            SubTaskType::LSTool(inputs) => format!("List Directory: {}", inputs.file_path),
            SubTaskType::GlobTool(inputs) => format!("Glob Search: {}", inputs.pattern),
            SubTaskType::GrepTool(inputs) => format!("Grep Search: {}", inputs.pattern),
            SubTaskType::ExplainTool(query) => format!("Explain: {}", query),
        }
    }
    
    pub fn icon(&self) -> &'static str {
        match self {
            SubTaskType::Task(_) => "🧠",
            SubTaskType::Bash(_) => "🖥️",
            SubTaskType::FileReadTool(_) => "📄",
            SubTaskType::FileEditTool(_) => "✏️",
            SubTaskType::LSTool(_) => "📁",
            SubTaskType::GlobTool(_) => "🔍",
            SubTaskType::GrepTool(_) => "🔎",
            SubTaskType::ExplainTool(_) => "💡",
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
    async fn handle_subtask(&self, ai_client: &Box<dyn AiClient>, _context_memory: Arc<Mutex<ContextMemory>>) -> Option<SubtaskOutput> {
        let input = &self.0;

        let system_prompt = r#"
You are an expert AI assistant for a command-line tool that helps with software development tasks of a local codebase.
Your job is to analyze user requests and determine what operations the command-line tool should  perform.

IMPORTANT: You MUST recommend multiple tools (at least 2-3) to complete the task in a step-by-step fashion. 
Break down complex tasks into a sequence of simpler operations using different tools in the optimal order.

SUGGESTED WORKFLOW PATTERN:
1. Use search tools (GlobTool, GrepTool) to find relevant files
2. Use FileReadTool to examine the contents of those files
3. Use ExplainTool as a final step to provide an explanation using all the gathered context

For example, if asked to "explain the logging system in this codebase", you should recommend:
1. First use GlobTool to find all source files
2. Then use GrepTool to search for "log" or "logger" patterns in those files
3. Use FileReadTool to examine the most relevant files in detail
4. Finally use ExplainTool to provide a comprehensive explanation based on all gathered information


Remember to ALWAYS conclude with ExplainTool to provide a comprehensive answer based on all gathered information.
"#;

        // Load function schema from JSON file
        let functions_json = include_str!("schemas/task_functions.json");
        let functions: serde_json::Value = serde_json::from_str(functions_json)
            .expect("Failed to parse task_functions.json");

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
        let message_only_response = match ai_client
            .chat_completion_with_functions(messages, functions.clone(), true )
            .await {
                Ok(resp) => resp,
                Err(e) => {
                    println!("WARN: Failed to get chat completion: {:?}", e);
                    return None;
                },  
            };



        let Some(first_content) = message_only_response.content  else {


            return None ; 
        };


          let _ = cliclack::log::info(format!(" My plan : {}", first_content));



         let secondary_input_messages = vec![
             Message {
                role: MessageRole::System,
                content: system_prompt.to_string(),
                name: None,
            },
             Message {
                role: MessageRole::User,
                content: first_content.to_string(),
                name: None,
            },
         ];   

         println!("secondary input messages {:?}", secondary_input_messages);

        let secondary_response = match ai_client
            .chat_completion_with_functions( secondary_input_messages ,  functions.clone(), false )
            .await {
                Ok(resp) => resp,
                Err(e) => {
                    println!("WARN: Failed to get chat completion: {:?}", e);
                    return None;
                },  
            };


        
        // Display AI response text if any
        if let Some(content) = secondary_response.content {
            println!("{}", content);
        }
        
        // Process function calls
        let Some(tool_calls) = secondary_response.tool_calls else {
            println!("WARN: No tool calls chosen by AI");
            return None;
        };

        if tool_calls.is_empty() {
            println!("WARN: Empty tool calls list");
            return None;
        }

        // Convert tool calls to subtasks
        let mut built_sub_tasks = Vec::new();
        for tool_call in &tool_calls {
            println!("Processing tool call: {:?}", tool_call);

            if let Some(sub_task_type) = SubTaskType::from_tool_call(tool_call.clone()) {
                built_sub_tasks.push(sub_task_type);
            }
        }

        // Check if we have multiple subtasks and need to process them in sequence
        if built_sub_tasks.len() > 1 {
            println!("✅ Received multiple tool calls: {} tools", built_sub_tasks.len());
            // Push subtasks to the queue with depth increment for proper execution flow
            Some(SubtaskOutput::PushSubtasks(built_sub_tasks))
        } else if built_sub_tasks.len() == 1 {
            // Just a single subtask - use the simpler form
            Some(SubtaskOutput::PushSubtasks(built_sub_tasks))
        } else {
            println!("WARN: No valid subtasks created from tool calls");
            None
        }
    }
}









pub struct BashTool(String);  //query 
 
#[ async_trait ] 
impl SubtaskTool for BashTool {
    async fn handle_subtask(&self, _ai_client: &Box<dyn AiClient>, _context_memory: Arc<Mutex<ContextMemory>> ) -> Option<SubtaskOutput > { 
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
   pub file_path: String, 
}
pub struct LSTool( LSToolInputs ) ;




#[ async_trait ] 
impl SubtaskTool for LSTool {
    async fn handle_subtask(&self, _ai_client: &Box<dyn AiClient>, _context_memory: Arc<Mutex<ContextMemory>> ) -> Option<SubtaskOutput >  { 




            None


     }


}










#[derive(Serialize,Deserialize,Clone,Debug)]
pub struct GlobToolInputs  {
   pub pattern: String,
   pub path: Option<String>
}

pub struct GlobTool {
    pub inputs: GlobToolInputs
}





#[ async_trait ] 
impl SubtaskTool for GlobTool {
    async fn handle_subtask(&self, _ai_client: &Box<dyn AiClient>, context_memory: Arc<Mutex<ContextMemory>>) -> Option<SubtaskOutput> { 
        use crate::memory::{MemoryFragment, MemoryMetadata};
        use chrono::Utc;
        use glob::glob;
        use std::path::PathBuf;
        use std::fs;
        
        // Get the pattern and base path
        let pattern = &self.inputs.pattern;
        let base_path = match &self.inputs.path {
            Some(path) => PathBuf::from(path),
            None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        };
        
        // Construct the full pattern with the base path
        let search_pattern = if base_path.to_string_lossy().ends_with('/') {
            format!("{}{}", base_path.display(), pattern)
        } else {
            format!("{}/{}", base_path.display(), pattern)
        };
        
        // Log the search
        println!("🔍 Searching for files with pattern: {}", search_pattern);
        
        // Collect results and detailed path information
        let mut results = Vec::new();
        let mut detailed_results = Vec::new();
        
        // Use the glob crate to perform the pattern matching
        match glob(&search_pattern) {
            Ok(paths) => {
                for entry in paths {
                    match entry {
                        Ok(path) => {
                            // Get additional file metadata when possible
                            let metadata = fs::metadata(&path).ok();
                            let is_dir = metadata.as_ref().map_or(false, |m| m.is_dir());
                            let size = metadata.as_ref().map_or(0, |m| m.len());
                            let path_str = path.to_string_lossy().to_string();
                            
                            // Format path with type indicator for display
                            let type_indicator = if is_dir { "📁" } else { "📄" };
                            let size_str = if !is_dir { format!(" ({} bytes)", size) } else { String::new() };
                            let formatted_path = format!("{} {}{}", type_indicator, path_str, size_str);
                            
                            results.push(formatted_path);
                            
                            // Add detailed information for memory
                            detailed_results.push((
                                path_str.clone(),
                                if is_dir { "directory".to_string() } else { "file".to_string() },
                                size
                            ));
                        },
                        Err(e) => {
                            println!("⚠️ Error: {:?}", e);
                        }
                    }
                }
            },
            Err(e) => {
                println!("⚠️ Invalid glob pattern: {:?}", e);
                return None;
            }
        }
        
        // Sort results (directories first, then files)
        let mut sorted_indexes = (0..results.len()).collect::<Vec<_>>();
        sorted_indexes.sort_by(|&a, &b| {
            let a_is_dir = results[a].starts_with("📁");
            let b_is_dir = results[b].starts_with("📁");
            
            if a_is_dir && !b_is_dir {
                std::cmp::Ordering::Less
            } else if !a_is_dir && b_is_dir {
                std::cmp::Ordering::Greater
            } else {
                results[a].cmp(&results[b])
            }
        });
        
        // Reorder both results based on the sorted indexes
        let sorted_results: Vec<String> = sorted_indexes.iter().map(|&i| results[i].clone()).collect();
        let sorted_detailed: Vec<(String, String, u64)> = sorted_indexes.iter().map(|&i| detailed_results[i].clone()).collect();
        
        // Format the output for display
        let output = if sorted_results.is_empty() {
            "No files found matching the pattern.".to_string()
        } else {
            format!("Found {} file(s):\n{}", sorted_results.len(), sorted_results.join("\n"))
        };
        
        // Log the results
        println!("{}", output);
        
        // Create a memory fragment from the results
        let memory_fragment = if !sorted_detailed.is_empty() {
            // Format metadata for memory
            let mut mem_content = format!("Results for glob pattern: {}\n\n", search_pattern);
            
            for (path, file_type, size) in &sorted_detailed {
                if file_type == "directory" {
                    mem_content.push_str(&format!("- Directory: {}\n", path));
                } else {
                    mem_content.push_str(&format!("- File: {} ({} bytes)\n", path, size));
                }
            }
            
            // Create memory metadata
            let memory_metadata = MemoryMetadata {
                file_type: Some("search_results".to_string()),
                path: Some(base_path.to_string_lossy().to_string()),
                timestamp: Some(Utc::now().timestamp()),
                tags: vec![
                    "glob_search".to_string(),
                    format!("pattern:{}", pattern)
                ],
            };
            
            MemoryFragment {
                source: "glob_search".to_string(),
                content: mem_content,
                metadata: Some(memory_metadata),
            }
        } else {
            // Create a memory fragment for empty results
            MemoryFragment {
                source: "glob_search".to_string(),
                content: format!("No files found matching glob pattern: {}", search_pattern),
                metadata: Some(MemoryMetadata {
                    file_type: Some("search_results".to_string()),
                    path: Some(base_path.to_string_lossy().to_string()),
                    timestamp: Some(Utc::now().timestamp()),
                    tags: vec![
                        "glob_search".to_string(),
                        format!("pattern:{}", pattern),
                        "empty_results".to_string()
                    ],
                }),
            }
        };
        
        // Add the memory fragment to context memory
        {
            let mut memory = context_memory.lock().await;
            memory.add_frag(memory_fragment.clone());
        }
        
        // Return the memory fragment as output
        Some(SubtaskOutput::AddToContextMemory(memory_fragment))
    }
}










#[derive(Serialize,Deserialize,Clone,Debug)]
pub struct GrepToolInputs  {
   pub pattern: String,
   pub include: Option<String>,
   pub path: Option<String>
}

pub struct GrepTool( GrepToolInputs ); 






#[ async_trait ] 
impl SubtaskTool for GrepTool {
    async fn handle_subtask(&self, _ai_client: &Box<dyn AiClient>, _context_memory: Arc<Mutex<ContextMemory>> ) -> Option<SubtaskOutput >  { 





            None


     }


}









#[derive(Serialize,Deserialize,Clone,Debug)]
pub struct FileReadToolInputs  {
   pub file_path: String,
   pub limit: Option<u32>,
   pub offset: Option<u32>
}



pub struct FileReadTool( FileReadToolInputs );  //query 
 
#[ async_trait ] 
impl SubtaskTool for FileReadTool {
    async fn handle_subtask(&self, _ai_client: &Box<dyn AiClient>, _context_memory: Arc<Mutex<ContextMemory>> ) -> Option<SubtaskOutput >  { 





            None


     }


}



#[derive(Serialize,Deserialize, Clone , Debug )]
pub struct FileEditToolInputs  {
   pub file_path: String,
   pub old_string: String,
   pub new_string: String
}


pub struct FileEditTool( FileEditToolInputs );   
 
#[ async_trait ] 
impl SubtaskTool for FileEditTool {
	async fn handle_subtask(&self, _ai_client: &Box<dyn AiClient>, _context_memory: Arc<Mutex<ContextMemory>> ) -> Option<SubtaskOutput >  { 








            None
	 }


}


 

// --------------

pub struct ExplainTool(String);  // Query string

#[async_trait]
impl SubtaskTool for ExplainTool {
    async fn handle_subtask(&self, ai_client: &Box<dyn AiClient>, context_memory: Arc<Mutex<ContextMemory>>) -> Option<SubtaskOutput> {
        use colored::Colorize;
        use std::time::Instant;
        
        // Get the query to explain
        let query = &self.0;
        
        println!("\n{} {}\n", "💡".bold(), "Generating explanation based on context...".cyan().bold());
        
        // Create a system prompt that instructs the AI to use the context
        let system_prompt = r#"
You are an expert AI assistant explaining a software codebase with context.
Use the provided context to thoroughly answer the user's question.
Focus only on the information present in the context and the specific query.
If the context doesn't contain relevant information, acknowledge the limitations 
of what you can explain based on the available context.

Format your response clearly using markdown when appropriate:
- Use bullet points for lists
- Use code blocks for code examples or file paths
- Use headings to organize longer responses
"#;

        // Collect all context data from memory fragments
        let context_data = {
            let memory = context_memory.lock().await;
            let fragments = memory.get_fragments();
            
            if fragments.is_empty() {
                "No context information has been collected yet.".to_string()
            } else {
                // Format all fragments as context
                let mut context_str = format!("Local {} relevant context of information:\n\n", fragments.len());
                
                for (i, fragment) in fragments.iter().enumerate() {
                    context_str.push_str(&format!("=== CONTEXT ITEM {} (from {}) ===\n", i+1, fragment.source));
                    context_str.push_str(&fragment.content);
                    context_str.push_str("\n\n");
                    
                    // Add metadata if present
                    if let Some(meta) = &fragment.metadata {
                        if let Some(path) = &meta.path {
                            context_str.push_str(&format!("Path: {}\n", path));
                        }
                        if !meta.tags.is_empty() {
                            context_str.push_str(&format!("Tags: {}\n", meta.tags.join(", ")));
                        }
                        context_str.push_str("\n");
                    }
                }
                
                context_str
            }
        };
        
        // Build messages with context and query
        let messages = vec![
            crate::ai::Message {
                role: crate::ai::MessageRole::System,
                content: system_prompt.to_string(),
                name: None,
            },
            crate::ai::Message {
                role: crate::ai::MessageRole::User,
                content: format!("Here is the context information I've gathered:\n\n{}\n\nBased on this context, please explain: {}", context_data, query),
                name: None,
            },
        ];
        
        // Start timer for measuring explanation generation time
        let start_time = Instant::now();
        
        // Generate explanation using the AI client
        let explanation = match ai_client.generate_text(messages).await {
            Ok(text) => text,
            Err(e) => {
                println!("{} {}: {}", "❌".red().bold(), "Error generating explanation".red().bold(), e);
                return Some(SubtaskOutput::SubtaskComplete());
            }
        };
        
        // Calculate and format elapsed time
        let elapsed = start_time.elapsed();
        let elapsed_secs = elapsed.as_secs_f32();
        let timing_msg = format!("Generated in {:.2}s", elapsed_secs);
        
        // Print the generated explanation
        println!("\n{}\n", "=".repeat(80).cyan());
        println!("{}\n", explanation);
        println!("{}\n", "=".repeat(80).cyan());
        println!("{}\n", timing_msg.dimmed());
        
        // Return complete signal
        Some(SubtaskOutput::SubtaskComplete())
    }
}