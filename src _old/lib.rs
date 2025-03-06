pub mod ai;
pub mod cli;
pub mod codebase;
pub mod config;
pub mod error;
pub mod memory;
pub mod storage;
pub mod task;
pub mod tools;
pub mod utils;

pub use error::{AgentError, AgentResult};

// Re-export main types for easier access
pub use ai::AiClient;
pub use cli::run_cli;
pub use config::Settings;
pub use memory::{MemoryManager, WorkingContext};
pub use storage::Storage;
pub use task::{Task, TaskExecutor};
pub use tools::{Tool, ToolArgs, ToolOutput, SharedContext};