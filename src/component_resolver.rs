//! Component instantiation and resolution system

use crate::ast::*;
use crate::error::{CompilerError, Result};
use crate::types::*;
use crate::variable_context::{VariableContext, VariableScope};
use std::collections::HashMap;
use regex;

pub struct ComponentResolver {
    instantiation_stack: Vec<String>, // Track instantiation to detect recursion
}

impl ComponentResolver {
    pub fn new() -> Self {
        Self {
            instantiation_stack: Vec::new(),
        }
    }
    
    /// Substitute variables in a string using the provided mapping
    /// Only supports ${variable} syntax
    pub fn substitute_variables(&self, input: &str, mapping: &HashMap<String, String>) -> Result<String> {
        let mut result = input.to_string();
        
        // Only support ${variable} syntax
        let var_regex = regex::Regex::new(r"\$\{([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap();
        
        // Process ${variable} syntax
        for capture in var_regex.captures_iter(input) {
            if let Some(var_name) = capture.get(1) {
                let var_name_str = var_name.as_str();
                if let Some(value) = mapping.get(var_name_str) {
                    let pattern = format!("${{{}}}", var_name_str);
                    result = result.replace(&pattern, value);
                } else {
                    return Err(CompilerError::Variable {
                        file: "test".to_string(),
                        line: 0,
                        message: format!("Undefined variable: ${{{}}}", var_name_str),
                    });
                }
            }
        }
        
        Ok(result)
    }
    
    pub fn resolve_components(&mut self, ast: &mut AstNode, state: &mut CompilerState) -> Result<()> {
        self.resolve_recursive(ast, state)?;
        self.update_component_statistics(state);
        Ok(())
    }
    
    fn resolve_recursive(&mut self, ast: &mut AstNode, state: &mut CompilerState) -> Result<()> {
        match ast {
            AstNode::File { app, components, .. } => {
                // First pass: collect all component definitions
                for component_node in components {
                    if let AstNode::Component { name, .. } = component_node {
                        if self.instantiation_stack.contains(name) {
                            return Err(CompilerError::component_legacy(
                                0,
                                format!("Recursive component definition detected: {}", name)
                            ));
                        }
                    }
                }
                
                // Second pass: resolve component instances in the main app
                if let Some(app_node) = app {
                    self.resolve_element_components(app_node, state)?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn resolve_element_components(&mut self, element: &mut AstNode, state: &mut CompilerState) -> Result<()> {
        match element {
            AstNode::Element { element_type, properties, children, .. } => {
                // Check if this element is a component instance
                if let Some(component_def) = self.find_component_definition(element_type, state) {
                    self.instantiate_component(element, &component_def, state)?;
                } else {
                    // Regular element - just process children
                    for child in children {
                        self.resolve_element_components(child, state)?;
                    }
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn find_component_definition(&self, name: &str, state: &CompilerState) -> Option<ComponentDefinition> {
        state.component_defs.iter()
            .find(|comp| comp.name == name)
            .cloned()
    }
    
    fn instantiate_component(
        &mut self,
        element: &mut AstNode,
        component_def: &ComponentDefinition,
        state: &mut CompilerState
    ) -> Result<()> {
        if let AstNode::Element { element_type, properties, children, .. } = element {
            // Check for recursive instantiation
            if self.instantiation_stack.contains(&component_def.name) {
                return Err(CompilerError::component_legacy(
                    0,
                    format!("Recursive component instantiation: {}", component_def.name)
                ));
            }
            
            self.instantiation_stack.push(component_def.name.clone());
            
            // Get the component template
            let template = self.get_component_template(component_def, state)?;
            
            // Push component scope and add component properties
            state.variable_context.push_scope(VariableScope::Component);
            
            // Add component properties to variable context
            for prop_def in &component_def.properties {
                // Strip quotes from default values for variable substitution
                let clean_default = if prop_def.default_value.starts_with('"') && prop_def.default_value.ends_with('"') {
                    prop_def.default_value[1..prop_def.default_value.len()-1].to_string()
                } else {
                    prop_def.default_value.clone()
                };
                
                state.variable_context.add_string_variable(
                    prop_def.name.clone(),
                    clean_default,
                    state.current_file_path.clone(),
                    0 // TODO: get actual line from component definition
                )?;
            }
            
            // Override with instance properties
            for instance_prop in properties {
                // Strip quotes from string values for variable substitution
                let clean_value = match &instance_prop.value {
                    PropertyValue::String(s) => {
                        if s.starts_with('"') && s.ends_with('"') {
                            s[1..s.len()-1].to_string()
                        } else {
                            s.clone()
                        }
                    }
                    _ => instance_prop.value.to_string(),
                };
                
                state.variable_context.add_string_variable(
                    instance_prop.key.clone(),
                    clean_value,
                    state.current_file_path.clone(),
                    instance_prop.line
                )?;
            }
            
            // Clone and customize the template using the variable context
            let mut instantiated_template = template.clone();
            self.apply_variable_substitution(&mut instantiated_template, state)?;
            
            // Handle instance children (slot content)
            if !children.is_empty() {
                self.inject_slot_content(&mut instantiated_template, children)?;
            }
            
            // Replace the component instance with the instantiated template
            *element = instantiated_template;
            
            // Resolve component function templates while in component scope
            self.resolve_component_function_templates(&component_def.name, state)?;
            
            // Recursively resolve any nested components
            self.resolve_element_components(element, state)?;
            
            // Pop component scope
            state.variable_context.pop_scope()?;
            
            self.instantiation_stack.pop();
        }
        
        Ok(())
    }
    
    fn get_component_template(&self, component_def: &ComponentDefinition, state: &CompilerState) -> Result<AstNode> {
        // Look for the template in our temporary AST storage
        if let Some(template_ast) = state.component_ast_templates.get(&component_def.name) {
            Ok(template_ast.clone())
        } else {
            Err(CompilerError::component_legacy(
                0,
                format!("Component '{}' has no template defined", component_def.name)
            ))
        }
    }
    
    fn convert_element_to_ast(&self, element: &Element, state: &CompilerState) -> Result<AstNode> {
        let mut properties = Vec::new();
        
        // Convert source properties back to AST properties
        for source_prop in &element.source_properties {
            properties.push(AstProperty::new(
                source_prop.key.clone(),
                PropertyValue::String(source_prop.value.clone()),
                source_prop.line_num,
            ));
        }
        
        // Convert children
        let mut children = Vec::new();
        for &child_index in &element.children {
            if let Some(child_element) = state.elements.get(child_index) {
                children.push(self.convert_element_to_ast(child_element, state)?);
            }
        }
        
        Ok(AstNode::Element {
            element_type: element.source_element_name.clone(),
            properties,
            pseudo_selectors: Vec::new(), // TODO: Convert state property sets back
            children,
        })
    }
    
    fn apply_variable_substitution(&self, template: &mut AstNode, state: &mut CompilerState) -> Result<()> {
        match template {
            AstNode::Element { properties, children, .. } => {
                // Replace variable references in properties using the variable context
                for prop in properties {
                    let value_str = prop.value.to_string();
                    let substituted = state.variable_context.substitute_variables(&value_str)?;
                    prop.value = PropertyValue::String(substituted);
                }
                
                // Recursively process children
                for child in children {
                    self.apply_variable_substitution(child, state)?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn inject_slot_content(&self, template: &mut AstNode, slot_content: &[AstNode]) -> Result<()> {
        // Find content slot in template (element with id="content_slot" or similar)
        if let Some(slot_element) = self.find_content_slot(template) {
            // Add slot content as children
            if let AstNode::Element { children, .. } = slot_element {
                children.extend_from_slice(slot_content);
            }
        } else {
            // No explicit slot found - append to root element
            if let AstNode::Element { children, .. } = template {
                children.extend_from_slice(slot_content);
            }
        }
        
        Ok(())
    }
    
    fn find_content_slot<'a>(&self, element: &'a mut AstNode) -> Option<&'a mut AstNode> {
        let is_slot = if let AstNode::Element { properties, .. } = element {
            properties.iter().any(|prop| {
                prop.key == "id"
                    && (prop.cleaned_value() == "content_slot"
                        || prop.cleaned_value() == "slot"
                        || prop.cleaned_value().contains("slot"))
            })
        } else {
            false
        };

        if is_slot {
            return Some(element);
        }

        if let AstNode::Element { children, .. } = element {
            for child in children {
                if let Some(slot) = self.find_content_slot(child) {
                    return Some(slot);
                }
            }
        }

        None
    }

    
    fn update_component_statistics(&self, state: &mut CompilerState) {
        // Update header flags if components were used
        if !state.component_defs.is_empty() {
            state.header_flags |= FLAG_HAS_COMPONENT_DEFS;
        }
        
        // Calculate component definition sizes
        for component in &mut state.component_defs {
            let mut size = 2u32; // name_index + property_count
            
            // Add property definitions size
            for prop_def in &component.properties {
                size += 3; // name_index + type_hint + default_value_length
                size += prop_def.default_value.len() as u32;
            }
            
            component.calculated_size = size;
        }
    }
    
    /// Validate component definitions for correctness
    pub fn validate_component_definitions(&self, state: &CompilerState) -> Result<()> {
        for component in &state.component_defs {
            // Check for valid property types
            for prop_def in &component.properties {
                self.validate_property_type(&prop_def.value_type_hint, &component.name)?;
            }
            
            // Check that component has a template
            if component.definition_root_element_index.is_none() {
                return Err(CompilerError::component_legacy(
                    component.definition_start_line,
                    format!("Component '{}' has no template element defined", component.name)
                ));
            }
        }
        
        Ok(())
    }
    
    fn validate_property_type(&self, value_type: &ValueType, component_name: &str) -> Result<()> {
        match value_type {
            ValueType::String | ValueType::Int | ValueType::Float | 
            ValueType::Bool | ValueType::Color | ValueType::Resource => {
                // Basic types are always valid
                Ok(())
            }
            ValueType::StyleId => {
                // Style references should be validated against available styles
                // This would require access to the style table
                Ok(())
            }
            ValueType::Enum => {
                // Enum types need validation of their values
                // This would require parsing the enum definition
                Ok(())
            }
            ValueType::Custom => {
                log::warn!("Component '{}' uses custom property type", component_name);
                Ok(())
            }
            _ => {
                Err(CompilerError::component_legacy(
                    0,
                    format!("Invalid property type in component '{}'", component_name)
                ))
            }
        }
    }
    
    /// Get statistics about component usage
    pub fn get_component_stats(&self, state: &CompilerState) -> ComponentStats {
        let mut stats = ComponentStats {
            total_definitions: state.component_defs.len(),
            total_instantiations: 0,
            max_instantiation_depth: 0,
            definitions_by_complexity: HashMap::new(),
        };
        
        // Count component usage in element tree
        for element in &state.elements {
            if element.is_component_instance {
                stats.total_instantiations += 1;
            }
        }
        
        // Analyze component complexity
        for component in &state.component_defs {
            let complexity = self.calculate_component_complexity(component, state);
            stats.definitions_by_complexity.insert(component.name.clone(), complexity);
        }
        
        stats
    }
    
    fn calculate_component_complexity(&self, component: &ComponentDefinition, state: &CompilerState) -> ComponentComplexity {
        let mut complexity = ComponentComplexity {
            property_count: component.properties.len(),
            template_element_count: 0,
            max_nesting_depth: 0,
            has_slot_content: false,
        };
        
        // Analyze template complexity
        if let Some(root_index) = component.definition_root_element_index {
            if let Some(root_element) = state.elements.get(root_index) {
                complexity.template_element_count = self.count_template_elements(root_element, state);
                complexity.max_nesting_depth = self.calculate_nesting_depth(root_element, state, 0);
                complexity.has_slot_content = self.has_slot_markers(root_element);
            }
        }
        
        complexity
    }
    
    fn count_template_elements(&self, element: &Element, state: &CompilerState) -> usize {
        let mut count = 1; // This element
        
        for &child_index in &element.children {
            if let Some(child) = state.elements.get(child_index) {
                count += self.count_template_elements(child, state);
            }
        }
        
        count
    }
    
    fn calculate_nesting_depth(&self, element: &Element, state: &CompilerState, current_depth: usize) -> usize {
        let mut max_depth = current_depth;
        
        for &child_index in &element.children {
            if let Some(child) = state.elements.get(child_index) {
                let child_depth = self.calculate_nesting_depth(child, state, current_depth + 1);
                max_depth = max_depth.max(child_depth);
            }
        }
        
        max_depth
    }
    
    fn has_slot_markers(&self, element: &Element) -> bool {
        // Check if element has id containing "slot"
        for prop in &element.source_properties {
            if prop.key == "id" && prop.value.contains("slot") {
                return true;
            }
        }
        
        false
    }
    
    /// Resolve component function templates with current variable context
    fn resolve_component_function_templates(
        &self, 
        component_name: &str, 
        state: &mut CompilerState
    ) -> Result<()> {
        use crate::types::{FunctionScope, ResolvedFunction};
        
        // Find all function templates for this component
        let component_templates: Vec<_> = state.function_templates
            .iter()
            .filter(|template| {
                matches!(&template.scope, FunctionScope::Component(name) if name == component_name)
            })
            .cloned()
            .collect();
        
        for template in component_templates {
            // Check if all required variables are available
            let mut all_vars_available = true;
            for var_name in &template.required_vars {
                if !state.variable_context.has_variable(var_name) {
                    all_vars_available = false;
                    break;
                }
            }
            
            if !all_vars_available {
                continue; // Skip this template
            }
            
            // Resolve the template with current variable context
            let resolved_name = state.variable_context.substitute_variables(&template.name_pattern)?;
            let resolved_body = state.variable_context.substitute_variables(&template.body)?;
            
            // Build complete function code
            let param_list = template.parameters.join(", ");
            let code = format!(
                "function {}({})\n{}\nend",
                resolved_name,
                param_list,
                resolved_body
            );
            
            // Create instance context identifier  
            let instance_context = format!("{}:{}", component_name, resolved_name);
            
            let resolved_function = ResolvedFunction {
                name: resolved_name.clone(),
                code,
                template_id: template.id,
                instance_context: Some(instance_context.clone()),
                language: template.language.clone(),
                parameters: template.parameters.clone(),
            };
            
            // Add to resolved functions
            state.resolved_functions.insert(resolved_name.clone(), resolved_function);
            
            // Track component functions
            state.component_functions
                .entry(instance_context)
                .or_insert_with(Vec::new)
                .push(resolved_name);
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ComponentStats {
    pub total_definitions: usize,
    pub total_instantiations: usize,
    pub max_instantiation_depth: usize,
    pub definitions_by_complexity: HashMap<String, ComponentComplexity>,
}

#[derive(Debug, Clone)]
pub struct ComponentComplexity {
    pub property_count: usize,
    pub template_element_count: usize,
    pub max_nesting_depth: usize,
    pub has_slot_content: bool,
}

impl ComponentStats {
    pub fn print_summary(&self) {
        println!("Component Statistics:");
        println!("  Definitions: {}", self.total_definitions);
        println!("  Instantiations: {}", self.total_instantiations);
        println!("  Max depth: {}", self.max_instantiation_depth);
        
        if !self.definitions_by_complexity.is_empty() {
            println!("  Complexity breakdown:");
            for (name, complexity) in &self.definitions_by_complexity {
                println!("    {}: {} props, {} elements, depth {}{}",
                        name,
                        complexity.property_count,
                        complexity.template_element_count,
                        complexity.max_nesting_depth,
                        if complexity.has_slot_content { " (slotted)" } else { "" });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_variable_substitution() {
        let resolver = ComponentResolver::new();
        let mut mapping = HashMap::new();
        mapping.insert("title".to_string(), "\"Hello World\"".to_string());
        mapping.insert("count".to_string(), "42".to_string());
        
        let result = resolver.substitute_variables("text: $title, value: $count", &mapping).unwrap();
        assert_eq!(result, "text: \"Hello World\", value: 42");
    }
    
    #[test]
    fn test_undefined_variable_error() {
        let resolver = ComponentResolver::new();
        let mapping = HashMap::new();
        
        let result = resolver.substitute_variables("text: $undefined", &mapping);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_component_complexity_calculation() {
        let resolver = ComponentResolver::new();
        let component_def = ComponentDefinition {
            name: "TestComponent".to_string(),
            properties: vec![
                ComponentPropertyDef {
                    name: "title".to_string(),
                    value_type_hint: ValueType::String,
                    default_value: "Default".to_string(),
                },
                ComponentPropertyDef {
                    name: "count".to_string(),
                    value_type_hint: ValueType::Int,
                    default_value: "0".to_string(),
                },
            ],
            definition_start_line: 1,
            definition_root_element_index: None,
            calculated_size: 0,
            internal_template_element_offsets: HashMap::new(),
        };
        
        let state = CompilerState::new();
        let complexity = resolver.calculate_component_complexity(&component_def, &state);
        
        assert_eq!(complexity.property_count, 2);
        assert_eq!(complexity.template_element_count, 0); // No template
    }
}
