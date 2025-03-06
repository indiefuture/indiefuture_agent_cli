pub mod ai;
pub mod cli;
pub mod codebase;
pub mod config;
pub mod error;
pub mod memory;
pub mod task;
pub mod utils;
pub mod agent_engine;

pub mod subtasks;

pub use error::{AgentError, AgentResult};

// Re-export main types for easier access
pub use ai::AiClient;
pub use cli::run_cli;
pub use config::Settings;
pub use memory::{MemoryManager, WorkingContext};
pub use task::{SubTask, SubTaskType, SubTaskExecutor   };