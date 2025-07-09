//! Unified variable tracking and substitution system
//! 
//! This module provides a centralized variable context that can track variables from:
//! - @variables blocks (global variables)
//! - Component properties (component-local variables) 
//! - Function parameters (function-local variables)
//! - Style calculations (computed variables)

use crate::error::{CompilerError, Result};
use crate::types::*;
use crate::module_context::ModuleContext;
use std::collections::HashMap;
use std::path::PathBuf;
use regex::Regex;

/// Variable scope levels
#[derive(Debug, Clone, PartialEq)]
pub enum VariableScope {
    Global,     // @variables blocks
    Module,     // Module-level variables (with import priority)
    Component,  // Component property substitution
    Function,   // Function parameter substitution
    Style,      // Style inheritance and calculations
}

/// A variable entry with scope and metadata
#[derive(Debug, Clone)]
pub struct VariableEntry {
    pub name: String,
    pub value: String,
    pub scope: VariableScope,
    pub source_file: String,
    pub source_line: usize,
    pub value_type: ValueType,
    pub module_path: Option<PathBuf>,
    pub import_order: Option<usize>,
    pub is_private: bool,
}

/// Unified variable context for all substitution operations
#[derive(Debug, Clone)]
pub struct VariableContext {
    // Variables by scope - inner scopes shadow outer scopes
    variables: Vec<HashMap<String, VariableEntry>>,
    current_scope: VariableScope,
    
    // Stack for scope management
    scope_stack: Vec<VariableScope>,
    
    // Module-level variable management
    module_variables: HashMap<PathBuf, HashMap<String, VariableEntry>>,
    current_module: Option<PathBuf>,
    import_order_counter: usize,
}

impl VariableContext {
    pub fn new() -> Self {
        let mut context = Self {
            variables: vec![HashMap::new()], // Start with global scope
            current_scope: VariableScope::Global,
            scope_stack: Vec::new(),
            module_variables: HashMap::new(),
            current_module: None,
            import_order_counter: 0,
        };
        
        // Initialize global scope
        context.variables.push(HashMap::new());
        context
    }
    
    /// Push a new variable scope (component, function, etc.)
    pub fn push_scope(&mut self, scope: VariableScope) {
        self.scope_stack.push(self.current_scope.clone());
        self.current_scope = scope;
        self.variables.push(HashMap::new());
    }
    
    /// Pop the current scope, returning to the previous one
    pub fn pop_scope(&mut self) -> Result<()> {
        if self.variables.len() <= 1 {
            return Err(CompilerError::variable_legacy(
                0,
                "Cannot pop global variable scope".to_string()
            ));
        }
        
        self.variables.pop();
        self.current_scope = self.scope_stack.pop()
            .unwrap_or(VariableScope::Global);
        
        Ok(())
    }
    
    /// Add a variable to the current scope
    pub fn add_variable(&mut self, entry: VariableEntry) -> Result<()> {
        if let Some(current_scope_vars) = self.variables.last_mut() {
            current_scope_vars.insert(entry.name.clone(), entry);
            Ok(())
        } else {
            Err(CompilerError::variable_legacy(
                0,
                "No variable scope available".to_string()
            ))
        }
    }
    
    /// Add a simple string variable to current scope
    pub fn add_string_variable(&mut self, name: String, value: String, source_file: String, source_line: usize) -> Result<()> {
        let entry = VariableEntry {
            name: name.clone(),
            value,
            scope: self.current_scope.clone(),
            source_file,
            source_line,
            value_type: ValueType::String,
            module_path: self.current_module.clone(),
            import_order: None,
            is_private: name.starts_with('_'),
        };
        self.add_variable(entry)
    }
    
