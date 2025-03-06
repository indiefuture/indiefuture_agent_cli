pub mod parser;
pub mod scanner;

use crate::error::AgentResult;
use parser::FileInfo;
use scanner::CodeScanner;
use std::sync::Arc;

/// Manages code scanning and parsing
pub struct CodebaseScanner {
    scanner: Arc<CodeScanner>,
}

impl CodebaseScanner {
    pub fn new(
        scan_path: &std::path::Path,
        ignore_patterns: Vec<String>,
        supported_extensions: Vec<String>,
    ) -> Self {
        Self {
            scanner: Arc::new(CodeScanner::new(
                scan_path,
                ignore_patterns,
                supported_extensions,
            )),
        }
    }

    /// Find files relevant to a given query
    pub async fn find_relevant_files(&self, query: &str) -> AgentResult<Vec<FileInfo>> {
        self.scanner.find_relevant_files(query).await
    }

    /// Scan a file by path
    pub async fn scan_file(&self, path: &std::path::Path) -> AgentResult<Option<FileInfo>> {
        self.scanner.scan_file(path).await
    }

    /// Scan a specific directory
    pub async fn scan_directory(&self, path: &std::path::Path) -> AgentResult<Vec<FileInfo>> {
        self.scanner.scan_directory(path).await
    }

    /// Get the root path of the scanner
    pub fn root_path(&self) -> &std::path::Path {
        self.scanner.root_path()
    }
}
