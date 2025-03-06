pub mod short_term;
pub mod vector_store;
pub mod working_context;

use crate::error::AgentResult;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Re-export the main types
pub use short_term::ShortTermMemory;
pub use vector_store::VectorStore;
pub use working_context::{WorkingContext, WorkingContextManager};

/// Manages both short-term and long-term memory systems
pub struct MemoryManager {
    pub short_term: Arc<ShortTermMemory>,
    pub vector_store: Arc<VectorStore>,
    pub working_context: Arc<WorkingContextManager>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub metadata: MemoryMetadata,
    pub timestamp: String,
    pub embeddings: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub source: String,
    pub file_path: Option<String>,
    pub language: Option<String>,
    pub tags: Vec<String>,
    pub task_id: Option<String>,
}

impl MemoryManager {
    pub fn new(short_term: ShortTermMemory, vector_store: VectorStore) -> Self {
        Self {
            short_term: Arc::new(short_term),
            vector_store: Arc::new(vector_store),
            working_context: Arc::new(WorkingContextManager::new()),
        }
    }
    
    /// Add content to both short-term and long-term memory
    pub async fn add_memory(&self, content: &str, metadata: MemoryMetadata) -> AgentResult<String> {
        // Add to short-term memory first
        let entry_id = self.short_term.add(content, metadata.clone())?;
        
        // Add to vector store if embeddings are enabled
        if self.vector_store.is_available() {
            let _ = self.vector_store.add(entry_id.clone(), content, metadata).await?;
        }
        
        Ok(entry_id)
    }
    
    /// Retrieve relevant memories based on a query
    pub async fn retrieve_relevant(&self, query: &str, limit: usize) -> AgentResult<Vec<MemoryEntry>> {
        let mut results = Vec::new();
        
        // Always check short-term memory first
        let short_term_results = self.short_term.search(query, limit)?;
        results.extend(short_term_results);
        
        // If we need more results and vector store is available, query it too
        if results.len() < limit && self.vector_store.is_available() {
            let vector_results = self.vector_store.search(query, limit - results.len()).await?;
            results.extend(vector_results);
        }
        
        Ok(results)
    }
    
    /// Clear short-term memory but keep long-term
    pub fn clear_short_term(&self) -> AgentResult<()> {
        self.short_term.clear()
    }
    
    /// Create a working context for a task
    pub fn create_working_context(&self, task_id: &str, parent_id: Option<&str>) -> AgentResult<WorkingContext> {
        self.working_context.create_context(task_id, parent_id)
    }
    
    /// Get a working context for a task
    pub fn get_working_context(&self, task_id: &str) -> Option<WorkingContext> {
        self.working_context.get_context_for_task(task_id)
    }
    
    /// Update a working context
    pub fn update_working_context(&self, context: WorkingContext) -> AgentResult<()> {
        self.working_context.update_context(context)
    }
}