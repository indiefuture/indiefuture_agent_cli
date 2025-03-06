pub mod claude;
pub mod openai;
//pub mod prompt;

use crate::ai::openai::GptToolCall;
use crate::error::AgentResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    Developer,
    User,
    Assistant,
}

impl ToString for MessageRole {
    fn to_string(&self) -> String {
        match self {
            MessageRole::Developer => "developer".to_string(),
            MessageRole::System => "system".to_string(),
            MessageRole::User => "user".to_string(),
            MessageRole::Assistant => "assistant".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    //not used ?
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<GptToolCall>>,
}

#[async_trait]
pub trait AiClient: Send + Sync {
    /// Generate text from a conversation history
    async fn generate_text(&self, messages: Vec<Message>) -> AgentResult<String>;

    /// Generate chat completion with functions
    async fn chat_completion_with_functions(
        &self,
        messages: Vec<Message>,
        functions: serde_json::Value,
        force_message_only: bool,
    ) -> AgentResult<ChatCompletionResponse> {
        // Default implementation to support backward compatibility
        // Derived implementations should override this
        Err(crate::error::AgentError::AiApi(
            "Function calling not implemented for this AI provider".to_string(),
        ))
    }

    /// Generate embeddings for a text
    async fn generate_embeddings(&self, text: &str) -> AgentResult<Vec<f32>>;

    /// Get the name of the AI provider
    fn provider_name(&self) -> String;

    /// Get the model name being used
    fn model_name(&self) -> String;

    /// Create a clone of this client
    fn clone_box(&self) -> Box<dyn AiClient>;
}

/// Factory function to create an AI client based on configuration
pub fn create_ai_client(
    provider: &str,
    model: &str,
    api_key: &str,
) -> AgentResult<Box<dyn AiClient>> {
    match provider {
        "openai" => {
            let client = openai::OpenAiClient::new(api_key, model)?;
            Ok(Box::new(client))
        }
        "claude" => {
            let client = claude::ClaudeClient::new(api_key, model)?;
            Ok(Box::new(client))
        }
        _ => Err(crate::error::AgentError::AiApi(format!(
            "Unsupported AI provider: {}",
            provider
        ))),
    }
}
