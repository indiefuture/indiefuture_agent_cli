use crate::codebase::parser::{FileInfo, detect_language};
use crate::error::{AgentError, AgentResult};
use crate::utils;
use ignore::{DirEntry, Walk, WalkBuilder};
use regex::Regex;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Responsible for scanning and finding relevant code files
pub struct CodeScanner {
    root_path: PathBuf,
    ignore_patterns: Vec<String>,
    supported_extensions: Vec<String>,
}

impl CodeScanner {
    pub fn new(
        scan_path: &Path,
        ignore_patterns: Vec<String>,
        supported_extensions: Vec<String>,
    ) -> Self {
        Self {
            root_path: scan_path.to_path_buf(),
            ignore_patterns,
            supported_extensions,
        }
    }

    /// Get the root path being scanned
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// Find files that might be relevant to a query
    pub async fn find_relevant_files(&self, query: &str) -> AgentResult<Vec<FileInfo>> {
        // Create regex patterns from query terms
        let terms: Vec<&str> = query
            .split_whitespace()
            .filter(|&term| term.len() >= 3)
            .collect();

        let patterns: Vec<Regex> = terms
            .iter()
            .filter_map(|&term| Regex::new(&format!("(?i){}", regex::escape(term))).ok())
            .collect();

        if patterns.is_empty() {
            return Ok(Vec::new());
        }

        // Find files that match the patterns
        let mut matches = Vec::new();
        let walker = self.create_walker()?;

        for entry in walker {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                continue;
            }

            let path = entry.path();
            if !self.is_supported_file(path) {
                continue;
            }

            // Read file content
            let content = match fs::read_to_string(path).await {
                Ok(content) => content,
                Err(_) => continue,
            };

            // Check if content matches any pattern
            let mut relevance_score = 0;
            for pattern in &patterns {
                if pattern.is_match(&content) {
                    relevance_score += 1;
                }
            }

            if relevance_score > 0 {
                let language = detect_language(path);
                let relative_path = pathdiff::diff_paths(path, &self.root_path)
                    .unwrap_or_else(|| path.to_path_buf());

                matches.push(FileInfo {
                    path: relative_path.to_string_lossy().to_string(),
                    content,
                    language,
                    relevance: relevance_score,
                });
            }
        }

        // Sort by relevance
        matches.sort_by(|a, b| b.relevance.cmp(&a.relevance));

        // Limit to top 5 most relevant files
        let top_matches = matches.into_iter().take(5).collect();
        Ok(top_matches)
    }

    /// Scan a single file
    pub async fn scan_file(&self, path: &Path) -> AgentResult<Option<FileInfo>> {
        if !path.is_file() {
            return Ok(None);
        }

        if !self.is_supported_file(path) {
            return Ok(None);
        }

        let content = fs::read_to_string(path)
            .await
            .map_err(|e| AgentError::Io(e))?;

        let language = detect_language(path);
        let relative_path = pathdiff::diff_paths(path, &self.root_path)
            .unwrap_or_else(|| path.to_path_buf());

        Ok(Some(FileInfo {
            path: relative_path.to_string_lossy().to_string(),
            content,
            language,
            relevance: 1,
        }))
    }

    /// Scan a directory for all supported files
    pub async fn scan_directory(&self, path: &Path) -> AgentResult<Vec<FileInfo>> {
        let mut files = Vec::new();
        let walker = WalkBuilder::new(path)
            .hidden(false)
            .build();

        for entry in walker {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                continue;
            }

            let file_path = entry.path();
            if !self.is_supported_file(file_path) {
                continue;
            }

            if let Ok(Some(file_info)) = self.scan_file(file_path).await {
                files.push(file_info);
            }
        }

        Ok(files)
    }

    /// Create a file walker with appropriate ignore patterns
    fn create_walker(&self) -> AgentResult<Walk> {
        let mut builder = WalkBuilder::new(&self.root_path);
        builder.hidden(false);

        // Add custom ignore patterns
        for pattern in &self.ignore_patterns {
            builder.add_custom_ignore_filename(pattern);
        }

        Ok(builder.build())
    }

    /// Check if a file has a supported extension
    fn is_supported_file(&self, path: &Path) -> bool {
        utils::is_supported_extension(path, &self.supported_extensions)
    }
}