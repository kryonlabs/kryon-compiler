//! Preprocessor for handling @include directives and file inclusion

use crate::error::{CompilerError, Result, SourceMap};
use crate::types::MAX_INCLUDE_DEPTH;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Preprocessor {
    included_files: HashSet<PathBuf>,
    current_depth: usize,
    source_map: SourceMap,
    combined_line_count: usize,
}

impl Preprocessor {
    pub fn new() -> Self {
        Self {
            included_files: HashSet::new(),
            current_depth: 0,
            source_map: SourceMap::new(),
            combined_line_count: 1,
        }
    }

    /// Process @include directives recursively and return combined content with source map
    pub fn process_includes(&mut self, file_path: &str) -> Result<(String, SourceMap)> {
        let content = self.process_includes_recursive(file_path, 0)?;
        Ok((content, self.source_map.clone()))
    }

    fn process_includes_recursive(&mut self, file_path: &str, depth: usize) -> Result<String> {
        if depth > MAX_INCLUDE_DEPTH {
            return Err(CompilerError::Include {
                message: format!("Maximum include depth ({}) exceeded processing '{}'", 
                                MAX_INCLUDE_DEPTH, file_path),
            });
        }

        // Resolve to absolute path to prevent circular includes
        let canonical_path = fs::canonicalize(file_path).map_err(|e| {
            CompilerError::FileNotFound {
                path: format!("{}: {}", file_path, e),
            }
        })?;

        // Check for circular includes
        if self.included_files.contains(&canonical_path) {
            return Err(CompilerError::Include {
                message: format!("Circular include detected: '{}'", file_path),
            });
        }

        self.included_files.insert(canonical_path.clone());

        // Read the file
        let content = fs::read_to_string(&canonical_path).map_err(|e| {
            CompilerError::FileNotFound {
                path: format!("{}: {}", file_path, e),
            }
        })?;

        // Process includes in this file
        let base_dir = canonical_path.parent().unwrap_or(Path::new(""));
        let processed_content = self.process_content(&content, base_dir, file_path, depth)?;

        self.included_files.remove(&canonical_path);
        Ok(processed_content)
    }

    fn process_content(&mut self, content: &str, base_dir: &Path, current_file: &str, depth: usize) -> Result<String> {
        let mut result = String::new();
        let mut line_num = 0;
        let lines: Vec<&str> = content.lines().collect();

        for line in &lines {
            line_num += 1;
            let trimmed = line.trim_start();

            if let Some(include_path) = self.parse_include_line(trimmed, line_num)? {
                // Resolve include path relative to current file
                let full_include_path = if Path::new(&include_path).is_absolute() {
                    include_path.clone()
                } else {
                    base_dir.join(&include_path).to_string_lossy().to_string()
                };

                log::debug!("Processing include: {} -> {}", include_path, full_include_path);

                // Recursively process the included file
                match self.process_includes_recursive(&full_include_path, depth + 1) {
                    Ok(included_content) => {
                        // Add source mapping for each line from the included file
                        for (i, _) in included_content.lines().enumerate() {
                            self.source_map.add_line_mapping(
                                self.combined_line_count + i + 1, 
                                &full_include_path, 
                                i + 1
                            );
                        }
                        
                        // Update combined line count for the included content
                        let included_line_count = included_content.lines().count();
                        self.combined_line_count += included_line_count;
                        
                        result.push_str(&included_content);
                        if !included_content.ends_with('\n') && !included_content.is_empty() {
                            result.push('\n');
                            self.combined_line_count += 1;
                        }
                    }
                    Err(e) => {
                        return Err(CompilerError::Include {
                            message: format!("Error processing include '{}' at line {}: {}", 
                                           include_path, line_num, e),
                        });
                    }
                }
            } else {
                // Regular line - add to source mapping and result
                self.source_map.add_line_mapping(self.combined_line_count, current_file, line_num);
                result.push_str(line);
                result.push('\n');
                self.combined_line_count += 1;
            }
        }

        Ok(result)
    }

