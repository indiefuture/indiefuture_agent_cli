use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Vector database error: {0}")]
    VectorDb(String),

    #[error("AI API error: {0}")]
    AiApi(String),

    #[error("Task execution error: {0}")]
    TaskExecution(String),

    #[error("Code parsing error: {0}")]
    CodeParsing(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("CLI error: {0}")]
    Cli(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    #[error("Working context error: {0}")]
    WorkingContext(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type AgentResult<T> = Result<T, AgentError>;
