use crate::error::{AgentError, AgentResult};
use crate::memory::{MemoryEntry, MemoryMetadata};
use crate::utils;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Maximum capacity for the short-term memory queue
const DEFAULT_CAPACITY: usize = 100;

/// Manages recent conversation history and context
pub struct ShortTermMemory {
    entries: Arc<Mutex<VecDeque<MemoryEntry>>>,
    capacity: usize,
}

impl ShortTermMemory {
    pub fn new(capacity: Option<usize>) -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(
                capacity.unwrap_or(DEFAULT_CAPACITY),
            ))),
            capacity: capacity.unwrap_or(DEFAULT_CAPACITY),
        }
    }

    /// Add an entry to short-term memory
    pub fn add(&self, content: &str, metadata: MemoryMetadata) -> AgentResult<String> {
        let id = utils::generate_id();
        let timestamp = utils::current_timestamp();

        let entry = MemoryEntry {
            id: id.clone(),
            content: content.to_string(),
            metadata,
            timestamp,
            embeddings: None,
        };

        let mut entries = self.entries.lock().map_err(|e| {
            AgentError::Storage(format!("Failed to lock short-term memory entries: {}", e))
        })?;

        // If we're at capacity, remove the oldest entry
        if entries.len() >= self.capacity {
            entries.pop_front();
        }

        entries.push_back(entry);
        Ok(id)
    }

    /// Get all entries in the short-term memory
    pub fn get_all(&self) -> AgentResult<Vec<MemoryEntry>> {
        let entries = self.entries.lock().map_err(|e| {
            AgentError::Storage(format!("Failed to lock short-term memory entries: {}", e))
        })?;

        Ok(entries.iter().cloned().collect())
    }

    /// Get a specific entry by ID
    pub fn get_by_id(&self, id: &str) -> AgentResult<Option<MemoryEntry>> {
        let entries = self.entries.lock().map_err(|e| {
            AgentError::Storage(format!("Failed to lock short-term memory entries: {}", e))
        })?;

        Ok(entries.iter().find(|e| e.id == id).cloned())
    }

    /// Simple keyword search in short-term memory
    pub fn search(&self, query: &str, limit: usize) -> AgentResult<Vec<MemoryEntry>> {
        let entries = self.entries.lock().map_err(|e| {
            AgentError::Storage(format!("Failed to lock short-term memory entries: {}", e))
        })?;

        let query_lowercase = query.to_lowercase();
        let results: Vec<MemoryEntry> = entries
            .iter()
            .filter(|e| e.content.to_lowercase().contains(&query_lowercase))
            .take(limit)
            .cloned()
            .collect();

        Ok(results)
    }

    /// Clear all entries from short-term memory
    pub fn clear(&self) -> AgentResult<()> {
        let mut entries = self.entries.lock().map_err(|e| {
            AgentError::Storage(format!("Failed to lock short-term memory entries: {}", e))
        })?;

        entries.clear();
        Ok(())
    }

    /// Get the most recent entries, up to the specified limit
    pub fn get_recent(&self, limit: usize) -> AgentResult<Vec<MemoryEntry>> {
        let entries = self.entries.lock().map_err(|e| {
            AgentError::Storage(format!("Failed to lock short-term memory entries: {}", e))
        })?;

        let results: Vec<MemoryEntry> = entries
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        Ok(results)
    }

    /// Get the current memory size
    pub fn size(&self) -> AgentResult<usize> {
        let entries = self.entries.lock().map_err(|e| {
            AgentError::Storage(format!("Failed to lock short-term memory entries: {}", e))
        })?;

        Ok(entries.len())
    }
}