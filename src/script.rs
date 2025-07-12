//! Script processing and integration

use crate::ast::{AstNode, ScriptSource};
use crate::error::{CompilerError, Result};
use crate::types::*;
use crate::codegen::{ScriptStorageInline, ScriptStorageExternal};
use regex::Regex;
use std::collections::HashMap;

pub struct ScriptProcessor {
    function_regex: HashMap<ScriptLanguage, Regex>,
}

impl ScriptProcessor {
    pub fn new() -> Self {
        let mut function_regex = HashMap::new();
        
        // Regex patterns for different languages to extract function names
        function_regex.insert(
            ScriptLanguage::Lua,
            Regex::new(r"function\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(").unwrap()
        );
        
        function_regex.insert(
            ScriptLanguage::JavaScript,
            Regex::new(r"function\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(").unwrap()
        );
        
        function_regex.insert(
            ScriptLanguage::Python,
            Regex::new(r"def\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(").unwrap()
        );
        
        function_regex.insert(
            ScriptLanguage::Wren,
            Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)\s*\([^)]*\)\s*\{").unwrap()
        );
        
        Self { function_regex }
    }
    
    pub fn process_script(&self, script_node: &AstNode, state: &mut CompilerState) -> Result<ScriptEntry> {
        match script_node {
            AstNode::Script { language, name, source, mode } => {
                let language_id = ScriptLanguage::from_name(language)
                    .ok_or_else(|| CompilerError::script_legacy(0, format!("Unsupported script language: {}", language)))?;
                
                let name_index = if let Some(n) = name {
                    // Apply variable substitution to the script name
                    let substituted_name = state.variable_context.substitute_variables(n)?;
                    state.add_string(&substituted_name)?
                } else {
                    0
                };
                
                let (storage_format, data_size, code_data, resource_index, substituted_source) = match source {
                    ScriptSource::Inline(code) => {
                        // Apply variable substitution to the script code
                        let substituted_code = state.variable_context.substitute_variables(code)?;
                        
                        let storage = if mode.as_deref() == Some("external") {
                            ScriptStorageExternal
                        } else {
                            ScriptStorageInline
                        };
                        
                        (storage, substituted_code.len() as u16, substituted_code.as_bytes().to_vec(), None, ScriptSource::Inline(substituted_code))
                    }
                    ScriptSource::External(path) => {
                        let res_type = match language_id {
                            ScriptLanguage::Lua => ResourceType::Script,
                            ScriptLanguage::JavaScript => ResourceType::Script,
                            ScriptLanguage::Python => ResourceType::Script,
                            ScriptLanguage::Wren => ResourceType::Script,
                        };
                        
                        let resource_idx = state.add_resource(res_type as u8, path)?;
                        (ScriptStorageExternal, resource_idx as u16, Vec::new(), Some(resource_idx), source.clone())
                    }
                };
                
                // Extract entry points (function names) from the substituted source
                let entry_points = self.extract_entry_points(language_id, &substituted_source)?;
                let mut script_functions = Vec::new();
                
                for func_name in entry_points {
                    let func_name_index = state.add_string(&func_name)?;
                    script_functions.push(ScriptFunction {
                        function_name: func_name,
                        function_name_index: func_name_index,
                    });
                }
                

                let calculated_size = self.calculate_script_size(&script_functions, &code_data);

                Ok(ScriptEntry {
                    language_id,
                    name: if let Some(n) = name {
                        state.variable_context.substitute_variables(n).unwrap_or_else(|_| n.clone())
                    } else {
                        String::new()
                    },
                    name_index,
                    storage_format,
                    entry_point_count: script_functions.len() as u8,
                    data_size,
                    entry_points: script_functions,
                    code_data,
                    resource_index,
                    calculated_size,
                    source_line_num: 0, // Would be set by parser
                })
            }
            _ => Err(CompilerError::script_legacy(0, "Expected script node"))
        }
    }
    
    fn extract_entry_points(&self, language: ScriptLanguage, source: &ScriptSource) -> Result<Vec<String>> {
        let code = match source {
            ScriptSource::Inline(code) => code,
            ScriptSource::External(_) => return Ok(Vec::new()), // External scripts analyzed at runtime
        };
        
        let regex = self.function_regex.get(&language)
            .ok_or_else(|| CompilerError::script_legacy(0, format!("No function extraction pattern for {:?}", language)))?;
        
        let mut functions = Vec::new();
        
        for captures in regex.captures_iter(code) {
            if let Some(func_name) = captures.get(1) {
                let name = func_name.as_str().to_string();
                if !functions.contains(&name) {
                    functions.push(name);
                }
            }
        }
        
        Ok(functions)
    }
    
    fn calculate_script_size(&self, entry_points: &[ScriptFunction], code_data: &[u8]) -> u32 {
        // Basic calculation: header + entry points + code data
        let header_size = 8; // Basic script entry header
        let entry_points_size = entry_points.len() * 2; // Each entry point is 2 bytes (string index)
        let code_size = code_data.len();
        
        (header_size + entry_points_size + code_size) as u32
    }
    
    /// Validate script syntax (basic validation)
    pub fn validate_script_syntax(&self, language: ScriptLanguage, code: &str) -> Result<()> {
        match language {
            ScriptLanguage::Lua => self.validate_lua_syntax(code),
            ScriptLanguage::JavaScript => self.validate_javascript_syntax(code),
            ScriptLanguage::Python => self.validate_python_syntax(code),
            ScriptLanguage::Wren => self.validate_wren_syntax(code),
        }
    }
    
    fn validate_lua_syntax(&self, code: &str) -> Result<()> {
        // Basic Lua syntax validation
        let mut paren_count = 0;
        let mut brace_count = 0;
        let mut in_string = false;
        let mut in_comment = false;
        
        for line in code.lines() {
            in_comment = false;
            for (i, ch) in line.char_indices() {
                if in_comment {
                    break;
                }
                
                if in_string {
                    if ch == '"' && (i == 0 || line.chars().nth(i-1) != Some('\\')) {
                        in_string = false;
                    }
                    continue;
                }
                
                match ch {
                    '"' => in_string = true,
                    '-' if line.chars().nth(i+1) == Some('-') => in_comment = true,
                    '(' => paren_count += 1,
                    ')' => paren_count -= 1,
                    '{' => brace_count += 1,
                    '}' => brace_count -= 1,
                    _ => {}
                }
                
                if paren_count < 0 || brace_count < 0 {
                    return Err(CompilerError::script_legacy(0, "Unmatched brackets in Lua script"));
                }
            }
        }
        
        if paren_count != 0 || brace_count != 0 {
            return Err(CompilerError::script_legacy(0, "Unmatched brackets in Lua script"));
        }
        
        Ok(())
    }
    
    fn validate_javascript_syntax(&self, code: &str) -> Result<()> {
        // Basic JavaScript syntax validation (similar to Lua but with different comment syntax)
        let mut paren_count = 0;
        let mut brace_count = 0;
        let mut bracket_count = 0;
        let mut in_string = false;
        let mut in_comment = false;
        
        for line in code.lines() {
            in_comment = false;
            let chars: Vec<char> = line.chars().collect();
            
            for i in 0..chars.len() {
                if in_comment {
                    break;
                }
                
                if in_string {
                    if chars[i] == '"' && (i == 0 || chars[i-1] != '\\') {
                        in_string = false;
                    }
                    continue;
                }
                
                match chars[i] {
                    '"' => in_string = true,
                    '/' if i + 1 < chars.len() && chars[i+1] == '/' => in_comment = true,
                    '(' => paren_count += 1,
                    ')' => paren_count -= 1,
                    '{' => brace_count += 1,
                    '}' => brace_count -= 1,
                    '[' => bracket_count += 1,
                    ']' => bracket_count -= 1,
                    _ => {}
                }
                
                if paren_count < 0 || brace_count < 0 || bracket_count < 0 {
                    return Err(CompilerError::script_legacy(0, "Unmatched brackets in JavaScript script"));
                }
            }
        }
        
        if paren_count != 0 || brace_count != 0 || bracket_count != 0 {
            return Err(CompilerError::script_legacy(0, "Unmatched brackets in JavaScript script"));
        }
        
        Ok(())
    }
    
    fn validate_python_syntax(&self, _code: &str) -> Result<()> {
        // Python syntax validation is more complex due to indentation
        // For now, just accept any Python code
        Ok(())
    }
    
    fn validate_wren_syntax(&self, code: &str) -> Result<()> {
        // Basic Wren syntax validation (similar to JavaScript)
        self.validate_javascript_syntax(code)
    }
}

