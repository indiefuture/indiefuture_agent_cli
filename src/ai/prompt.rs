use crate::ai::{Message, MessageRole};
use crate::memory::MemoryEntry;
use crate::subtasks::OperationType;
use serde_json::{json, Value};

/// Builds a prompt with relevant context for the AI
pub struct PromptBuilder {
    system_message: String,
    user_query: String,
    max_context_tokens: usize,
    contexts: Vec<ContextBlock>,
}

/// A block of context with metadata for inclusion in the prompt
#[derive(Clone)]
struct ContextBlock {
    content: String,
    source: String,
    relevance_score: f32,
    estimated_tokens: usize,
}

impl PromptBuilder {
    pub fn new(system_message: String, user_query: String) -> Self {
        Self {
            system_message,
            user_query,
            max_context_tokens: 8000, // Conservative limit for most models
            contexts: Vec::new(),
        }
    }

    /// Add context from memory entries
    pub fn add_memory_context(&mut self, memories: &[MemoryEntry], token_limit: Option<usize>) -> &mut Self {
        if let Some(limit) = token_limit {
            self.max_context_tokens = limit;
        }

        for memory in memories {
            // Roughly estimate tokens (1 token ≈ 4 chars for English text)
            let estimated_tokens = memory.content.len() / 4;
            
            let context = ContextBlock {
                content: memory.content.clone(),
                source: memory.metadata.source.clone(),
                relevance_score: 1.0, // In a real system, this would be the similarity score
                estimated_tokens,
            };
            
            self.contexts.push(context);
        }
        
        self
    }
    
    /// Add a single block of context
    pub fn add_context(&mut self, content: &str, source: &str, relevance_score: f32) -> &mut Self {
        let estimated_tokens = content.len() / 4;
        
        let context = ContextBlock {
            content: content.to_string(),
            source: source.to_string(),
            relevance_score,
            estimated_tokens,
        };
        
        self.contexts.push(context);
        self
    }
    
    /// Set the maximum number of tokens to use for context
    pub fn set_max_context_tokens(&mut self, max_tokens: usize) -> &mut Self {
        self.max_context_tokens = max_tokens;
        self
    }
    
    /// Build the final messages for the AI
    pub fn build(&self) -> Vec<Message> {
        let system_message = Message {
            role: MessageRole::System,
            content: self.system_message.clone(),
            name: None,
        };
        
        // Sort contexts by relevance score
        let mut sorted_contexts = self.contexts.clone();
        sorted_contexts.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        
        // Build context string within token limit
        let mut context_string = String::new();
        let mut total_tokens = 0;
        
        for context in sorted_contexts {
            if total_tokens + context.estimated_tokens > self.max_context_tokens {
                break;
            }
            
            context_string.push_str(&format!("\n\n--- {} ---\n{}", context.source, context.content));
            total_tokens += context.estimated_tokens;
        }
        
        // Build user message with context
        let user_message = if !context_string.is_empty() {
            Message {
                role: MessageRole::User,
                content: format!(
                    "Use the following context to answer my question:\n{}\n\nMy question is: {}",
                    context_string, self.user_query
                ),
                name: None,
            }
        } else {
            Message {
                role: MessageRole::User,
                content: self.user_query.clone(),
                name: None,
            }
        };
        
        vec![system_message, user_message]
    }
}

/// Helper function to estimate the number of tokens in a string
/// This is a rough approximation; a real implementation would use a tokenizer
pub fn estimate_tokens(text: &str) -> usize {
    // Roughly 4 characters per token for English text
    text.len() / 4
}

/// Creates function definitions for OpenAI API to structure task decomposition
pub fn create_subtask_functions() -> Value {
    json!([
        {
            "name": "execute_search",
            "description": "Search for files in the codebase based on a query",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query to find relevant files"
                    },
                    "priority": {
                        "type": "integer",
                        "description": "Task priority (1-5, where 1 is highest)",
                        "default": 2
                    }
                },
                "required": ["query"]
            }
        },
        {
            "name": "execute_read",
            "description": "Read a specific file path to analyze its contents",
            "parameters": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    },
                    "priority": {
                        "type": "integer",
                        "description": "Task priority (1-5, where 1 is highest)",
                        "default": 2
                    }
                },
                "required": ["file_path"]
            }
        },
        {
            "name": "execute_update",
            "description": "Update or modify a file with new content",
            "parameters": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to the file to update"
                    },
                    "changes": {
                        "type": "string",
                        "description": "Description of the changes to make"
                    },
                    "priority": {
                        "type": "integer",
                        "description": "Task priority (1-5, where 1 is highest)",
                        "default": 3
                    }
                },
                "required": ["file_path", "changes"]
            }
        },
        {
            "name": "execute_bash",
            "description": "Execute a bash command on the system",
            "parameters": {
                "type": "object", 
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute"
                    },
                    "priority": {
                        "type": "integer",
                        "description": "Task priority (1-5, where 1 is highest)",
                        "default": 3
                    }
                },
                "required": ["command"]
            }
        },
        {
            "name": "execute_task",
            "description": "Create a general task that processes input and responds with analysis",
            "parameters": {
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "Description of the task to perform"
                    },
                    "priority": {
                        "type": "integer",
                        "description": "Task priority (1-5, where 1 is highest)",
                        "default": 3
                    }
                },
                "required": ["description"]
            }
        }
    ])
}

/// Maps function names to operation types
pub fn function_name_to_operation_type(function_name: &str) -> OperationType {
    match function_name {
        "execute_search" => OperationType::SEARCH,
        "execute_read" => OperationType::READ,
        "execute_update" => OperationType::UPDATE,
        "execute_bash" => OperationType::BASH,
        "execute_task" => OperationType::TASK,
        _ => OperationType::UNKNOWN
    }
}

/// Creates function definitions for UPDATE operations
pub fn create_update_functions() -> Value {
    json!([
        {
            "name": "delete_lines",
            "description": "Delete lines from a file",
            "parameters": {
                "type": "object",
                "properties": {
                    "start_line": {
                        "type": "integer",
                        "description": "The line number to start deletion (1-based)"
                    },
                    "end_line": {
                        "type": "integer",
                        "description": "The line number to end deletion (inclusive)"
                    }
                },
                "required": ["start_line", "end_line"]
            }
        },
        {
            "name": "add_lines",
            "description": "Add lines to a file at a specific position",
            "parameters": {
                "type": "object",
                "properties": {
                    "line_number": {
                        "type": "integer",
                        "description": "The line number where to add content (1-based, content will be added after this line)"
                    },
                    "content": {
                        "type": "string",
                        "description": "The text content to add"
                    }
                },
                "required": ["line_number", "content"]
            }
        },
        {
            "name": "edit_lines",
            "description": "Edit specific lines in a file",
            "parameters": {
                "type": "object",
                "properties": {
                    "start_line": {
                        "type": "integer",
                        "description": "The line number to start editing (1-based)"
                    },
                    "end_line": {
                        "type": "integer",
                        "description": "The line number to end editing (inclusive)"
                    },
                    "new_content": {
                        "type": "string",
                        "description": "The new content to replace the specified lines"
                    }
                },
                "required": ["start_line", "end_line", "new_content"]
            }
        },
        {
            "name": "replace_text",
            "description": "Replace specific text in a file",
            "parameters": {
                "type": "object",
                "properties": {
                    "old_text": {
                        "type": "string",
                        "description": "The text to replace"
                    },
                    "new_text": {
                        "type": "string",
                        "description": "The new text to insert"
                    }
                },
                "required": ["old_text", "new_text"]
            }
        }
    ])
}