    /// Look up a variable by name, searching from current scope to global with module priority
    pub fn get_variable(&self, name: &str) -> Option<&VariableEntry> {
        // First, search in current scopes (Component, Function, Style)
        for scope_vars in self.variables.iter().rev() {
            if let Some(var) = scope_vars.get(name) {
                // Local definitions always override imports
                if var.scope != VariableScope::Module {
                    return Some(var);
                }
            }
        }
        
        // Then search in module variables with import priority
        // Current module variables override imported ones
        if let Some(current_module) = &self.current_module {
            if let Some(module_vars) = self.module_variables.get(current_module) {
                if let Some(var) = module_vars.get(name) {
                    return Some(var);
                }
            }
        }
        
        // Finally, search in imported module variables by import order (later imports override earlier ones)
        let mut best_match: Option<&VariableEntry> = None;
        let mut best_import_order = 0;
        
        for module_vars in self.module_variables.values() {
            if let Some(var) = module_vars.get(name) {
                if let Some(import_order) = var.import_order {
                    if best_match.is_none() || import_order >= best_import_order {
                        best_match = Some(var);
                        best_import_order = import_order;
                    }
                }
            }
        }
        
        if best_match.is_some() {
            return best_match;
        }
        
        // Finally, search in regular scopes for any remaining variables
        for scope_vars in self.variables.iter().rev() {
            if let Some(var) = scope_vars.get(name) {
                return Some(var);
            }
        }
        
        None
    }
    
    /// Substitute all $variable references in a string
    pub fn substitute_variables(&self, input: &str) -> Result<String> {
        let mut result = input.to_string();
        
        // Use regex to find all $variable references
        let var_regex = Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)")
            .map_err(|e| CompilerError::variable_legacy(0, format!("Regex error: {}", e)))?;
        
        // Collect all matches first to avoid borrowing issues
        let matches: Vec<_> = var_regex.captures_iter(input).collect();
        
        for captures in matches {
            if let Some(var_match) = captures.get(0) {
                let var_name = &captures[1];
                
                if let Some(var_entry) = self.get_variable(var_name) {
                    result = result.replace(var_match.as_str(), &var_entry.value);
                } else {
                    return Err(CompilerError::variable_legacy(
                        0,
                        format!("Undefined variable: ${}", var_name)
                    ));
                }
            }
        }
        
