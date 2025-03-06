use log::info;

#[derive(Default)]
pub struct ContextMemory {
    fragments: Vec<MemoryFragment>,
}

impl ContextMemory {
    pub fn add_frag(&mut self, frag: MemoryFragment) {
        // Log the fragment that was added
        info!("📝 Adding memory fragment to context:");
        info!("  Source: {}", frag.source);

        // Log a preview of the content (first 100 chars)
        let preview = if frag.content.len() > 100 {
            format!("{}...", &frag.content[0..100])
        } else {
            frag.content.clone()
        };
        info!("  Content: {}", preview);

        // Log metadata if available
        if let Some(metadata) = &frag.metadata {
            if let Some(path) = &metadata.path {
                info!("  Path: {}", path);
            }
            if let Some(file_type) = &metadata.file_type {
                info!("  Type: {}", file_type);
            }
            if !metadata.tags.is_empty() {
                info!("  Tags: {}", metadata.tags.join(", "));
            }
        }

        // Actually store the fragment
        self.fragments.push(frag);

        info!(
            "✅ Memory fragment added successfully (total: {})",
            self.fragments.len()
        );
    }

    // Get all fragments
    pub fn get_fragments(&self) -> &Vec<MemoryFragment> {
        &self.fragments
    }

    // Get the total number of fragments
    pub fn fragment_count(&self) -> usize {
        self.fragments.len()
    }

    // Clear all fragments
    pub fn clear(&mut self) {
        self.fragments.clear();
    }
}

#[derive(Debug, Clone)]
pub struct MemoryFragment {
    pub source: String, // Where the data came from (e.g., "glob search", "file content")
    pub content: String, // The actual content/data
    pub metadata: Option<MemoryMetadata>, // Additional metadata
}

#[derive(Debug, Clone)]
pub struct MemoryMetadata {
    pub file_type: Option<String>, // E.g., "directory", "file", etc.
    pub path: Option<String>,      // File or directory path if applicable
    pub timestamp: Option<i64>,    // Unix timestamp when the data was captured
    pub tags: Vec<String>,         // Optional tags for categorization
}