impl CompilerState {
    pub fn add_resource(&mut self, resource_type: u8, path: &str) -> Result<u8> {
        let name_index = self.add_string(path)?;
        
        // Check for existing resource
        for entry in &self.resources {
            if entry.resource_type as u8 == resource_type && entry.name_index == name_index {
                return Ok(entry.index);
            }
        }
        
        if self.resources.len() >= MAX_RESOURCES {
            return Err(CompilerError::LimitExceeded {
                limit_type: "resources".to_string(),
                limit: MAX_RESOURCES,
            });
        }
        
        let index = self.resources.len() as u8;
        self.resources.push(ResourceEntry {
            resource_type: ResourceType::Script, // This should be parameterized
            name_index,
            format: ResourceFormat::External,
            data_string_index: name_index,
            index,
            calculated_size: 4,
        });
        
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_lua_functions() {
        let processor = ScriptProcessor::new();
        let code = r#"
            function hello()
                print("Hello")
            end
            
            function world(param)
                return param
            end
            
            local function private_func()
                -- private
            end
        "#;
        
        let source = ScriptSource::Inline(code.to_string());
        let functions = processor.extract_entry_points(ScriptLanguage::Lua, &source).unwrap();
        
        assert_eq!(functions.len(), 2);
        assert!(functions.contains(&"hello".to_string()));
        assert!(functions.contains(&"world".to_string()));
    }
    
    #[test]
    fn test_validate_lua_syntax() {
        let processor = ScriptProcessor::new();
        
        let valid_code = r#"
            function test()
                if true then
                    print("valid")
                end
            end
        "#;
        
        assert!(processor.validate_lua_syntax(valid_code).is_ok());
        
        let invalid_code = "function test( print('missing paren') end";
        assert!(processor.validate_lua_syntax(invalid_code).is_err());
    }
    
    #[test]
    fn test_javascript_functions() {
        let processor = ScriptProcessor::new();
        let code = r#"
            function handleClick() {
                console.log("clicked");
            }
            
            function validateInput(value) {
                return value.length > 0;
            }
        "#;
        
        let source = ScriptSource::Inline(code.to_string());
        let functions = processor.extract_entry_points(ScriptLanguage::JavaScript, &source).unwrap();
        
        assert_eq!(functions.len(), 2);
        assert!(functions.contains(&"handleClick".to_string()));
        assert!(functions.contains(&"validateInput".to_string()));
    }
}