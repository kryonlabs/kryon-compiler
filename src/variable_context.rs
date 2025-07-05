//! Unified variable tracking and substitution system
//! 
//! This module provides a centralized variable context that can track variables from:
//! - @variables blocks (global variables)
//! - Component properties (component-local variables) 
//! - Function parameters (function-local variables)
//! - Style calculations (computed variables)

use crate::error::{CompilerError, Result};
use crate::types::*;
use std::collections::HashMap;
use regex::Regex;

/// Variable scope levels
#[derive(Debug, Clone, PartialEq)]
pub enum VariableScope {
    Global,     // @variables blocks
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
}

/// Unified variable context for all substitution operations
#[derive(Debug, Clone)]
pub struct VariableContext {
    // Variables by scope - inner scopes shadow outer scopes
    variables: Vec<HashMap<String, VariableEntry>>,
    current_scope: VariableScope,
    
    // Stack for scope management
    scope_stack: Vec<VariableScope>,
}

impl VariableContext {
    pub fn new() -> Self {
        let mut context = Self {
            variables: vec![HashMap::new()], // Start with global scope
            current_scope: VariableScope::Global,
            scope_stack: Vec::new(),
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
        };
        self.add_variable(entry)
    }
    
    /// Look up a variable by name, searching from current scope to global
    pub fn get_variable(&self, name: &str) -> Option<&VariableEntry> {
        // Search from innermost scope to outermost
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
}