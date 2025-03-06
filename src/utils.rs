use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Generate a unique ID for tasks, entries, etc.
pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

/// Get current timestamp in ISO format
pub fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

/// Check if a given file extension is supported
pub fn is_supported_extension(path: &std::path::Path, supported_extensions: &[String]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| supported_extensions.contains(&ext.to_lowercase()))
        .unwrap_or(false)
}

/// Truncate a string to a maximum length, adding ellipsis if truncated
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[0..max_len - 3])
    }
}

/// Convert a file path to a string, handling error cases
pub fn path_to_string(path: &std::path::Path) -> String {
    path.to_string_lossy().to_string()
}

/// Check if a string is likely to be binary content
pub fn is_likely_binary(content: &str) -> bool {
    // Count null bytes and non-UTF8 characters as indicators of binary content
    let null_byte_count = content.chars().filter(|&c| c == '\0').count();
    let total_chars = content.len();

    // If more than 1% of the content is null bytes, consider it binary
    if total_chars > 0 && (null_byte_count as f64 / total_chars as f64) > 0.01 {
        return true;
    }

    // Check for common binary file signatures
    if content.starts_with("\u{1f}\u{8b}") || // gzip
       content.starts_with("PK\u{3}\u{4}") || // zip
       content.starts_with("\u{89}PNG") || // png
       content.starts_with("GIF8") || // gif
       content.starts_with("\u{ff}\u{d8}\u{ff}")
    // jpeg
    {
        return true;
    }

    false
}

/// Format a time duration in a human-readable format
pub fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!(
            "{}h {}m {}s",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
    }
}
