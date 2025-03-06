use std::path::Path;

/// Information about a parsed code file
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String,
    pub content: String,
    pub language: Option<String>,
    pub relevance: usize,
}

/// Detects the programming language of a file based on extension
pub fn detect_language(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| match ext.to_lowercase().as_str() {
            "rs" => Some("rust".to_string()),
            "ts" | "tsx" => Some("typescript".to_string()),
            "js" | "jsx" => Some("javascript".to_string()),
            "py" => Some("python".to_string()),
            "go" => Some("go".to_string()),
            "java" => Some("java".to_string()),
            "c" | "h" => Some("c".to_string()),
            "cpp" | "hpp" | "cc" => Some("cpp".to_string()),
            "json" => Some("json".to_string()),
            "yaml" | "yml" => Some("yaml".to_string()),
            "toml" => Some("toml".to_string()),
            "md" => Some("markdown".to_string()),
            "sh" => Some("shell".to_string()),
            "html" | "htm" => Some("html".to_string()),
            "css" => Some("css".to_string()),
            "sql" => Some("sql".to_string()),
            _ => None,
        })
        .flatten()
}

/// Parses a code file and extracts important structures
/// This is a simple implementation for the MVP
/// A more sophisticated version would use actual parsers for each language
pub fn parse_file(content: &str, language: Option<&str>) -> Vec<CodeStructure> {
    let mut structures = Vec::new();

    // Skip parsing if no language is specified
    let language = match language {
        Some(lang) => lang,
        None => return structures,
    };

    // Split the content into lines for analysis
    let lines: Vec<&str> = content.lines().collect();

    match language {
        "rust" => parse_rust(&lines, &mut structures),
        "typescript" | "javascript" => parse_js_ts(&lines, &mut structures),
        "python" => parse_python(&lines, &mut structures),
        // Add more language parsers as needed
        _ => {} // Unsupported language, return empty structures
    }

    structures
}

/// Extract Rust code structures
fn parse_rust(lines: &[&str], structures: &mut Vec<CodeStructure>) {
    let mut in_struct = false;
    let mut in_impl = false;
    let mut in_fn = false;
    let mut struct_name = String::new();
    let mut impl_name = String::new();
    let mut fn_name = String::new();
    let mut start_line = 0;

    for (i, &line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Look for struct definitions
        if trimmed.starts_with("struct ") && trimmed.contains("{") {
            if let Some(name) = extract_name(trimmed, "struct ") {
                in_struct = true;
                struct_name = name;
                start_line = i;
            }
        }
        // Look for struct end
        else if in_struct && trimmed == "}" {
            structures.push(CodeStructure {
                kind: "struct".to_string(),
                name: struct_name.clone(),
                line_start: start_line,
                line_end: i,
            });
            in_struct = false;
        }
        // Look for impl blocks
        else if trimmed.starts_with("impl ") && trimmed.contains("{") {
            if let Some(name) = extract_name(trimmed, "impl ") {
                in_impl = true;
                impl_name = name;
                start_line = i;
            }
        }
        // Look for impl end
        else if in_impl && trimmed == "}" {
            structures.push(CodeStructure {
                kind: "impl".to_string(),
                name: impl_name.clone(),
                line_start: start_line,
                line_end: i,
            });
            in_impl = false;
        }
        // Look for functions
        else if trimmed.starts_with("fn ") && trimmed.contains("(") {
            if let Some(name) = extract_name(trimmed, "fn ") {
                in_fn = true;
                fn_name = name;
                start_line = i;
            }
        }
        // Look for function end - simplistic approach
        else if in_fn && trimmed == "}" {
            structures.push(CodeStructure {
                kind: "function".to_string(),
                name: fn_name.clone(),
                line_start: start_line,
                line_end: i,
            });
            in_fn = false;
        }
    }
}

/// Extract JavaScript/TypeScript code structures
fn parse_js_ts(lines: &[&str], structures: &mut Vec<CodeStructure>) {
    let mut in_class = false;
    let mut in_function = false;
    let mut class_name = String::new();
    let mut fn_name = String::new();
    let mut start_line = 0;

    for (i, &line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Look for class definitions
        if (trimmed.starts_with("class ") || trimmed.starts_with("export class "))
            && trimmed.contains("{")
        {
            let search_term = if trimmed.starts_with("export") {
                "export class "
            } else {
                "class "
            };
            if let Some(name) = extract_name(trimmed, search_term) {
                in_class = true;
                class_name = name;
                start_line = i;
            }
        }
        // Look for class end
        else if in_class && trimmed == "}" {
            structures.push(CodeStructure {
                kind: "class".to_string(),
                name: class_name.clone(),
                line_start: start_line,
                line_end: i,
            });
            in_class = false;
        }
        // Look for functions/methods
        else if (trimmed.starts_with("function ")
            || trimmed.starts_with("const ") && trimmed.contains(" = function"))
            && trimmed.contains("(")
        {
            if let Some(name) = extract_function_name_js(trimmed) {
                in_function = true;
                fn_name = name;
                start_line = i;
            }
        }
        // Look for function end - simplistic approach
        else if in_function && trimmed == "}" {
            structures.push(CodeStructure {
                kind: "function".to_string(),
                name: fn_name.clone(),
                line_start: start_line,
                line_end: i,
            });
            in_function = false;
        }
    }
}

