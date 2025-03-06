use crate::ai::{AiClient, Message, MessageRole, ChatCompletionResponse, FunctionCall};
use crate::error::{AgentError, AgentResult};
use async_trait::async_trait;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ClaudeClient {
    client: Client,
    api_key: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct ClaudeCompletionRequest {
    model: String,
    messages: Vec<ClaudeMessage>,
    temperature: f32,
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: Vec<ClaudeContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Serialize)]
struct ClaudeContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeCompletionResponse {
    content: Vec<ClaudeResponseContent>,
    #[serde(default)]
    tool_calls: Option<Vec<ClaudeToolCall>>,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponseContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeToolCall {
    id: String,
    #[serde(rename = "type")]
    tool_type: String,
    name: String,
    input: Value,
}

impl ClaudeClient {
    pub fn new(api_key: &str, model: &str) -> AgentResult<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        
        let mut auth_value = header::HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|e| AgentError::AiApi(format!("Invalid API key format: {}", e)))?;
        auth_value.set_sensitive(true);
        headers.insert("x-api-key", auth_value);
        headers.insert(
            "anthropic-version", 
            header::HeaderValue::from_static("2023-06-01")
        );
        
        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(120))
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
impl AiClient for ClaudeClient {
    async fn generate_text(&self, messages: Vec<Message>) -> AgentResult<String> {
        // Extract system message if present
        let mut system_message = None;
        let filtered_messages: Vec<Message> = messages
            .into_iter()
            .filter(|m| {
                if m.role == MessageRole::System {
                    system_message = Some(m.content.clone());
                    false
                } else {
                    true
                }
            })
            .collect();
        
        let api_messages: Vec<ClaudeMessage> = filtered_messages
            .into_iter()
            .map(|m| ClaudeMessage {
                role: m.role.to_string(),
                content: vec![ClaudeContent {
                    content_type: "text".to_string(),
                    text: m.content,
                }],
                name: m.name,
            })
            .collect();
        
        let request = ClaudeCompletionRequest {
            model: self.model.clone(),
            messages: api_messages,
            temperature: 0.7,
            max_tokens: Some(4000),
            system: system_message,
            tools: None,
        };
        
        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .json(&request)
            .send()
            .await
            .map_err(|e| AgentError::AiApi(format!("Claude API request failed: {}", e)))?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AgentError::AiApi(format!(
                "Claude API returned error status: {}, body: {}",
                status,
                error_text
            )));
        }
        
        let response_data: ClaudeCompletionResponse = response
            .json()
            .await
            .map_err(|e| AgentError::AiApi(format!("Failed to parse Claude response: {}", e)))?;
        
        // Concatenate all text content from response
        let content: String = response_data
            .content
            .iter()
            .filter(|c| c.content_type == "text")
            .map(|c| c.text.clone())
            .collect();
        
        if content.is_empty() {
            return Err(AgentError::AiApi("Claude API returned no content".to_string()));
        }
        
        Ok(content)
    }
    
    async fn chat_completion_with_functions(
        &self, 
        messages: Vec<Message>, 
        functions: Value
    ) -> AgentResult<ChatCompletionResponse> {
        // Extract system message if present
        let mut system_message = None;
        let filtered_messages: Vec<Message> = messages
            .into_iter()
            .filter(|m| {
                if m.role == MessageRole::System {
                    system_message = Some(m.content.clone());
                    false
                } else {
                    true
                }
            })
            .collect();
        
        let api_messages: Vec<ClaudeMessage> = filtered_messages
            .into_iter()
            .map(|m| ClaudeMessage {
                role: m.role.to_string(),
                content: vec![ClaudeContent {
                    content_type: "text".to_string(),
                    text: m.content,
                }],
                name: m.name,
            })
            .collect();
        
        // Convert OpenAI-style functions to Claude tools
        let tools = if let Value::Array(function_array) = &functions {
            let claude_tools: Vec<Value> = function_array
                .iter()
                .map(|func| {
                    let name = func["name"].as_str().unwrap_or("unknown");
                    let description = func["description"].as_str().unwrap_or("");
                    let parameters = func["parameters"].clone();
                    
                    json!({
                        "type": "function",
                        "function": {
                            "name": name,
                            "description": description,
                            "parameters": parameters
                        }
                    })
                })
                .collect();
            
            Some(claude_tools)
        } else {
            None
        };
        
        let request = ClaudeCompletionRequest {
            model: self.model.clone(),
            messages: api_messages,
            temperature: 0.7,
            max_tokens: Some(4000),
            system: system_message,
            tools,
        };
        
        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .json(&request)
            .send()
            .await
            .map_err(|e| AgentError::AiApi(format!("Claude API request failed: {}", e)))?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AgentError::AiApi(format!(
                "Claude API returned error status: {}, body: {}",
                status,
                error_text
            )));
        }
        
        let response_data: ClaudeCompletionResponse = response
            .json()
            .await
            .map_err(|e| AgentError::AiApi(format!("Failed to parse Claude response: {}", e)))?;
        
        // Extract content
        let content: String = response_data
            .content
            .iter()
            .filter(|c| c.content_type == "text")
            .map(|c| c.text.clone())
            .collect();
        
        // Process tool calls if present
        if let Some(tool_calls) = response_data.tool_calls {
            if !tool_calls.is_empty() {
                let tool_call = &tool_calls[0]; // Take the first tool call
                
                // Convert the input to a JSON string
                let arguments = serde_json::to_string(&tool_call.input)
                    .map_err(|e| AgentError::AiApi(format!("Failed to serialize function arguments: {}", e)))?;
                
                return Ok(ChatCompletionResponse {
                    content: Some(content),

                    tool_calls : None  // FOR NOW !! FIX !! !!!!!!!!!!
                    
                });
            }
        }
        
        // No tool calls, just return the content
        Ok(ChatCompletionResponse {
            content: Some(content),
            tool_calls: None,
        })
    }
    
    async fn generate_embeddings(&self, _text: &str) -> AgentResult<Vec<f32>> {
        // Claude doesn't have an official embeddings API as of this implementation
        // For the MVP, we'll just return an error, and the system will use OpenAI for embeddings
        Err(AgentError::AiApi(
            "Claude does not support embeddings. Use OpenAI for embedding generation.".to_string(),
        ))
    }
    
    fn provider_name(&self) -> String {
        "claude".to_string()
    }
    
    fn model_name(&self) -> String {
        self.model.clone()
    }
    
    fn clone_box(&self) -> Box<dyn AiClient> {
        Box::new(self.clone())
    }
}