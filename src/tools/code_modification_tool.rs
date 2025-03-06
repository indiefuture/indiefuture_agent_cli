use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde_json::{json, Value};

use crate::error::{AgentError, AgentResult};
use crate::memory::MemoryManager;
use crate::tools::{Capability, EventType, Tool, ToolArgs, ToolOutput};

/// Tool for modifying code files
pub struct CodeModificationTool {
    workspace_root: PathBuf,
    memory_manager: Arc<MemoryManager>,
    require_confirmation: bool,
}

impl CodeModificationTool {
    pub fn new(workspace_root: PathBuf, memory_manager: Arc<MemoryManager>, require_confirmation: bool) -> Self {
        Self {
            workspace_root,
            memory_manager,
            require_confirmation,
        }
    }
    
    /// Validate file path is within workspace
    fn validate_path(&self, path: &Path) -> AgentResult<PathBuf> {
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };
        
        // Check that the path is inside the workspace root
        match absolute_path.strip_prefix(&self.workspace_root) {
            Ok(_) => Ok(absolute_path),
            Err(_) => Err(AgentError::ToolExecution(format!(
                "Path {} is outside the workspace root", path.display()
            ))),
        }
    }
    
    /// Read a file's content
    fn read_file(&self, path: &Path) -> AgentResult<String> {
        let validated_path = self.validate_path(path)?;
        
        fs::read_to_string(&validated_path)
            .map_err(|e| AgentError::ToolExecution(format!(
                "Failed to read file {}: {}", path.display(), e
            )))
    }
    
    /// Write content to a file
    fn write_file(&self, path: &Path, content: &str) -> AgentResult<()> {
        let validated_path = self.validate_path(path)?;
        
        // Create parent directories if they don't exist
        if let Some(parent) = validated_path.parent() {
            fs::create_dir_all(parent).map_err(|e| AgentError::ToolExecution(format!(
                "Failed to create directory {}: {}", parent.display(), e
            )))?;
        }
        
        // Write the content
        fs::write(&validated_path, content).map_err(|e| AgentError::ToolExecution(format!(
            "Failed to write file {}: {}", path.display(), e
        )))
    }
    
    /// Get user confirmation for a modification
    fn get_confirmation(&self, operation: &str, path: &Path) -> AgentResult<bool> {
        if !self.require_confirmation {
            return Ok(true);
        }
        
        // In a real implementation, this would prompt the user interactively
        // For this implementation, we'll just log and approve
        log::info!("Operation: {} on file {}", operation, path.display());
        log::info!("Auto-approving operation");
        
        Ok(true)
    }
}

impl Tool for CodeModificationTool {
    fn name(&self) -> &str {
        "code_modification"
    }
    
    fn description(&self) -> &str {
        "Modifies code files safely with validation"
    }
    
