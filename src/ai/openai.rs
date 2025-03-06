use std::collections::HashMap;
use crate::ai::{AiClient, Message, MessageRole, ChatCompletionResponse, FunctionCall};
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
/*
#[derive(Debug, Serialize)]
struct OpenAiCompletionRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    temperature: f32,
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    functions: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function_call: Option<Value>,
}*/

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompletionResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GptFunctionCall {
    pub name: String,
    pub arguments:  serde_json::Value , // serde_json::Value  //this a stringified array i think         // Vec< Box<serde_json::Value> >
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GptToolCall {
    pub r#type: String,//will always be 'function'
    pub function: GptFunctionCall, // serde_json::Value  //this a stringified array i think         // Vec< Box<serde_json::Value> >
}



#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    /*#[serde(default)]
    content: Option<String>,
    #[serde(default)]
    function_call: Option<OpenAiFunctionCall>,*/


      pub content: Option<String>,
    pub role: Option<String>,
    pub tool_calls: Option<Vec<GptToolCall>>,

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
                name: m.name,
            })
            .collect();
            


           

           let request_body = json!({
                        "model": self.model,
                        "messages": api_messages,

                        //"temperature": 0.7,
                        // "max_tokens": Some(4000),
                       
                    
                    });



 
        
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .json(& request_body )
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
        
        Ok(response_data.choices[0].message.content.clone().unwrap_or_default())
    }
    
    async fn chat_completion_with_functions(
        &self, 
        messages: Vec<Message>, 
        functions: Value
    ) -> AgentResult<ChatCompletionResponse> {



        let api_messages: Vec<OpenAiMessage> = messages
            .into_iter()
            .map(|m| OpenAiMessage {
                role: m.role.to_string(),
                content: m.content,
                name: m.name,
            })
            .collect();


            let mut function_tools = Vec::new();


            if let Some(functions_array) = functions.as_array(){

                for func_raw in functions_array {


                    if let Ok(func) = serde_json::from_value::<OpenAiCallableFunction>( func_raw.clone() ){

                    function_tools .push(    FunctionTool{

                           r#type: "function".into(),
                           function: func.clone()
                        } );
                   }
                }
                    
            }

               println!("api_messages {:?}", api_messages);
           

            println!("function_tools {:?}", function_tools);


           let enable_function_calling = true ; // for now 

           let request_body = match enable_function_calling {
 
                    true => json!({
                           // "model": self.model.clone(),
                             "model": "gpt-4o" ,

                            "messages": api_messages,
                            "temperature": 0.7,  // Lower temperature for more deterministic responses
                            "max_tokens": 8000,  // Ensure enough tokens for multiple tool calls

                            "tools": function_tools,
                            "tool_choice": "auto",  // Allow multiple tool calls
                            "parallel_tool_calls" : true
                        }),

                    false => json!({
                        "model": self.model,
                        "messages": api_messages,

                        //"temperature": 0.7,
                        // "max_tokens": Some(4000),
                       
                    
                    }),

        };



            
            println!("request_body {:?}", request_body);




/*
        let request = OpenAiCompletionRequest {
            model: self.model.clone(),
            messages: api_messages,
            temperature: 0.7,
            max_tokens: Some(4000),
            functions: Some(functions),

            function_call: Some(json!({"name": "required"})), // Properly formatted to force function calling

           // function_call: json!("required").into()   //Some(json!("auto")),
        };*/
        
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AgentError::AiApi(format!("OpenAI API request failed: {}", e)))?;
        

           println!("response {:?}", response);


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
        


           println!("response_data {:?}", response_data);

        if response_data.choices.is_empty() {
            return Err(AgentError::AiApi("OpenAI API returned no choices".to_string()));
        }
        
        let message = &response_data.choices[0].message;
        
        let result = if let Some(tool_calls) = &message.tool_calls {



            ChatCompletionResponse {
                content: message.content.clone(),
                tool_calls: Some(tool_calls.to_vec()),
            }


            
        } else {
            ChatCompletionResponse {
                content: message.content.clone(),
                tool_calls: None,
            }
        };
        
        Ok(result)
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


#[derive(Serialize,Debug)]
pub struct FunctionTool {
    
    r#type: String,
    function: OpenAiCallableFunction 
}



#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAiCallableFunction {
    pub name: String,
    pub description: String,
    pub parameters: OpenAiCallableParameters,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAiCallableParameters {
    #[serde(rename = "type")]
    pub type_field: String,
    pub properties: HashMap<String, serde_json::Value>, //Box<serde_json::Value> >,  // value is actually a property descriptor
    pub required: Vec<String>,                          //these are the required properties
}