    /// Parse an @include line and extract the file path
    /// Returns None if this isn't a valid include line
    fn parse_include_line(&self, line: &str, line_num: usize) -> Result<Option<String>> {
        if !line.starts_with("@include") {
            return Ok(None);
        }

        let after_include = line[8..].trim_start(); // Skip "@include"

        // Must start with quote
        if !after_include.starts_with('"') {
            return Err(CompilerError::Parse {
                file: "<unknown>".to_string(),
                line: line_num,
                message: "Invalid @include syntax: path must be quoted".to_string(),
            });
        }

        // Find closing quote
        let mut end_quote_pos = None;
        let mut escaped = false;
        
        for (i, ch) in after_include[1..].char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                end_quote_pos = Some(i + 1);
                break;
            }
        }

        let end_quote_pos = end_quote_pos.ok_or_else(|| CompilerError::Parse {
            file: "<unknown>".to_string(),
            line: line_num,
            message: "Invalid @include syntax: missing closing quote".to_string(),
        })?;

        let path = &after_include[1..end_quote_pos];
        
        // Check what comes after the closing quote
        let after_quote = after_include[end_quote_pos + 1..].trim_start();
        
        // Should be empty or a comment
        if !after_quote.is_empty() && !after_quote.starts_with('#') {
            return Err(CompilerError::Parse {
                file: "<unknown>".to_string(),
                line: line_num,
                message: format!("Invalid @include syntax: unexpected content after path: '{}'", 
                               after_quote),
            });
        }

        if path.is_empty() {
            return Err(CompilerError::Parse {
                file: "<unknown>".to_string(),
                line: line_num,
                message: "Invalid @include syntax: path cannot be empty".to_string(),
            });
        }

        // Process escape sequences in the path
        let unescaped_path = self.unescape_string(path)?;
        
        Ok(Some(unescaped_path))
    }

    /// Unescape string literals in include paths
    fn unescape_string(&self, s: &str) -> Result<String> {
        let mut result = String::new();
        let mut chars = s.chars();
        
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some('\'') => result.push('\''),
                    Some(other) => {
                        result.push('\\');
                        result.push(other);
                    }
                    None => result.push('\\'),
                }
            } else {
                result.push(ch);
            }
        }
        
        Ok(result)
    }
}

/// Convenience function to preprocess a file with includes
pub fn preprocess_file(file_path: &str) -> Result<(String, SourceMap)> {
    let mut preprocessor = Preprocessor::new();
    preprocessor.process_includes(file_path)
}

/// Legacy function for backward compatibility
pub fn preprocess_file_legacy(file_path: &str) -> Result<String> {
    let mut preprocessor = Preprocessor::new();
    let (content, _) = preprocessor.process_includes(file_path)?;
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let file_path = dir.path().join(name);
        fs::write(&file_path, content).unwrap();
        file_path
    }

    #[test]
    fn test_basic_include() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create included file
        create_test_file(&temp_dir, "included.kry", "Container { text: \"included\" }");
        
        // Create main file
        let main_content = format!(
            "@include \"{}\"\nApp {{ }}", 
            temp_dir.path().join("included.kry").to_string_lossy()
        );
        let main_file = create_test_file(&temp_dir, "main.kry", &main_content);
        
        let mut preprocessor = Preprocessor::new();
        let result = preprocessor.process_includes(main_file.to_str().unwrap()).unwrap();
        
        assert!(result.0.contains("Container { text: \"included\" }"));
        assert!(result.0.contains("App { }"));
    }

    #[test]
    fn test_relative_include() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create subdirectory
        let sub_dir = temp_dir.path().join("components");
        fs::create_dir(&sub_dir).unwrap();
        
        // Create included file in subdirectory
        create_test_file(&temp_dir, "components/button.kry", 
                        "Button { text: \"Click me\" }");
        
        // Create main file with relative include
        let main_content = r#"@include "components/button.kry"
