pub mod bash_tool;
pub mod code_search_tool;
pub mod code_modification_tool;
pub mod documentation_tool;
pub mod task_tool;

use crate::error::AgentResult;
use crate::task::Task;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

pub use bash_tool::BashExecutionTool;
pub use code_search_tool::CodeSearchTool;
pub use code_modification_tool::CodeModificationTool;
pub use documentation_tool::DocumentationTool;
pub use task_tool::TaskProcessingTool;

/// Capability types for tools
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capability {
    ExecuteCommand,
    FileSystemAccess,
    ProcessManagement,
    CodeSearch,
    CodeModification,
    DocumentationGeneration,
    TaskProcessing,
}

/// Operation types for subtasks - defines the high-level operations that can be performed
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SubtaskOperation {
    /// The main task that processes user input and creates subtasks
    Task,
    /// Read a file or code snippet and add it to context
    Read,
    /// Write/update a file with new content
    Update,
    /// Search for files or code based on criteria
    Search,
    /// Execute a bash command
    Bash,
}

/// Arguments passed to a tool
#[derive(Debug, Clone)]
pub struct ToolArgs {
    pub command: String,
    pub parameters: HashMap<String, Value>,
    pub context: Arc<SharedContext>,
    pub task_queue: Option<Arc<Mutex<VecDeque<Task>>>>,
}

/// Output from a tool execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolOutput {
    pub success: bool,
    pub result: Value,
    pub message: Option<String>,
    pub artifacts: HashMap<String, Vec<u8>>,
}

/// Shared context between tool executions
#[derive(Debug)]
pub struct SharedContext {
    pub variables: HashMap<String, Value>,
    pub artifacts: HashMap<String, Vec<u8>>,
    pub history: Vec<ContextEvent>,
    pub metadata: HashMap<String, String>,
}

/// Event in the context history
#[derive(Debug, Clone)]
pub struct ContextEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: EventType,
    pub description: String,
    pub data: Option<Value>,
}

/// Type of context event
#[derive(Debug, Clone)]
pub enum EventType {
    ToolExecution,
    UserInput,
    SystemEvent,
    MemoryAccess,
}

/// Tool interface for all executable tools
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: &ToolArgs) -> AgentResult<ToolOutput>;
    fn capabilities(&self) -> Vec<Capability>;
}

impl SharedContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            artifacts: HashMap::new(),
            history: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn add_variable(&mut self, key: &str, value: Value) {
        self.variables.insert(key.to_string(), value);
    }

    pub fn get_variable(&self, key: &str) -> Option<&Value> {
        self.variables.get(key)
    }
    
    pub fn add_artifact(&mut self, key: &str, data: Vec<u8>) {
        self.artifacts.insert(key.to_string(), data);
    }

    pub fn add_event(&mut self, event_type: EventType, description: &str, data: Option<Value>) {
        let event = ContextEvent {
            timestamp: chrono::Utc::now(),
            event_type,
            description: description.to_string(),
            data,
        };
        self.history.push(event);
    }
}