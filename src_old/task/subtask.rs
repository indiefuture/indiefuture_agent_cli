use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::ai::AiClient;
use crate::codebase::CodebaseScanner;
use crate::error::{AgentError, AgentResult};
use crate::memory::MemoryManager;






/// Status of a subtask
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

/// A subtask represents a unit of work to be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub parent_id: Option<String>,
    pub subtask_type: SubTaskType,
    pub status: TaskStatus,
    pub result: Option<String>,
}

impl SubTask {
    pub fn new(subtask_type: SubTaskType, parent_id: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id,
            subtask_type,
            status: TaskStatus::Pending,
            result: None,
        }
    }
}

/// A queue for managing subtasks
#[derive(Debug, Clone)]
pub struct SubTaskQueue {
    queue: Arc<Mutex<Vec<SubTask>>>,
}

impl SubTaskQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add(&self, subtask: SubTask) {
        let mut queue = self.queue.lock().unwrap();
        queue.push(subtask);
    }

    pub fn next(&self) -> Option<SubTask> {
        let mut queue = self.queue.lock().unwrap();
        queue.pop()
    }

    pub fn is_empty(&self) -> bool {
        let queue = self.queue.lock().unwrap();
        queue.is_empty()
    }

    pub fn peek(&self) -> Option<SubTask> {
        let queue = self.queue.lock().unwrap();
        queue.last().cloned()
    }

    pub fn len(&self) -> usize {
        let queue = self.queue.lock().unwrap();
        queue.len()
    }

    pub fn list(&self) -> Vec<SubTask> {
        let queue = self.queue.lock().unwrap();
        queue.clone()
    }
}

// For backward compatibility - these will be removed later

/// Legacy operation type - to be replaced by SubTaskType
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    TASK,
    READ,
    WRITE,
    SEARCH,
    BASH,
    UPDATE,    // Added for backward compatibility
    UNKNOWN,   // Added for backward compatibility
}

impl OperationType {
    pub fn icon(&self) -> &str {
        match self {
            OperationType::TASK => "📋",
            OperationType::READ => "👁️",
            OperationType::WRITE => "✏️",
            OperationType::SEARCH => "🔎",
            OperationType::BASH => "🔧",
            OperationType::UPDATE => "✏️",
            OperationType::UNKNOWN => "❓",
        }
    }
}

/// Legacy Task - to be replaced by SubTask 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub parent_id: Option<String>,
    pub description: String,
    pub operation_type: Option<OperationType>,
    pub status: TaskStatus,
    pub result: Option<String>,
}

impl Task {
    pub fn new(description: String, operation_type: Option<OperationType>, parent_id: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id,
            description,
            operation_type,
            status: TaskStatus::Pending,
            result: None,
        }
    }
}

/// Legacy TaskExecutor - to be replaced by new implementation
pub struct TaskExecutor {
    memory_manager: Arc<MemoryManager>,
    ai_client: Box<dyn AiClient>,
    codebase_scanner: Arc<CodebaseScanner>,
    workspace_root: PathBuf,
    max_concurrent_tasks: usize,
    tasks: HashMap<String, Task>,
    status_callback: Option<Box<dyn Fn(&Task) + Send + Sync>>,
    user_confirmation_callback: Option<Box<dyn Fn(&str, &str) -> bool + Send + Sync>>,
}

impl TaskExecutor {
    pub fn new(
        memory_manager: Arc<MemoryManager>,
        ai_client: Box<dyn AiClient>,
        codebase_scanner: Arc<CodebaseScanner>,
        workspace_root: PathBuf,
        max_concurrent_tasks: usize,
    ) -> Self {
        Self {
            memory_manager,
            ai_client,
            codebase_scanner,
            workspace_root,
            max_concurrent_tasks,
            tasks: HashMap::new(),
            status_callback: None,
            user_confirmation_callback: None,
        }
    }

    pub fn set_status_callback(&mut self, callback: Box<dyn Fn(&Task) + Send + Sync>) {
        self.status_callback = Some(callback);
    }

    pub fn set_user_confirmation_callback(
        &mut self,
        callback: Box<dyn Fn(&str, &str) -> bool + Send + Sync>,
    ) {
        self.user_confirmation_callback = Some(callback);
    }

    pub fn queue_tasks(&mut self, tasks: Vec<Task>) -> AgentResult<()> {
        for task in tasks {
            self.tasks.insert(task.id.clone(), task);
        }
        Ok(())
    }

    pub async fn execute_tasks(&mut self) -> AgentResult<HashMap<String, Task>> {
        // Implementation omitted as this is legacy code
        // The new implementation will use SubTaskQueue
        Ok(self.tasks.clone())
    }
} 