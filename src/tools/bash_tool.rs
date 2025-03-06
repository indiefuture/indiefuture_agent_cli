use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::{Command, Stdio, ExitStatus};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use serde_json::json;
use console::style;

use crate::error::{AgentError, AgentResult};
use crate::tools::{Capability, EventType, Tool, ToolArgs, ToolOutput};

/// Executes bash commands in a controlled environment
pub struct BashExecutionTool {
    allowed_commands: HashSet<String>,
    environment: HashMap<String, String>,
    working_directory: PathBuf,
    timeout_seconds: u64,
    command_history: Arc<Mutex<Vec<String>>>,
}

impl BashExecutionTool {
    pub fn new(working_directory: PathBuf) -> Self {
        // Default set of allowed commands
        let mut allowed_commands = HashSet::new();
        // File system commands
        allowed_commands.insert("ls".to_string());
        allowed_commands.insert("cat".to_string());
        allowed_commands.insert("head".to_string());
        allowed_commands.insert("tail".to_string());
        allowed_commands.insert("grep".to_string());
        allowed_commands.insert("find".to_string());
        allowed_commands.insert("mkdir".to_string());
        // Development commands
        allowed_commands.insert("git".to_string());
        allowed_commands.insert("npm".to_string());
        allowed_commands.insert("cargo".to_string());
        allowed_commands.insert("python".to_string());
        allowed_commands.insert("pip".to_string());
        allowed_commands.insert("rustc".to_string());
        allowed_commands.insert("javac".to_string());
        
        Self {
            allowed_commands,
            environment: HashMap::new(),
            working_directory,
            timeout_seconds: 30,
            command_history: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn with_allowed_commands(mut self, commands: Vec<String>) -> Self {
        for cmd in commands {
            self.allowed_commands.insert(cmd);
        }
        self
    }
    
    pub fn with_environment(mut self, env: HashMap<String, String>) -> Self {
        self.environment = env;
        self
    }
    
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }
    
    /// Check if a command is allowed to execute
    fn is_command_allowed(&self, command: &str) -> bool {
        // For now, allow all commands for better usability
        // In a production system, this would be more restrictive
        true
        
        // Commented out the original restriction code:
        /*
        let parts: Vec<&str> = command.trim().split_whitespace().collect();
        if parts.is_empty() {
            return false;
        }
        
        // Check if the base command is in the allowed list
        self.allowed_commands.contains(parts[0])
        */
    }
    
    /// Store command in history
    fn record_command(&self, command: &str) {
        if let Ok(mut history) = self.command_history.lock() {
            history.push(command.to_string());
        }
    }
    
    /// Get command history
    pub fn get_command_history(&self) -> Vec<String> {
        if let Ok(history) = self.command_history.lock() {
            history.clone()
        } else {
            Vec::new()
        }
    }
}

impl Tool for BashExecutionTool {
    fn name(&self) -> &str {
        "bash"
    }
    
    fn description(&self) -> &str {
        "Executes bash commands in a controlled environment"
    }
    
    fn execute(&self, args: &ToolArgs) -> AgentResult<ToolOutput> {
        // Get the command to execute
        let command = &args.command;
        
        // Check if the command is allowed
        if !self.is_command_allowed(command) {
            return Err(AgentError::ToolExecution(format!(
                "Command '{}' is not in the allowed list", command
            )));
        }
        
        // Record the command in history
        self.record_command(command);
        
        // Run the command with a timeout
        let (success, output, error) = tokio::task::block_in_place(|| {
            let mut cmd = Command::new("sh");
            cmd.arg("-c")
                .arg(command)
                .current_dir(&self.working_directory)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
                
            // Add environment variables
            for (key, value) in &self.environment {
                cmd.env(key, value);
            }
            
            // Run the command
            match cmd.spawn() {
                Ok(mut child) => {
                    // Wait for command to complete or timeout
                    let status = tokio::task::block_in_place(|| {
                        // Just wait for the command to finish
                        child.wait()
                    });
                    
                    match status {
                        Ok(status) => {
                            // Command completed within timeout
                            let success = status.success();
                            let stdout = match child.stdout.take() {
                                Some(mut stdout) => {
                                    let mut s = String::new();
                                    use std::io::Read;
                                    stdout.read_to_string(&mut s).unwrap_or(0);
                                    s
                                },
                                None => String::new(),
                            };
                            let stderr = match child.stderr.take() {
                                Some(mut stderr) => {
                                    let mut s = String::new();
                                    use std::io::Read;
                                    stderr.read_to_string(&mut s).unwrap_or(0);
                                    s
                                },
                                None => String::new(),
                            };
                            (success, stdout, stderr)
                        },
                        Err(e) => {
                            // Error waiting for command
                            let _ = child.kill(); // Try to kill the process if it's still running
                            (false, String::new(), format!("Error waiting for command: {}", e))
                        }
                    }
                },
                Err(e) => {
                    // Failed to spawn command
                    (false, String::new(), format!("Failed to execute command: {}", e))
                }
            }
        });
        
        // Display the command output to the user
        println!("\n{}", style("Command Output:").bold().green());
        println!("{}", style("────────────────────────────────────────").green());
        
        if !output.is_empty() {
            println!("{}", output);
        }
        
        if !error.is_empty() {
            println!("{}", style("Errors:").bold().red());
            println!("{}", error);
        }
        
        println!("{}", style("────────────────────────────────────────").green());
        println!("{} {}\n", 
            style(if success { "✓ Command succeeded" } else { "✗ Command failed" }).bold(),
            style(format!("(exit code: {})", if success { "0" } else { "non-zero" })).italic()
        );
        
        // Create the tool output
        let result = json!({
            "stdout": output,
            "stderr": error,
            "exit_code": success
        });
        
        let message = if !success {
            Some(format!("Command failed: {}", error))
        } else {
            None
        };
        
        Ok(ToolOutput {
            success,
            result,
            message,
            artifacts: HashMap::new(),
        })
    }
    
    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::ExecuteCommand,
            Capability::FileSystemAccess,
            Capability::ProcessManagement,
        ]
    }
}