use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use serde_json::Value;

use crate::error::AgentResult;
use crate::utils;

/// Represents contextual data for a specific task or subtask
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingContext {
    pub id: String,
    pub task_id: String,
    pub parent_id: Option<String>,
    pub variables: HashMap<String, Value>,
    pub created_at: String,
    pub updated_at: String,
}

impl WorkingContext {
    pub fn new(task_id: &str, parent_id: Option<&str>) -> Self {
        let now = utils::current_timestamp();
        Self {
            id: utils::generate_id(),
            task_id: task_id.to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            variables: HashMap::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
    
    pub fn set_variable(&mut self, key: &str, value: Value) {
        self.variables.insert(key.to_string(), value);
        self.updated_at = utils::current_timestamp();
    }
    
    pub fn get_variable(&self, key: &str) -> Option<&Value> {
        self.variables.get(key)
    }
    
    pub fn merge(&mut self, other: &WorkingContext) {
        for (key, value) in &other.variables {
            self.variables.insert(key.clone(), value.clone());
        }
        self.updated_at = utils::current_timestamp();
    }
}

/// Manages working context for all tasks
pub struct WorkingContextManager {
    contexts: Mutex<HashMap<String, WorkingContext>>,
}

impl WorkingContextManager {
    pub fn new() -> Self {
        Self {
            contexts: Mutex::new(HashMap::new()),
        }
    }
    
    /// Create a new context for a task
    pub fn create_context(&self, task_id: &str, parent_id: Option<&str>) -> AgentResult<WorkingContext> {
        let context = WorkingContext::new(task_id, parent_id);
        let context_id = context.id.clone();
        
        let mut contexts = self.contexts.lock().unwrap();
        contexts.insert(context_id, context.clone());
        
        Ok(context)
    }
    
    /// Get a context by ID
    pub fn get_context(&self, context_id: &str) -> Option<WorkingContext> {
        let contexts = self.contexts.lock().unwrap();
        contexts.get(context_id).cloned()
    }
    
    /// Get a context by task ID
    pub fn get_context_for_task(&self, task_id: &str) -> Option<WorkingContext> {
        let contexts = self.contexts.lock().unwrap();
        contexts.values().find(|c| c.task_id == task_id).cloned()
    }
    
    /// Update a context
    pub fn update_context(&self, context: WorkingContext) -> AgentResult<()> {
        let mut contexts = self.contexts.lock().unwrap();
        contexts.insert(context.id.clone(), context);
        Ok(())
    }
    
    /// Delete a context
    pub fn delete_context(&self, context_id: &str) -> AgentResult<()> {
        let mut contexts = self.contexts.lock().unwrap();
        contexts.remove(context_id);
        Ok(())
    }
    
    /// Get all contexts
    pub fn get_all_contexts(&self) -> Vec<WorkingContext> {
        let contexts = self.contexts.lock().unwrap();
        contexts.values().cloned().collect()
    }
}