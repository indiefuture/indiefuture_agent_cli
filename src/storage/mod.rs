pub mod sled_store;

pub use sled_store::SledStore;

use crate::error::AgentResult;
use serde::{de::DeserializeOwned, Serialize};

/// Trait for persistent storage implementations
pub trait Storage: Send + Sync {
    /// Store a value with a key
    fn put<T: Serialize>(&self, key: &str, value: &T) -> AgentResult<()>;
    
    /// Retrieve a value by key
    fn get<T: DeserializeOwned>(&self, key: &str) -> AgentResult<Option<T>>;
    
    /// Delete a value by key
    fn delete(&self, key: &str) -> AgentResult<()>;
    
    /// Check if a key exists
    fn contains(&self, key: &str) -> AgentResult<bool>;
    
    /// List all keys with a prefix
    fn list_keys(&self, prefix: &str) -> AgentResult<Vec<String>>;
}