//! Preprocessor for handling @include directives and file inclusion
//! 
//! Now supports module-level isolation where each @include creates
//! an isolated context with its own variables, styles, and components.

use crate::error::{CompilerError, Result, SourceMap};
use crate::types::MAX_INCLUDE_DEPTH;
use crate::module_context::{ModuleContext, ModuleGraph};
use std::collections::{HashSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Preprocessor {
    included_files: HashSet<PathBuf>,
    current_depth: usize,
    source_map: SourceMap,
    combined_line_count: usize,
    
    // New: Global cache of processed files to prevent duplicate inclusion
    processed_file_cache: HashMap<PathBuf, String>,
    
    // New: Module isolation support
    module_graph: Option<ModuleGraph>,
    processed_modules: HashMap<PathBuf, ModuleContext>,
}

impl Preprocessor {
    pub fn new() -> Self {
        Self {
            included_files: HashSet::new(),
            current_depth: 0,
            source_map: SourceMap::new(),
            combined_line_count: 1,
            processed_file_cache: HashMap::new(),
            module_graph: None,
            processed_modules: HashMap::new(),
        }
    }

    /// Process @include directives recursively and return combined content with source map
    /// (Legacy method - maintains backward compatibility)
    pub fn process_includes(&mut self, file_path: &str) -> Result<(String, SourceMap)> {
        let content = self.process_includes_recursive(file_path, 0)?;
        Ok((content, self.source_map.clone()))
    }
    
    /// NEW: Process includes with module isolation
    /// Each @include creates an isolated module context
    pub fn process_includes_isolated(&mut self, file_path: &str) -> Result<ModuleGraph> {
        let root_path = fs::canonicalize(file_path).map_err(|e| {
            CompilerError::FileNotFound {
                path: format!("{}: {}", file_path, e),
            }
        })?;
        
        // Initialize module graph
        let mut graph = ModuleGraph::new(root_path.clone());
        
        // Process the root module and all its dependencies
        self.process_module_recursive(&root_path, &mut graph, 0)?;
        
        // Resolve module dependencies and import order
        graph.resolve_dependencies()?;
        
        // Apply imports with override priority
        self.apply_module_imports(&mut graph)?;
        
        Ok(graph)
    }
    
    fn process_module_recursive(&mut self, module_path: &PathBuf, graph: &mut ModuleGraph, depth: usize) -> Result<()> {
        if depth > MAX_INCLUDE_DEPTH {
            return Err(CompilerError::Include {
                message: format!("Maximum include depth ({}) exceeded processing '{}'", 
                                MAX_INCLUDE_DEPTH, module_path.display()),
            });
        }
        
        // Check if we've already processed this module
        if graph.modules.contains_key(module_path) {
            return Ok(());
        }
        
        // Check for circular includes in current processing stack
        if self.included_files.contains(module_path) {
            return Err(CompilerError::Include {
                message: format!("Circular include detected: '{}'", module_path.display()),
            });
        }
        
        self.included_files.insert(module_path.clone());
        
        // Read and process the module file
        let content = fs::read_to_string(module_path).map_err(|e| {
            CompilerError::FileNotFound {
                path: format!("{}: {}", module_path.display(), e),
            }
        })?;
        
        // Create module context
        let mut module = ModuleContext::new(module_path.clone());
        
        // Process the content and extract includes
        let (processed_content, include_paths) = self.process_module_content(&content, module_path, depth)?;
        module.content = processed_content;
        
        // Process all included modules first (dependencies)
        let mut import_order = 0;
        for include_path in include_paths {
            self.process_module_recursive(&include_path, graph, depth + 1)?;
            
            // Record the import in the module
            let import = crate::module_context::ModuleImport {
                module_path: include_path,
                accessible_items: Vec::new(), // Will be filled during import resolution
                import_order,
            };
            module.imports.push(import);
            import_order += 1;
        }
        
        // Add this module to the graph
        graph.add_module(module);
        
        self.included_files.remove(module_path);
        Ok(())
    }
    
    fn process_module_content(&mut self, content: &str, module_path: &PathBuf, _depth: usize) -> Result<(String, Vec<PathBuf>)> {
        let mut result = String::new();
        let mut include_paths = Vec::new();
        let mut line_num = 0;
        let lines: Vec<&str> = content.lines().collect();
        let base_dir = module_path.parent().unwrap_or(Path::new(""));
        
        for line in &lines {
            line_num += 1;
            let trimmed = line.trim_start();
            
            if let Some(include_path) = self.parse_include_line(trimmed, line_num)? {
                // Store include path for comment before moving it
                let include_comment = include_path.clone();
                
                // Resolve include path relative to current module
                let full_include_path = if Path::new(&include_path).is_absolute() {
                    PathBuf::from(include_path)
                } else {
                    base_dir.join(&include_path)
                };
                
                let canonical_include_path = fs::canonicalize(&full_include_path).map_err(|e| {
                    CompilerError::FileNotFound {
                        path: format!("{}: {}", full_include_path.display(), e),
                    }
                })?;
                
                include_paths.push(canonical_include_path);
                
                // In isolated mode, @include lines are removed from content
                // The module content will be merged later during compilation
                result.push_str(&format!("# @include processed: {}\n", include_comment));
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }
        
        Ok((result, include_paths))
    }
    
    fn apply_module_imports(&mut self, graph: &mut ModuleGraph) -> Result<()> {
        // Apply imports in dependency order
        for module_path in graph.compilation_order.clone() {
            let import_list = if let Some(module) = graph.modules.get(&module_path) {
                module.imports.clone()
            } else {
                continue;
            };
            
            for import in import_list {
                if let Some(imported_module) = graph.modules.get(&import.module_path).cloned() {
                    if let Some(importing_module) = graph.modules.get_mut(&module_path) {
                        importing_module.import_module(&imported_module, import.import_order)?;
                    }
                }
            }
        }
        
        Ok(())
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

                // Check if we've already processed this file
                let canonical_include_path = fs::canonicalize(&full_include_path).map_err(|e| {
                    CompilerError::FileNotFound {
                        path: format!("{}: {}", full_include_path, e),
                    }
                })?;
                
                let included_content = if let Some(_cached_content) = self.processed_file_cache.get(&canonical_include_path) {
                    // File already included - just add a comment instead of duplicating content
                    log::debug!("Using cached content for: {}", full_include_path);
                    format!("# Already included: {}", include_path)
                } else {
                    // Process the file and cache the result
                    match self.process_includes_recursive(&full_include_path, depth + 1) {
                        Ok(content) => {
                            self.processed_file_cache.insert(canonical_include_path, content.clone());
                            content
                        }
                        Err(e) => {
                            return Err(CompilerError::Include {
                                message: format!("Error processing include '{}' at line {}: {}", 
                                               include_path, line_num, e),
                            });
                        }
                    }
                };
                
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