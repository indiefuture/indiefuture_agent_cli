use crate::error::{AgentError, AgentResult};
use crate::storage::Storage;
use serde::{de::DeserializeOwned, Serialize};
use sled::Db;
use std::path::Path;

/// Persistent storage using Sled database
pub struct SledStore {
    db: Db,
}

impl SledStore {
    pub fn new(path: &Path) -> AgentResult<Self> {
        let db = sled::open(path).map_err(|e| {
            AgentError::Storage(format!("Failed to open Sled database at {:?}: {}", path, e))
        })?;
        
        Ok(Self { db })
    }
}

impl Storage for SledStore {
    fn put<T: Serialize>(&self, key: &str, value: &T) -> AgentResult<()> {
        let serialized = serde_json::to_vec(value)
            .map_err(|e| AgentError::Serialization(e))?;
        
        self.db
            .insert(key.as_bytes(), serialized)
            .map_err(|e| {
                AgentError::Storage(format!("Failed to store value for key '{}': {}", key, e))
            })?;
        
        self.db.flush().map_err(|e| {
            AgentError::Storage(format!("Failed to flush database: {}", e))
        })?;
        
        Ok(())
    }
    
    fn get<T: DeserializeOwned>(&self, key: &str) -> AgentResult<Option<T>> {
        match self.db.get(key.as_bytes()) {
            Ok(Some(bytes)) => {
                let value = serde_json::from_slice(&bytes)
                    .map_err(|e| AgentError::Serialization(e))?;
                Ok(Some(value))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(AgentError::Storage(format!(
                "Failed to get value for key '{}': {}",
                key, e
            ))),
        }
    }
    
    fn delete(&self, key: &str) -> AgentResult<()> {
        self.db
            .remove(key.as_bytes())
            .map_err(|e| {
                AgentError::Storage(format!("Failed to delete key '{}': {}", key, e))
            })?;
        
        self.db.flush().map_err(|e| {
            AgentError::Storage(format!("Failed to flush database: {}", e))
        })?;
        
        Ok(())
    }
    
    fn contains(&self, key: &str) -> AgentResult<bool> {
        match self.db.contains_key(key.as_bytes()) {
            Ok(exists) => Ok(exists),
            Err(e) => Err(AgentError::Storage(format!(
                "Failed to check if key '{}' exists: {}",
                key, e
            ))),
        }
    }
    
    fn list_keys(&self, prefix: &str) -> AgentResult<Vec<String>> {
        let prefix_bytes = prefix.as_bytes();
        let iter = self.db.scan_prefix(prefix_bytes);
        
        let mut keys = Vec::new();
        for item in iter {
            match item {
                Ok((key_bytes, _)) => {
                    if let Ok(key_str) = std::str::from_utf8(&key_bytes) {
                        keys.push(key_str.to_string());
                    }
                }
                Err(e) => {
                    return Err(AgentError::Storage(format!(
                        "Failed to scan keys with prefix '{}': {}",
                        prefix, e
                    )))
                }
            }
        }
        
        Ok(keys)
    }
}