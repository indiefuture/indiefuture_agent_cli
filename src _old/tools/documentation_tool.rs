use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde_json::{json, Value};

use crate::error::{AgentError, AgentResult};
use crate::tools::{Capability, EventType, Tool, ToolArgs, ToolOutput};

/// Tool for generating and updating documentation
pub struct DocumentationTool {
    docs_directory: PathBuf,
}

impl DocumentationTool {
    pub fn new(docs_directory: PathBuf) -> Self {
        Self {
            docs_directory,
        }
    }
    
    /// Validate file path is within docs directory
    fn validate_path(&self, path: &Path) -> AgentResult<PathBuf> {
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.docs_directory.join(path)
        };
        
        // Check that the path is inside the docs directory
        match absolute_path.strip_prefix(&self.docs_directory) {
            Ok(_) => Ok(absolute_path),
            Err(_) => Err(AgentError::ToolExecution(format!(
                "Path {} is outside the docs directory", path.display()
            ))),
        }
    }
    
    /// Create or update a markdown file
    fn write_markdown(&self, path: &Path, content: &str) -> AgentResult<()> {
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
    
    /// Read a markdown file
    fn read_markdown(&self, path: &Path) -> AgentResult<String> {
        let validated_path = self.validate_path(path)?;
        
        fs::read_to_string(&validated_path)
            .map_err(|e| AgentError::ToolExecution(format!(
                "Failed to read file {}: {}", path.display(), e
            )))
    }
    
    /// List markdown files in a directory
    fn list_markdown_files(&self, subdir: Option<&Path>) -> AgentResult<Vec<PathBuf>> {
        let dir = match subdir {
            Some(path) => self.validate_path(path)?,
            None => self.docs_directory.clone(),
        };
        
        if !dir.exists() || !dir.is_dir() {
            return Err(AgentError::ToolExecution(format!(
                "Directory {} does not exist", dir.display()
            )));
        }
        
        let mut files = Vec::new();
        for entry in fs::read_dir(dir).map_err(|e| AgentError::ToolExecution(format!(
            "Failed to read directory: {}", e
        )))? {
            let entry = entry.map_err(|e| AgentError::ToolExecution(format!(
                "Failed to read directory entry: {}", e
            )))?;
            
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                files.push(path);
            }
        }
        
        Ok(files)
    }
}

impl Tool for DocumentationTool {
    fn name(&self) -> &str {
        "documentation"
    }
    
    fn description(&self) -> &str {
        "Generates and updates documentation files"
    }
    
    fn execute(&self, args: &ToolArgs) -> AgentResult<ToolOutput> {
        let operation = &args.command;
        
        match operation {
            op if op == "create" || op == "update" => {
                // Get parameters
                let file_path = match args.parameters.get("file_path") {
                    Some(p) if p.is_string() => PathBuf::from(p.as_str().unwrap()),
                    _ => return Err(AgentError::ToolExecution("Missing or invalid file_path parameter".to_string())),
                };
                
                let content = match args.parameters.get("content") {
                    Some(c) if c.is_string() => c.as_str().unwrap().to_string(),
                    _ => return Err(AgentError::ToolExecution("Missing or invalid content parameter".to_string())),
                };
                
                // Check file extension is .md
                if file_path.extension().map_or(true, |ext| ext != "md") {
                    return Err(AgentError::ToolExecution(
                        "File must have .md extension".to_string()
                    ));
                }
                
                // Create or update the file
                self.write_markdown(&file_path, &content)?;
                
                // In a real implementation, we would update the shared context
                // For now, just log the operation
                log::info!("{} documentation: {} (size: {} bytes)", 
                    if op == "create" { "Created" } else { "Updated" },
                    file_path.display(),
                    content.len()
                );
                
                Ok(ToolOutput {
                    success: true,
                    result: json!({
                        "operation": op,
                        "file_path": file_path.to_string_lossy(),
                        "size": content.len()
                    }),
                    message: None,
                    artifacts: HashMap::new(),
                })
            },
            
            op if op == "read" => {
                // Get file path
                let file_path = match args.parameters.get("file_path") {
                    Some(p) if p.is_string() => PathBuf::from(p.as_str().unwrap()),
                    _ => return Err(AgentError::ToolExecution("Missing or invalid file_path parameter".to_string())),
                };
                
                // Read the file
                let content = self.read_markdown(&file_path)?;
                
                // In a real implementation, we would update the shared context
                // For now, just log the operation
                log::info!("Read documentation: {} (size: {} bytes)", file_path.display(), content.len());
                
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
            
            op if op == "list" => {
                // Get optional subdirectory
                let subdir = args.parameters.get("subdir").and_then(|s| {
                    s.as_str().map(|s| Path::new(s))
                });
                
                // List markdown files
                let files = self.list_markdown_files(subdir)?;
                
                // Convert paths to relative paths
                let relative_paths: Vec<String> = files.iter()
                    .filter_map(|path| {
                        path.strip_prefix(&self.docs_directory).ok()
                            .map(|rel_path| rel_path.to_string_lossy().to_string())
                    })
                    .collect();
                
                // In a real implementation, we would update the shared context
                // For now, just log the operation
                log::info!("Listed documentation files in {} (found {} files)", 
                    subdir.map_or("docs directory".to_string(), |p| p.display().to_string()),
                    relative_paths.len()
                );
                
                Ok(ToolOutput {
                    success: true,
                    result: json!({
                        "files": relative_paths,
                        "count": relative_paths.len()
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
        vec![Capability::DocumentationGeneration]
    }
}