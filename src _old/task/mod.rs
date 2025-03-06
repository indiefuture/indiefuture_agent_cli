pub mod decomposition;
pub mod executor;

use crate::error::AgentResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

/// Re-export the main types and functions
pub use decomposition::TaskDecomposer;
pub use executor::TaskExecutor;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "Pending"),
            TaskStatus::InProgress => write!(f, "In Progress"),
            TaskStatus::Completed => write!(f, "Completed"),
            TaskStatus::Failed => write!(f, "Failed"),
            TaskStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    TASK,    // Main task operation
    READ,    // Read a file operation
    SEARCH,  // Search for files operation
    UPDATE,  // Update a file operation
    BASH,    // Execute a bash command
    UNKNOWN, // Default if not specified
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::TASK => write!(f, "TASK"),
            OperationType::READ => write!(f, "READ"),
            OperationType::SEARCH => write!(f, "SEARCH"),
            OperationType::UPDATE => write!(f, "UPDATE"),
            OperationType::BASH => write!(f, "BASH"),
            OperationType::UNKNOWN => write!(f, "UNKNOWN"),
        }
    }
}

// Get icon for operation type
impl OperationType {
    pub fn icon(&self) -> &str {
        match self {
            OperationType::TASK => "🔄",
            OperationType::READ => "📖",
            OperationType::SEARCH => "🔍",
            OperationType::UPDATE => "✏️",
            OperationType::BASH => "💻",
            OperationType::UNKNOWN => "➡️",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub parent_id: Option<String>,
    pub description: String,
    pub status: TaskStatus,
    pub dependencies: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub timeout: Duration,
    pub priority: u8,
    pub metadata: HashMap<String, String>,
    pub result: Option<String>,
    pub error: Option<String>,
    #[serde(default)]
    pub operation_type: Option<OperationType>,
}

impl Task {
    pub fn new(
        id: String,
        description: String,
        parent_id: Option<String>,
        dependencies: Vec<String>,
        timeout: Duration,
        priority: u8,
    ) -> Self {
        let now = crate::utils::current_timestamp();
        
        Self {
            id,
            parent_id,
            description,
            status: TaskStatus::Pending,
            dependencies,
            created_at: now.clone(),
            updated_at: now,
            completed_at: None,
            timeout,
            priority,
            metadata: HashMap::new(),
            result: None,
            error: None,
            operation_type: None,
        }
    }
    
    pub fn with_operation(mut self, operation_type: OperationType) -> Self {
        self.operation_type = Some(operation_type);
        self
    }
    
    pub fn mark_in_progress(&mut self) {
        self.status = TaskStatus::InProgress;
        self.updated_at = crate::utils::current_timestamp();
    }
    
    pub fn mark_completed(&mut self, result: Option<String>) {
        self.status = TaskStatus::Completed;
        self.result = result;
        let now = crate::utils::current_timestamp();
        self.updated_at = now.clone();
        self.completed_at = Some(now);
    }
    
    pub fn mark_failed(&mut self, error: String) {
        self.status = TaskStatus::Failed;
        self.error = Some(error);
        self.updated_at = crate::utils::current_timestamp();
    }
    
    pub fn mark_cancelled(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.updated_at = crate::utils::current_timestamp();
    }
    
    pub fn add_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }
    
    pub fn is_executable(&self, completed_tasks: &[String]) -> bool {
        if self.status != TaskStatus::Pending {
            return false;
        }
        
        // Check if all dependencies are completed
        self.dependencies
            .iter()
            .all(|dep_id| completed_tasks.contains(dep_id))
    }
}