App { }"#;
        let main_file = create_test_file(&temp_dir, "main.kry", main_content);
        
        let mut preprocessor = Preprocessor::new();
        let result = preprocessor.process_includes(main_file.to_str().unwrap()).unwrap();
        
        assert!(result.0.contains("Button { text: \"Click me\" }"));
        assert!(result.0.contains("App { }"));
    }

    #[test]
    fn test_nested_includes() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create deeply nested include
        create_test_file(&temp_dir, "level3.kry", "Text { text: \"level3\" }");
        
        let level2_content = format!(
            "@include \"{}\"\nText {{ text: \"level2\" }}", 
            temp_dir.path().join("level3.kry").to_string_lossy()
        );
        create_test_file(&temp_dir, "level2.kry", &level2_content);
        
        let level1_content = format!(
            "@include \"{}\"\nText {{ text: \"level1\" }}", 
            temp_dir.path().join("level2.kry").to_string_lossy()
        );
        create_test_file(&temp_dir, "level1.kry", &level1_content);
        
        let main_content = format!(
            "@include \"{}\"\nApp {{ }}", 
            temp_dir.path().join("level1.kry").to_string_lossy()
        );
        let main_file = create_test_file(&temp_dir, "main.kry", &main_content);
        
        let mut preprocessor = Preprocessor::new();
        let result = preprocessor.process_includes(main_file.to_str().unwrap()).unwrap();
        
        assert!(result.0.contains("level3"));
        assert!(result.0.contains("level2"));
        assert!(result.0.contains("level1"));
        assert!(result.0.contains("App { }"));
    }

    #[test]
    fn test_circular_include_detection() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create file1 that includes file2
        let file2_path = temp_dir.path().join("file2.kry");
        let file1_content = format!("@include \"{}\"\nText {{ }}", file2_path.to_string_lossy());
        create_test_file(&temp_dir, "file1.kry", &file1_content);
        
        // Create file2 that includes file1
        let file1_path = temp_dir.path().join("file1.kry");
        let file2_content = format!("@include \"{}\"\nContainer {{ }}", file1_path.to_string_lossy());
        create_test_file(&temp_dir, "file2.kry", &file2_content);
        
        let mut preprocessor = Preprocessor::new();
        let result = preprocessor.process_includes(file1_path.to_str().unwrap());
        
        assert!(result.is_err());
        if let Err(CompilerError::Include { message }) = result {
            assert!(message.contains("Circular include"));
        } else {
            panic!("Expected circular include error");
        }
    }

    #[test]
    fn test_max_depth_exceeded() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create a chain of includes that exceeds max depth
        let mut prev_file = None;
        for i in 0..=MAX_INCLUDE_DEPTH + 1 {
            let filename = format!("file{}.kry", i);
            let content = if let Some(prev) = prev_file {
                format!("@include \"{}\"\nText {{ text: \"{}\" }}", prev, i)
            } else {
                format!("Text {{ text: \"{}\" }}", i)
            };
            
            let file_path = create_test_file(&temp_dir, &filename, &content);
            prev_file = Some(file_path.to_string_lossy().to_string());
        }
        
        let main_file = temp_dir.path().join(format!("file{}.kry", MAX_INCLUDE_DEPTH + 1));
        
        let mut preprocessor = Preprocessor::new();
        let result = preprocessor.process_includes(main_file.to_str().unwrap());
        
        assert!(result.is_err());
        if let Err(CompilerError::Include { message }) = result {
            assert!(message.contains("Maximum include depth"));
        } else {
            panic!("Expected max depth error");
        }
    }

    #[test]
    fn test_include_with_comments() {
        let temp_dir = TempDir::new().unwrap();
        
        create_test_file(&temp_dir, "included.kry", "Text { text: \"test\" }");
        
        let main_content = format!(
            "@include \"{}\" # This is a comment\nApp {{ }}", 
            temp_dir.path().join("included.kry").to_string_lossy()
        );
        let main_file = create_test_file(&temp_dir, "main.kry", &main_content);
        
        let mut preprocessor = Preprocessor::new();
        let result = preprocessor.process_includes(main_file.to_str().unwrap()).unwrap();
        
        assert!(result.0.contains("Text { text: \"test\" }"));
        assert!(result.0.contains("App { }"));
    }

    #[test]
    fn test_invalid_include_syntax() {
        let temp_dir = TempDir::new().unwrap();
        
        // Missing quotes
        let main_content = "@include missing_quotes.kry\nApp { }";
        let main_file = create_test_file(&temp_dir, "main.kry", main_content);
        
        let mut preprocessor = Preprocessor::new();
        let result = preprocessor.process_includes(main_file.to_str().unwrap());
        
        assert!(result.is_err());
        if let Err(CompilerError::Parse { message, .. }) = result {
            assert!(message.contains("path must be quoted"));
        } else {
            panic!("Expected parse error");
        }
    }

    #[test]
    fn test_parse_include_line() {
        let preprocessor = Preprocessor::new();
        
        // Valid include
        let result = preprocessor.parse_include_line("@include \"test.kry\"", 1).unwrap();
        assert_eq!(result, Some("test.kry".to_string()));
        
        // Valid include with comment
        let result = preprocessor.parse_include_line("@include \"test.kry\" # comment", 1).unwrap();
        assert_eq!(result, Some("test.kry".to_string()));
        
        // Not an include line
        let result = preprocessor.parse_include_line("App { }", 1).unwrap();
        assert_eq!(result, None);
        
        // Invalid syntax
        assert!(preprocessor.parse_include_line("@include missing_quotes", 1).is_err());
        assert!(preprocessor.parse_include_line("@include \"unterminated", 1).is_err());
        assert!(preprocessor.parse_include_line("@include \"\" extra", 1).is_err());
    }
}