/// Extract Python code structures
fn parse_python(lines: &[&str], structures: &mut Vec<CodeStructure>) {
    let mut in_class = false;
    let mut in_function = false;
    let mut class_name = String::new();
    let mut fn_name = String::new();
    let mut start_line = 0;

    for (i, &line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Look for class definitions
        if trimmed.starts_with("class ") && (trimmed.contains(":") || trimmed.contains("(")) {
            if let Some(name) = extract_name(trimmed, "class ") {
                in_class = true;
                class_name = name;
                start_line = i;
            }
        }
        // Look for function definitions
        else if trimmed.starts_with("def ") && trimmed.contains("(") {
            if let Some(name) = extract_name(trimmed, "def ") {
                in_function = true;
                fn_name = name;
                start_line = i;
            }
        }

        // For Python, we can't reliably detect ends of blocks by syntax alone
        // A more sophisticated implementation would track indentation levels
        // For the MVP, we'll use a simplistic approach based on empty lines and indentation

        let next_line = lines.get(i + 1);
        if in_class
            && next_line.map_or(true, |line| {
                !line.starts_with(" ") && !line.starts_with("\t") && !line.is_empty()
            })
        {
            structures.push(CodeStructure {
                kind: "class".to_string(),
                name: class_name.clone(),
                line_start: start_line,
                line_end: i,
            });
            in_class = false;
        }

        if in_function
            && next_line.map_or(true, |line| {
                !line.starts_with(" ") && !line.starts_with("\t") && !line.is_empty()
            })
        {
            structures.push(CodeStructure {
                kind: "function".to_string(),
                name: fn_name.clone(),
                line_start: start_line,
                line_end: i,
            });
            in_function = false;
        }
    }
}

/// Helper function to extract a name from a definition line
fn extract_name(line: &str, prefix: &str) -> Option<String> {
    let after_prefix = line.trim_start_matches(prefix);
    let name_end =
        after_prefix.find(|c: char| c == ' ' || c == '{' || c == '(' || c == ':' || c == '<');

    if let Some(end) = name_end {
        Some(after_prefix[0..end].to_string())
    } else {
        None
    }
}

/// Helper function to extract JS/TS function names
fn extract_function_name_js(line: &str) -> Option<String> {
    if line.starts_with("function ") {
        return extract_name(line, "function ");
    }

    // Handle arrow functions and function expressions
    if let Some(equals_pos) = line.find(" = ") {
        let before_equals = &line[0..equals_pos];
        let const_pos = before_equals.find("const ");
        let let_pos = before_equals.find("let ");
        let var_pos = before_equals.find("var ");

        let start_pos = const_pos.or(let_pos).or(var_pos);
        if let Some(_pos) = start_pos {
            let declaration_keyword = if const_pos.is_some() {
                "const "
            } else if let_pos.is_some() {
                "let "
            } else {
                "var "
            };

            return extract_name(before_equals, declaration_keyword);
        }
    }

    None
}

/// Represents a code structure like a class, function, etc.
#[derive(Debug, Clone)]
pub struct CodeStructure {
    pub kind: String,      // "class", "function", "struct", etc.
    pub name: String,      // Name of the structure
    pub line_start: usize, // Starting line number
    pub line_end: usize,   // Ending line number
}

/// Summarizes a code file for easier understanding
pub fn summarize_file(file_info: &FileInfo) -> String {
    let structures = match &file_info.language {
        Some(lang) => parse_file(&file_info.content, Some(lang)),
        None => Vec::new(),
    };

    if structures.is_empty() {
        return format!("File: {}\nNo parseable structures found.", file_info.path);
    }

    let mut summary = format!("File: {}\n", file_info.path);
    summary.push_str("Structures:\n");

    for structure in structures {
        summary.push_str(&format!(
            "- {} '{}' (lines {}-{})\n",
            structure.kind, structure.name, structure.line_start, structure.line_end
        ));
    }

    summary
}
