use crate::error::{AgentError, AgentResult};
use crate::memory::{MemoryEntry, MemoryMetadata};
use crate::utils;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use uuid::Uuid;

const EMBEDDING_SIZE: usize = 1536; // Default for OpenAI embeddings

/// A simple in-memory vector database item
#[derive(Clone, Serialize, Deserialize)]
struct VectorItem {
    id: String,
    content: String,
    metadata: MemoryMetadata,
    timestamp: String,
    embedding: Vec<f32>,
}

/// Manages vector embeddings and semantic search
pub struct VectorStore {
    items: Arc<RwLock<Vec<VectorItem>>>,
    persistence_path: Option<PathBuf>,
    is_available: bool,
}

impl VectorStore {
    pub async fn new(data_dir: &str, collection_name: &str) -> Self {
        let data_path = Path::new(data_dir);
        let persistence_path = if data_path.exists() || fs::create_dir_all(data_path).is_ok() {
            let file_path = data_path.join(format!("{}.json", collection_name));
            Some(file_path)
        } else {
            log::warn!("Failed to create vector store directory: {:?}", data_path);
            None
        };
        
        // Try to load persisted data
        let items = if let Some(path) = &persistence_path {
            if path.exists() {
                match fs::read_to_string(path) {
                    Ok(content) => match serde_json::from_str::<Vec<VectorItem>>(&content) {
                        Ok(loaded_items) => {
                            log::info!("Loaded {} items from vector store", loaded_items.len());
                            loaded_items
                        }
                        Err(e) => {
                            log::error!("Failed to parse vector store data: {}", e);
                            Vec::new()
                        }
                    },
                    Err(e) => {
                        log::error!("Failed to read vector store file: {}", e);
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Self {
            items: Arc::new(RwLock::new(items)),
            persistence_path,
            is_available: true,
        }
    }

    /// Check if vector store is available
    pub fn is_available(&self) -> bool {
        self.is_available
    }

    /// Persist the vector store to disk
    async fn persist(&self) -> AgentResult<()> {
        if let Some(path) = &self.persistence_path {
            let items = self.items.read().map_err(|e| {
                AgentError::VectorDb(format!("Failed to acquire read lock: {}", e))
            })?;
            
            let json = serde_json::to_string_pretty(&*items)
                .map_err(|e| AgentError::Serialization(e))?;
            
            fs::write(path, json).map_err(|e| {
                AgentError::VectorDb(format!("Failed to write vector store file: {}", e))
            })?;
            
            Ok(())
        } else {
            Err(AgentError::VectorDb("No persistence path configured".to_string()))
        }
    }

    /// Add an entry to the vector store
    pub async fn add(
        &self,
        id: String,
        content: &str,
        metadata: MemoryMetadata,
    ) -> AgentResult<()> {
        if !self.is_available() {
            return Err(AgentError::VectorDb("Vector store is not available".to_string()));
        }

        // Generate embeddings for the content
        let embedding = self.generate_embeddings(content).await?;
        
        let item = VectorItem {
            id,
            content: content.to_string(),
            metadata,
            timestamp: utils::current_timestamp(),
            embedding,
        };

        // Add to in-memory store
        {
            let mut items = self.items.write().map_err(|e| {
                AgentError::VectorDb(format!("Failed to acquire write lock: {}", e))
            })?;
            items.push(item);
        }
        
        // Persist to disk
        self.persist().await?;

        Ok(())
    }

    /// Search for relevant entries based on a query
    pub async fn search(&self, query: &str, limit: usize) -> AgentResult<Vec<MemoryEntry>> {
        if !self.is_available() {
            return Err(AgentError::VectorDb("Vector store is not available".to_string()));
        }

        // Generate embeddings for the query
        let query_embedding = self.generate_embeddings(query).await?;
        
        // Calculate cosine similarity with all stored vectors
        let items = self.items.read().map_err(|e| {
            AgentError::VectorDb(format!("Failed to acquire read lock: {}", e))
        })?;
        
        // Vector with (index, similarity score) pairs
        let mut similarities: Vec<(usize, f32)> = Vec::with_capacity(items.len());
        
        for (idx, item) in items.iter().enumerate() {
            let similarity = cosine_similarity(&query_embedding, &item.embedding);
            similarities.push((idx, similarity));
        }
        
        // Sort by similarity score (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Take top matches
        let mut results = Vec::with_capacity(limit.min(similarities.len()));
        for (idx, _score) in similarities.iter().take(limit) {
            let item = &items[*idx];
            
            results.push(MemoryEntry {
                id: item.id.clone(),
                content: item.content.clone(),
                metadata: item.metadata.clone(),
                timestamp: item.timestamp.clone(),
                embeddings: Some(item.embedding.clone()),
            });
        }
        
        Ok(results)
    }

    /// Generate embeddings for a text input
    /// In a real implementation, this would call an embedding model API
    /// For this MVP, we'll use a simple placeholder
    async fn generate_embeddings(&self, text: &str) -> AgentResult<Vec<f32>> {
        // This is a placeholder implementation
        // In a real system, you would call an API like OpenAI's embeddings API
        
        // For now, we'll create a dummy embedding vector of the right size
        // A real implementation would integrate with the rust-bert crate or call an API
        
        // Create a deterministic but different vector based on the hash of the text
        let mut hash = 0u64;
        for c in text.chars() {
            hash = hash.wrapping_mul(31).wrapping_add(c as u64);
        }
        
        let mut rng = hash;
        let embeddings: Vec<f32> = (0..EMBEDDING_SIZE)
            .map(|_| {
                // Simple pseudo-random number generation based on the hash and position
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                let val = ((rng >> 33) as f32 / u32::MAX as f32) * 2.0 - 1.0;
                val
            })
            .collect();
        
        // Normalize the vector to unit length for cosine similarity
        let magnitude: f32 = embeddings.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized: Vec<f32> = embeddings.into_iter().map(|x| x / magnitude).collect();
        
        Ok(normalized)
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(a, b)| a * b).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if magnitude_a * magnitude_b == 0.0 {
        0.0
    } else {
        dot_product / (magnitude_a * magnitude_b)
    }
}