    fn execute(&self, args: &ToolArgs) -> AgentResult<ToolOutput> {
        let operation = &args.command;
        
        // Get operation parameters
        let file_path = match args.parameters.get("file_path") {
            Some(p) if p.is_string() => PathBuf::from(p.as_str().unwrap()),
            _ => return Err(AgentError::ToolExecution("Missing or invalid file_path parameter".to_string())),
        };
        
        match operation {
            op if op == "read" => {
                // Read file content
                let content = self.read_file(&file_path)?;
                
                // In a real implementation, we would update the shared context
                // For now, just log the operation
                log::info!("Read file: {} (size: {} bytes)", file_path.display(), content.len());
                
                Ok(ToolOutput {
                    success: true,
                    result: json!({
                        "content": content,
                        "file_path": file_path.to_string_lossy(),
                        "size": content.len()
                    }),
                    message: None,
                    artifacts: HashMap::new(),
                })
            },
            
            op if op == "write" || op == "create" => {
                let is_create = op == "create";
                
                // Get content to write
                let content = match args.parameters.get("content") {
                    Some(c) if c.is_string() => c.as_str().unwrap().to_string(),
                    _ => return Err(AgentError::ToolExecution("Missing or invalid content parameter".to_string())),
                };
                
                // Check if file exists
                let file_exists = self.validate_path(&file_path)?.exists();
                
                if file_exists && is_create {
                    return Err(AgentError::ToolExecution(format!(
                        "File {} already exists", file_path.display()
                    )));
                }
                
                // Get confirmation
                let operation_name = if is_create { "create" } else { "write" };
                if !self.get_confirmation(operation_name, &file_path)? {
                    return Err(AgentError::ToolExecution("Operation cancelled by user".to_string()));
                }
                
                // Write the file
                self.write_file(&file_path, &content)?;
                
                // In a real implementation, we would update the shared context
                // For now, just log the operation
                log::info!("{} file: {} (size: {} bytes)", 
                    if is_create { "Created" } else { "Wrote" },
                    file_path.display(), 
                    content.len()
                );
                
                Ok(ToolOutput {
                    success: true,
                    result: json!({
                        "operation": operation_name,
                        "file_path": file_path.to_string_lossy(),
                        "size": content.len()
                    }),
                    message: None,
                    artifacts: HashMap::new(),
                })
            },
            
            op if op == "modify" => {
                // Get the original content
                let original_content = self.read_file(&file_path)?;
                
                // Get the search and replace strings
                let search = match args.parameters.get("search") {
                    Some(s) if s.is_string() => s.as_str().unwrap().to_string(),
                    _ => return Err(AgentError::ToolExecution("Missing or invalid search parameter".to_string())),
                };
                
                let replace = match args.parameters.get("replace") {
                    Some(r) if r.is_string() => r.as_str().unwrap().to_string(),
                    _ => return Err(AgentError::ToolExecution("Missing or invalid replace parameter".to_string())),
                };
                
                // Check if search string exists in the file
                if !original_content.contains(&search) {
                    return Err(AgentError::ToolExecution(format!(
                        "Search string not found in file {}", file_path.display()
                    )));
                }
                
                // Perform the replacement
                let new_content = original_content.replace(&search, &replace);
                
                // Get confirmation
                if !self.get_confirmation("modify", &file_path)? {
                    return Err(AgentError::ToolExecution("Operation cancelled by user".to_string()));
                }
                
                // Write the file
                self.write_file(&file_path, &new_content)?;
                
                // In a real implementation, we would update the shared context
                // For now, just log the operation
                log::info!("Modified file: {} (new size: {} bytes)", file_path.display(), new_content.len());
                
                Ok(ToolOutput {
                    success: true,
                    result: json!({
                        "operation": "modify",
                        "file_path": file_path.to_string_lossy(),
                        "size": new_content.len(),
                        "changes": 1
                    }),
                    message: None,
                    artifacts: HashMap::new(),
                })
            },
            
            op if op == "delete" => {
                // Validate path
                let validated_path = self.validate_path(&file_path)?;
                
                // Check if file exists
                if !validated_path.exists() {
                    return Err(AgentError::ToolExecution(format!(
                        "File {} does not exist", file_path.display()
                    )));
                }
                
                // Get confirmation
                if !self.get_confirmation("delete", &file_path)? {
                    return Err(AgentError::ToolExecution("Operation cancelled by user".to_string()));
                }
                
                // Delete the file
                fs::remove_file(&validated_path).map_err(|e| AgentError::ToolExecution(format!(
                    "Failed to delete file {}: {}", file_path.display(), e
                )))?;
                
                // In a real implementation, we would update the shared context
                // For now, just log the operation
                log::info!("Deleted file: {}", file_path.display());
                
                Ok(ToolOutput {
                    success: true,
                    result: json!({
                        "operation": "delete",
                        "file_path": file_path.to_string_lossy()
                    }),
                    message: None,
                    artifacts: HashMap::new(),
                })
            },
            
            _ => Err(AgentError::ToolExecution(format!(
                "Unknown operation: {}", operation
            ))),
        }
    }
    
    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::CodeModification,
            Capability::FileSystemAccess,
        ]
    }
}