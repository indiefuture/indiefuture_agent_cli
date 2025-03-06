use crate::ai::{AiClient, Message, MessageRole};
use crate::error::{AgentError, AgentResult};
use async_trait::async_trait;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct OpenAiClient {
    client: Client,
    api_key: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct OpenAiCompletionRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    temperature: f32,
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    functions: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function_call: Option<String>,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompletionResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    #[serde(default)]
    content: String,
    #[serde(default)]
    function_call: Option<OpenAiFunctionCall>,
}

#[derive(Debug, Deserialize)]
struct OpenAiFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize)]
struct OpenAiEmbeddingRequest {
    model: String,
    input: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiEmbeddingResponse {
    data: Vec<OpenAiEmbedding>,
}

#[derive(Debug, Deserialize)]
struct OpenAiEmbedding {
    embedding: Vec<f32>,
}

impl OpenAiClient {
    pub fn new(api_key: &str, model: &str) -> AgentResult<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        
        let mut auth_value = header::HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|e| AgentError::AiApi(format!("Invalid API key format: {}", e)))?;
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);
        
        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| AgentError::AiApi(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(Self {
            client,
            api_key: api_key.to_string(),
            model: model.to_string(),
        })
    }
}

#[async_trait]
impl AiClient for OpenAiClient {
    async fn generate_text(&self, messages: Vec<Message>) -> AgentResult<String> {
        let api_messages: Vec<OpenAiMessage> = messages
            .into_iter()
            .map(|m| OpenAiMessage {
                role: m.role.to_string(),
                content: m.content,
            })
            .collect();
        
        let request = OpenAiCompletionRequest {
            model: self.model.clone(),
            messages: api_messages,
            temperature: 0.7,
            max_tokens: Some(4000),
            functions: None,
            function_call: None,
        };
        
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .json(&request)
            .send()
            .await
            .map_err(|e| AgentError::AiApi(format!("OpenAI API request failed: {}", e)))?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AgentError::AiApi(format!(
                "OpenAI API returned error status: {}, body: {}",
                status,
                error_text
            )));
        }
        
        let response_data: OpenAiCompletionResponse = response
            .json()
            .await
            .map_err(|e| AgentError::AiApi(format!("Failed to parse OpenAI response: {}", e)))?;
        
        if response_data.choices.is_empty() {
            return Err(AgentError::AiApi("OpenAI API returned no choices".to_string()));
        }
        
        Ok(response_data.choices[0].message.content.clone())
    }
    
    async fn generate_text_with_functions(
        &self,
        messages: Vec<Message>, 
        functions: Value,
        function_call: Option<&str>
    ) -> AgentResult<Value> {
        let api_messages: Vec<OpenAiMessage> = messages
            .into_iter()
            .map(|m| OpenAiMessage {
                role: m.role.to_string(),
                content: m.content,
            })
            .collect();
        
        let function_call_str = function_call.map(|s| s.to_string());
        
        let request = OpenAiCompletionRequest {
            model: self.model.clone(),
            messages: api_messages,
            temperature: 0.7,
            max_tokens: Some(4000),
            functions: Some(functions),
            function_call: function_call_str,
        };
        
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .json(&request)
            .send()
            .await
            .map_err(|e| AgentError::AiApi(format!("OpenAI API request failed: {}", e)))?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AgentError::AiApi(format!(
                "OpenAI API returned error status: {}, body: {}",
                status,
                error_text
            )));
        }
        
        let response_data: OpenAiCompletionResponse = response
            .json()
            .await
            .map_err(|e| AgentError::AiApi(format!("Failed to parse OpenAI response: {}", e)))?;
        
        if response_data.choices.is_empty() {
            return Err(AgentError::AiApi("OpenAI API returned no choices".to_string()));
        }
        
        let message = &response_data.choices[0].message;
        
        if let Some(function_call) = &message.function_call {
            // Parse the function call arguments as JSON
            let arguments = match serde_json::from_str::<Value>(&function_call.arguments) {
                Ok(args) => args,
                Err(e) => return Err(AgentError::AiApi(format!("Failed to parse function arguments: {}", e))),
            };
            
            // Return a structured JSON response
            Ok(json!({
                "function_calls": [{
                    "name": function_call.name,
                    "arguments": arguments
                }]
            }))
        } else {
            // No function call, just return the content
            Ok(json!({
                "content": message.content,
                "function_calls": []
            }))
        }
    }
    
    async fn generate_embeddings(&self, text: &str) -> AgentResult<Vec<f32>> {
        let request = OpenAiEmbeddingRequest {
            model: "text-embedding-ada-002".to_string(), // Standard embedding model
            input: text.to_string(),
        };
        
        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .json(&request)
            .send()
            .await
            .map_err(|e| AgentError::AiApi(format!("OpenAI API request failed: {}", e)))?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AgentError::AiApi(format!(
                "OpenAI API returned error status: {}, body: {}",
                status,
                error_text
            )));
        }
        
        let response_data: OpenAiEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| AgentError::AiApi(format!("Failed to parse OpenAI response: {}", e)))?;
        
        if response_data.data.is_empty() {
            return Err(AgentError::AiApi("OpenAI API returned no embeddings".to_string()));
        }
        
        Ok(response_data.data[0].embedding.clone())
    }
    
    fn provider_name(&self) -> String {
        "openai".to_string()
    }
    
    fn model_name(&self) -> String {
        self.model.clone()
    }
    
    fn clone_box(&self) -> Box<dyn AiClient> {
        Box::new(self.clone())
    }
}