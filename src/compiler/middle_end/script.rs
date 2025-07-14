//! Script processing and integration

use crate::compiler::frontend::ast::{AstNode, ScriptSource};
use crate::error::{CompilerError, Result};
use crate::core::*;
use crate::core::types::*;
use crate::CompilerOptions;

use crate::compiler::backend::codegen::{SCRIPT_STORAGE_INLINE, SCRIPT_STORAGE_EXTERNAL};
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
                            SCRIPT_STORAGE_EXTERNAL
                        } else {
                            SCRIPT_STORAGE_INLINE
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
                        (SCRIPT_STORAGE_EXTERNAL, resource_idx as u16, Vec::new(), Some(resource_idx), source.clone())
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
                    code_data: code_data.to_vec(),
                    resource_index,
                    calculated_size,
                    source_line_num: 0, // Would be set by parser
                })
            }
            _ => Err(CompilerError::script_legacy(0, "Expected script node"))
        }
    }
    
    pub fn extract_entry_points(&self, language: ScriptLanguage, source: &ScriptSource) -> Result<Vec<String>> {
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


/// Collect function templates from AST (Phase 1.3)
fn collect_function_templates(ast: &AstNode, state: &mut CompilerState) -> Result<()> {
    use crate::core::FunctionScope;
    
    
    match ast {
        AstNode::File { scripts, components, .. } => {
            // Collect global function templates (including scripts associated with components)
            for script_node in scripts {
                if let AstNode::Script { language, name, source, .. } = script_node {
                    if let Some(func_name) = name {
                        // Named global function
                        let template = create_function_template(
                            func_name,
                            language,
                            source,
                            &[],
                            FunctionScope::Global,
                            state
                        )?;
                        state.function_templates.push(template);
                    } else {
                        // Unnamed global @script block - could be associated with a component
                        // If we have components in this file, associate the script with the first component
                        let scope = if !components.is_empty() {
                            if let AstNode::Component { name: comp_name, .. } = &components[0] {
                                FunctionScope::Component(comp_name.clone())
                            } else {
                                FunctionScope::Global
                            }
                        } else {
                            FunctionScope::Global
                        };
                        
                        // @script blocks should be processed as complete scripts, not as individual function templates
                        // Skip function template creation for @script blocks - they will be handled in script processing phase
                    }
                }
            }
            
            // Collect component function templates
            for component_node in components {
                if let AstNode::Component { name: comp_name, functions, .. } = component_node {
                    for script_node in functions {
                        if let AstNode::Script { language, name, source, .. } = script_node {
                            if let Some(func_name) = name {
                                // Named @function - create single template
                                let template = create_function_template(
                                    func_name,
                                    language,
                                    source,
                                    &[],
                                    FunctionScope::Component(comp_name.clone()),
                                    state
                                )?;
                                state.function_templates.push(template);
                            } else {
                                // Unnamed @script block - store for deferred processing during component instantiation
                                println!("DEBUG: Storing @script block for component: {}", comp_name);
                                state.component_scripts
                                    .entry(comp_name.clone())
                                    .or_insert_with(Vec::new)
                                    .push(script_node.clone());
                                println!("DEBUG: Component {} now has {} stored scripts", comp_name, state.component_scripts.get(comp_name).unwrap().len());
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    
    Ok(())
}

/// Create a function template from parsed script data
fn create_function_template(
    name_pattern: &str,
    language: &str,
    source: &ScriptSource,
    parameters: &[String],
    scope: FunctionScope,
    state: &mut CompilerState,
) -> Result<FunctionTemplate> {
    use crate::core::{FunctionTemplate, SourceLocation};
    use std::collections::HashSet;
    
    let body = match source {
        ScriptSource::Inline(code) => code.clone(),
        ScriptSource::External(path) => {
            return Err(CompilerError::parse_legacy(
                0,
                "External script files not supported for function templates yet"
            ));
        }
    };
    
    // Extract required variables from name pattern and body
    let mut required_vars = HashSet::new();
    extract_variables_from_text(name_pattern, &mut required_vars);
    extract_variables_from_text(&body, &mut required_vars);
    
    let template_id = state.next_template_id;
    state.next_template_id += 1;
    
    Ok(FunctionTemplate {
        id: template_id,
        name_pattern: name_pattern.to_string(),
        body,
        parameters: parameters.to_vec(),
        language: language.to_string(),
        scope,
        required_vars,
        source_location: SourceLocation {
            file: state.current_file_path.clone(),
            line: state.current_line_num,
            column: 0,
        },
    })
}

/// Extract variables from text (simple $var pattern matching)
fn extract_variables_from_text(text: &str, vars: &mut std::collections::HashSet<String>) {
    let var_regex = regex::Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    for cap in var_regex.captures_iter(text) {
        if let Some(var_name) = cap.get(1) {
            vars.insert(var_name.as_str().to_string());
        }
    }
}

/// Resolve global function templates (Phase 1.35)
fn resolve_global_function_templates(state: &mut CompilerState) -> Result<()> {
    use crate::core::{FunctionScope, ResolvedFunction};
    
    for template in &state.function_templates {
        if template.scope == FunctionScope::Global {
            // Check if all required variables are available
            for var_name in &template.required_vars {
                if !state.variable_context.has_variable(var_name) {
                    // Skip this template - variables not available yet
                    continue;
                }
            }
            
            // Resolve the template
            let resolved_name = state.variable_context.substitute_variables(&template.name_pattern)?;
            let resolved_body = state.variable_context.substitute_variables(&template.body)?;
            
            // Build complete function code
            let param_list = template.parameters.join(", ");
            let code = if resolved_body.trim().starts_with("function") {
                // Body already contains complete function, use as-is
                resolved_body
            } else {
                // Body is just the content, wrap it with function declaration
                format!(
                    "function {}({})\n{}\nend",
                    resolved_name,
                    param_list,
                    resolved_body
                )
            };
            
            let resolved_function = ResolvedFunction {
                name: resolved_name.clone(),
                code,
                template_id: template.id,
                instance_context: None,
                language: template.language.clone(),
                parameters: template.parameters.clone(),
            };
            
            state.resolved_functions.insert(resolved_name, resolved_function);
        }
    }
    
    Ok(())
}

/// Process resolved scripts into ScriptEntry format (Phase 1.7)
fn process_resolved_scripts(state: &mut CompilerState) -> Result<()> {
    
    // Collect resolved functions first to avoid borrowing issues
    let resolved_funcs: Vec<_> = state.resolved_functions.values().cloned().collect();
    
    // Process all resolved functions as script entries
    for resolved_func in resolved_funcs {
        // resolved_func.code already contains the complete function with declaration and end
        // We need to convert it directly to ScriptEntry without re-processing
        
        let language_id = ScriptLanguage::from_name(&resolved_func.language)
            .unwrap_or(ScriptLanguage::Lua);
            
        let name_index = state.add_string(&resolved_func.name)?;
        
        // Create script function entry point
        let func_name_index = state.add_string(&resolved_func.name)?;
        let script_function = ScriptFunction {
            function_name: resolved_func.name.clone(),
            function_name_index: func_name_index,
        };
        
        // Apply variable substitution to the already-complete function code
        let substituted_code = state.variable_context.substitute_variables(&resolved_func.code)?;
        let code_data = substituted_code.as_bytes().to_vec();
        
        let calculated_size = 6 + 1 + code_data.len() as u32; // header + entry point + code data
        
        let script_entry = ScriptEntry {
            language_id,
            name: resolved_func.name.clone(),
            name_index,
            storage_format: SCRIPT_STORAGE_INLINE,
            entry_point_count: 1,
            data_size: code_data.len() as u16,
            code_data,
            entry_points: vec![script_function],
            resource_index: None,
            calculated_size,
            source_line_num: 0, // Not available for resolved functions
        };
        
        state.scripts.push(script_entry);
    }
    
    Ok(())
}



/// Process template variables and create template binding tables
pub fn process_template_variables(state: &mut CompilerState, options: &CompilerOptions) -> Result<()> {
    use std::collections::HashMap;
    
    // Collect all variables from @variables blocks
    let mut variable_map: HashMap<String, (u8, ValueType)> = HashMap::new();
    
    // First, collect variable data to avoid borrowing conflicts
    let variables_to_process: Vec<(String, String)> = state.variables.iter()
        .map(|(name, def)| (name.clone(), def.value.clone()))
        .collect();
    
    // Create template variables from the @variables block
    for (var_name, var_value) in variables_to_process {
        let name_index = if let Some(idx) = state.strings.iter().position(|s| s.text == var_name) {
            idx as u8
        } else {
            state.add_string(var_name.clone())?
        };
        
        let default_value_index = if let Some(idx) = state.strings.iter().position(|s| s.text == var_value) {
            idx as u8
        } else {
            state.add_string(var_value.clone())?
        };
        
        // Determine value type based on the value
        let value_type = if var_value.parse::<i32>().is_ok() {
            ValueType::Int
        } else if var_value.parse::<f32>().is_ok() {
            ValueType::Float
        } else if var_value == "true" || var_value == "false" {
            ValueType::Bool
        } else {
            ValueType::String
        };
        
        let template_var = TemplateVariable {
            name: var_name.clone(),
            name_index,
            value_type,
            default_value: var_value.clone(),
            default_value_index,
        };
        
        variable_map.insert(var_name.clone(), (state.template_variables.len() as u8, value_type));
        state.template_variables.push(template_var);
    }
    
    // Collect element properties data to avoid borrowing conflicts
    let mut properties_to_process = Vec::new();
    for (element_index, element) in state.elements.iter().enumerate() {
        for source_prop in &element.source_properties {
            // Check if this property has template variables
            let template_variables = extract_template_variables(&source_prop.value);
            
            if options.debug_mode {
                log::debug!("Element {}: property '{}' = '{}' -> template vars: {:?}", 
                           element_index, source_prop.key, source_prop.value, template_variables);
            }
            
            if !template_variables.is_empty() {
                properties_to_process.push((element_index, source_prop.key.clone(), source_prop.value.clone(), template_variables));
            }
        }
    }
    
    // Now process the collected properties
    for (element_index, prop_key, prop_value, template_variables) in properties_to_process {
        // Get the expression string index
        let expression_index = if let Some(idx) = state.strings.iter().position(|s| s.text == prop_value) {
            idx as u8
        } else {
            state.add_string(prop_value.clone())?
        };
                
        // Map property key to property ID
        let property_id = PropertyId::from_name(&prop_key) as u8;
        
        // Get variable indices
        let mut variable_indices = Vec::new();
        for var_name in &template_variables {
            if let Some((var_index, _)) = variable_map.get(var_name) {
                variable_indices.push(*var_index);
            }
        }
        
        let template_binding = TemplateBinding {
            element_index: element_index as u16,
            property_id,
            template_expression: prop_value.clone(),
            template_expression_index: expression_index,
            variable_count: variable_indices.len() as u8,
            variable_indices,
        };
        
        state.template_bindings.push(template_binding);
    }
    
    // Set the template variable flag if we have any template variables
    if !state.template_variables.is_empty() {
        state.header_flags |= FLAG_HAS_TEMPLATE_VARIABLES;
    }
    
    Ok(())
}

/// Extract template variables from a string ($variable_name)
fn extract_template_variables(value: &str) -> Vec<String> {
    use regex::Regex;
    
    let re = Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    let mut variables = Vec::new();
    
    for capture in re.captures_iter(value) {
        if let Some(var_name) = capture.get(1) {
            let name = var_name.as_str().to_string();
            if !variables.contains(&name) {
                variables.push(name);
            }
        }
    }
    
    variables
}



