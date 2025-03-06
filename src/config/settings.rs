use crate::error::{AgentError, AgentResult};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // AI API settings
    pub openai_api_key: Option<String>,
    pub claude_api_key: Option<String>,
    pub default_ai_provider: String,
    pub default_model: String,

    // Storage settings
    pub vector_store_path: PathBuf,
    pub sled_path: PathBuf,
    pub collection_name: String,

    // Task settings
    pub max_concurrent_tasks: usize,
    pub default_timeout_seconds: u64,

    // Codebase settings
    pub default_scan_path: PathBuf,
    pub ignore_patterns: Vec<String>,
    pub supported_extensions: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let default_data_dir = home_dir.join(".indiefuture");

        Self {
            openai_api_key: None,
            claude_api_key: None,
            default_ai_provider: "openai".to_string(),
            default_model: "gpt-4o".to_string(),
            vector_store_path: default_data_dir.join("vector_store"),
            sled_path: default_data_dir.join("sled_db"),
            collection_name: "code_embeddings".to_string(),
            max_concurrent_tasks: 5,
            default_timeout_seconds: 30,
            default_scan_path: PathBuf::from("."),
            ignore_patterns: vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
            ],
            supported_extensions: vec![
                "rs".to_string(),
                "ts".to_string(),
                "js".to_string(),
                "py".to_string(),
                "go".to_string(),
                "java".to_string(),
                "c".to_string(),
                "cpp".to_string(),
                "h".to_string(),
                "json".to_string(),
                "toml".to_string(),
                "yaml".to_string(),
                "md".to_string(),
                "txt".to_string(),
            ],
        }
    }
}

impl Settings {
    pub fn load() -> AgentResult<Self> {
        // Try to load .env file, but continue if it doesn't exist
        let _ = dotenv();

        let mut settings = Self::default();

        // Load settings from environment variables
        if let Ok(key) = env::var("OPENAI_API_KEY") {
            settings.openai_api_key = Some(key);
        }

        if let Ok(key) = env::var("CLAUDE_API_KEY") {
            settings.claude_api_key = Some(key);
        }

        if let Ok(provider) = env::var("DEFAULT_AI_PROVIDER") {
            settings.default_ai_provider = provider;
        }

        if let Ok(model) = env::var("DEFAULT_MODEL") {
            settings.default_model = model;
        }

        if let Ok(path) = env::var("VECTOR_STORE_PATH") {
            settings.vector_store_path = PathBuf::from(path);
        }

        if let Ok(path) = env::var("SLED_PATH") {
            settings.sled_path = PathBuf::from(path);
        }

        if let Ok(name) = env::var("COLLECTION_NAME") {
            settings.collection_name = name;
        }

        if let Ok(max_tasks) = env::var("MAX_CONCURRENT_TASKS") {
            if let Ok(max_tasks) = max_tasks.parse::<usize>() {
                settings.max_concurrent_tasks = max_tasks;
            }
        }

        if let Ok(timeout) = env::var("DEFAULT_TIMEOUT_SECONDS") {
            if let Ok(timeout) = timeout.parse::<u64>() {
                settings.default_timeout_seconds = timeout;
            }
        }

        // Ensure required directories exist
        if !settings.sled_path.exists() {
            if let Err(e) = std::fs::create_dir_all(&settings.sled_path) {
                return Err(AgentError::Configuration(format!(
                    "Failed to create Sled directory: {}",
                    e
                )));
            }
        }

        Ok(settings)
    }

    pub fn validate(&self) -> AgentResult<()> {
        match self.default_ai_provider.as_str() {
            "openai" => {
                if self.openai_api_key.is_none() {
                    return Err(AgentError::Configuration(
                        "OpenAI API key is required when using OpenAI provider".to_string(),
                    ));
                }
            }
            "claude" => {
                if self.claude_api_key.is_none() {
                    return Err(AgentError::Configuration(
                        "Claude API key is required when using Claude provider".to_string(),
                    ));
                }
            }
            provider => {
                return Err(AgentError::Configuration(format!(
                    "Unsupported AI provider: {}",
                    provider
                )));
            }
        }

        Ok(())
    }
}