        Ok(result)
    }
    
    /// Get all variables in current scope (for debugging)
    pub fn get_current_scope_variables(&self) -> Vec<String> {
        if let Some(current_vars) = self.variables.last() {
            current_vars.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get all variables in all scopes (for debugging)
    pub fn get_all_variables(&self) -> Vec<&VariableEntry> {
        let mut all_vars = Vec::new();
        for scope_vars in &self.variables {
            for var in scope_vars.values() {
                all_vars.push(var);
            }
        }
        all_vars
    }
    
    /// Create a property mapping for component instantiation
    pub fn create_component_property_mapping(&self, component_props: &[ComponentPropertyDef], instance_props: &[crate::ast::AstProperty]) -> Result<HashMap<String, String>> {
        let mut mapping = HashMap::new();
        
        // Start with default values from component definition
        for prop_def in component_props {
            mapping.insert(prop_def.name.clone(), prop_def.default_value.clone());
        }
        
        // Override with instance-provided values
        for instance_prop in instance_props {
            if component_props.iter().any(|p| p.name == instance_prop.key) {
                // Substitute any variables in the instance property value
                let substituted_value = self.substitute_variables(&instance_prop.value)?;
                mapping.insert(instance_prop.key.clone(), substituted_value);
            } else {
                // Allow unknown properties to pass through but warn
                log::warn!("Component property '{}' not defined in component schema", instance_prop.key);
                let substituted_value = self.substitute_variables(&instance_prop.value)?;
                mapping.insert(instance_prop.key.clone(), substituted_value);
            }
        }
        
        Ok(mapping)
    }
    
    /// Set the current module context for variable resolution
    pub fn set_current_module(&mut self, module_path: PathBuf) {
        self.current_module = Some(module_path);
    }
    
    /// Import variables from a module with override priority
    pub fn import_module_variables(&mut self, module: &ModuleContext, import_order: usize) -> Result<()> {
        let module_path = module.file_path.clone();
        
        // Get public variables from the module
        let public_variables = module.get_public_variables();
        
        // Convert to VariableEntry format
        let mut variable_entries = HashMap::new();
        for (name, var_def) in public_variables {
            let entry = VariableEntry {
                name: name.clone(),
                value: var_def.value.clone(),
                scope: VariableScope::Module,
                source_file: module_path.to_string_lossy().to_string(),
                source_line: var_def.def_line,
                value_type: ValueType::String, // Default to string for now
                module_path: Some(module_path.clone()),
                import_order: Some(import_order),
                is_private: false, // Only public variables are imported
            };
            variable_entries.insert(name, entry);
        }
        
        // Store the module variables
        self.module_variables.insert(module_path, variable_entries);
        
        Ok(())
    }
    
    /// Add variables from a module context to the current context
    pub fn add_module_variables(&mut self, module: &ModuleContext) -> Result<()> {
        for (name, var_def) in &module.variables {
            // Skip private variables unless we're in the same module
            if module.is_private(name) && self.current_module.as_ref() != Some(&module.file_path) {
                continue;
            }
            
            let entry = VariableEntry {
                name: name.clone(),
                value: var_def.value.clone(),
                scope: VariableScope::Module,
                source_file: module.file_path.to_string_lossy().to_string(),
                source_line: var_def.def_line,
                value_type: ValueType::String,
                module_path: Some(module.file_path.clone()),
                import_order: module.dependency_order,
                is_private: module.is_private(name),
            };
            
            self.add_variable(entry)?;
        }
        
        Ok(())
    }
}

impl Default for VariableContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_variable_scoping() {
        let mut ctx = VariableContext::new();
        
        // Add global variable
        ctx.add_string_variable("global_var".to_string(), "global_value".to_string(), "test.kry".to_string(), 1).unwrap();
        
        // Push component scope
        ctx.push_scope(VariableScope::Component);
        ctx.add_string_variable("comp_var".to_string(), "comp_value".to_string(), "test.kry".to_string(), 5).unwrap();
        
        // Should find both variables
        assert!(ctx.get_variable("global_var").is_some());
        assert!(ctx.get_variable("comp_var").is_some());
        
        // Pop scope
        ctx.pop_scope().unwrap();
        
        // Should only find global variable
        assert!(ctx.get_variable("global_var").is_some());
        assert!(ctx.get_variable("comp_var").is_none());
    }
    
    #[test]
    fn test_variable_substitution() {
        let mut ctx = VariableContext::new();
        ctx.add_string_variable("title".to_string(), "Hello World".to_string(), "test.kry".to_string(), 1).unwrap();
        ctx.add_string_variable("count".to_string(), "42".to_string(), "test.kry".to_string(), 2).unwrap();
        
        let result = ctx.substitute_variables("text: $title, value: $count").unwrap();
        assert_eq!(result, "text: Hello World, value: 42");
    }
    
    #[test]
    fn test_undefined_variable_error() {
        let ctx = VariableContext::new();
        let result = ctx.substitute_variables("text: $undefined");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_module_variable_isolation() {
        let mut ctx = VariableContext::new();
        
        // Create mock module contexts
        let mut module1 = ModuleContext::new(PathBuf::from("module1.kry"));
        module1.add_variable("shared_var".to_string(), crate::types::VariableDef {
            value: "module1_value".to_string(),
            raw_value: "module1_value".to_string(),
            def_line: 1,
            is_resolving: false,
            is_resolved: true,
        });
        
        let mut module2 = ModuleContext::new(PathBuf::from("module2.kry"));
        module2.add_variable("shared_var".to_string(), crate::types::VariableDef {
            value: "module2_value".to_string(),
            raw_value: "module2_value".to_string(),
            def_line: 1,
            is_resolving: false,
            is_resolved: true,
        });
        
        // Import module1 first, then module2 - module2 should override
        ctx.import_module_variables(&module1, 0).unwrap();
        ctx.import_module_variables(&module2, 1).unwrap();
        
        // Should get module2's value due to later import order
        if let Some(var) = ctx.get_variable("shared_var") {
            assert_eq!(var.value, "module2_value");
        } else {
            panic!("Variable not found");
        }
    }
    
    #[test]
    fn test_private_variable_isolation() {
        let mut ctx = VariableContext::new();
        
        // Create module with private variable
        let mut module = ModuleContext::new(PathBuf::from("module.kry"));
        module.add_variable("_private_var".to_string(), crate::types::VariableDef {
            value: "private_value".to_string(),
            raw_value: "private_value".to_string(),
            def_line: 1,
            is_resolving: false,
            is_resolved: true,
        });
        
        // Import module - private variable should not be accessible
        ctx.import_module_variables(&module, 0).unwrap();
        
        // Should not find private variable
        assert!(ctx.get_variable("_private_var").is_none());
    }
}