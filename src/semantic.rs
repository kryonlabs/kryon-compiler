//! Semantic analysis and validation for the Kryon compiler

use crate::ast::*;
use crate::error::{CompilerError, Result};
use crate::types::*;
use std::collections::{HashMap, HashSet};

pub struct SemanticAnalyzer {
    errors: Vec<CompilerError>,
    warnings: Vec<String>,
    variable_usage: HashMap<String, Vec<usize>>, // Track where variables are used
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            variable_usage: HashMap::new(),
        }
    }
    
    pub fn analyze(&mut self, ast: &AstNode, state: &mut CompilerState) -> Result<()> {
        // Phase 1: Collect all definitions
        self.collect_definitions(ast, state)?;
        
        // Phase 2: Resolve dependencies
        self.resolve_dependencies(state)?;
        
        // Phase 3: Validate elements and properties
        self.validate_elements(ast, state)?;
        
        // Phase 4: Check for unused definitions
        self.check_unused_definitions(state)?;
        
        // Return any accumulated errors
        if !self.errors.is_empty() {
            return Err(self.errors.remove(0));
        }
        
        // Log warnings
        for warning in &self.warnings {
            log::warn!("{}", warning);
        }
        
        Ok(())
    }
    
    fn collect_definitions(&mut self, ast: &AstNode, state: &mut CompilerState) -> Result<()> {
        match ast {
            AstNode::File { styles, components, scripts, .. } => {
                // Collect styles
                for style_node in styles {
                    self.collect_style_definition(style_node, state)?;
                }
                
                // Collect components
                for component_node in components {
                    self.collect_component_definition(component_node, state)?;
                }
                
                // Collect scripts
                for script_node in scripts {
                    self.collect_script_definition(script_node, state)?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn collect_style_definition(&mut self, ast: &AstNode, state: &mut CompilerState) -> Result<()> {
        if let AstNode::Style { name, extends, properties } = ast {
            // Check for duplicate style names
            if state.styles.iter().any(|s| s.source_name == *name) {
                return Err(CompilerError::semantic(
                    0,
                    format!("Style '{}' is already defined", name)
                ));
            }
            
            let style_id = (state.styles.len() + 1) as u8;
            let name_index = self.add_string_to_state(name, state)?;
            
            let mut style_entry = StyleEntry {
                id: style_id,
                source_name: name.clone(),
                name_index,
                extends_style_names: extends.clone(),
                properties: Vec::new(),
                source_properties: Vec::new(),
                calculated_size: 3, // Base size
                is_resolved: false,
                is_resolving: false,
            };
            
            // Convert AST properties to source properties
            for ast_prop in properties {
                style_entry.source_properties.push(SourceProperty {
                    key: ast_prop.key.clone(),
                    value: ast_prop.value.clone(),
                    line_num: ast_prop.line,
                });
            }
            
            state.styles.push(style_entry);
            state.header_flags |= FLAG_HAS_STYLES;
        }
        
        Ok(())
    }
    
    fn collect_component_definition(&mut self, ast: &AstNode, state: &mut CompilerState) -> Result<()> {
        if let AstNode::Component { name, properties, template } = ast {
            // Check for duplicate component names
            if state.component_defs.iter().any(|c| c.name == *name) {
                return Err(CompilerError::semantic(
                    0,
                    format!("Component '{}' is already defined", name)
                ));
            }
            
            let mut component_def = ComponentDefinition {
                name: name.clone(),
                properties: Vec::new(),
                definition_start_line: 0,
                definition_root_element_index: None,
                calculated_size: 0,
                internal_template_element_offsets: HashMap::new(),
            };
            
            // Convert component properties
            for comp_prop in properties {
                component_def.properties.push(ComponentPropertyDef {
                    name: comp_prop.name.clone(),
                    value_type_hint: comp_prop.value_type_hint(),
                    default_value: comp_prop.default_value.clone().unwrap_or_default(),
                });
            }
            
            state.component_defs.push(component_def);
            state.header_flags |= FLAG_HAS_COMPONENT_DEFS;
        }
        
        Ok(())
    }
    
    fn collect_script_definition(&mut self, _ast: &AstNode, state: &mut CompilerState) -> Result<()> {
        // Scripts are handled by the script processor
        state.header_flags |= FLAG_HAS_SCRIPTS;
        Ok(())
    }
    
    fn resolve_dependencies(&mut self, state: &mut CompilerState) -> Result<()> {
        // Resolve style inheritance
        self.resolve_style_inheritance(state)?;
        
        // Validate component property references
        self.validate_component_properties(state)?;
        
        Ok(())
    }
    
    fn resolve_style_inheritance(&mut self, state: &mut CompilerState) -> Result<()> {
        let mut resolved = HashSet::new();
        let mut resolving = HashSet::new();
        
        // Resolve each style
        for i in 0..state.styles.len() {
            if !resolved.contains(&i) {
                self.resolve_style_recursive(i, state, &mut resolved, &mut resolving)?;
            }
        }
        
        Ok(())
    }
    
    fn resolve_style_recursive(
        &mut self,
        style_index: usize,
        state: &mut CompilerState,
        resolved: &mut HashSet<usize>,
        resolving: &mut HashSet<usize>,
    ) -> Result<()> {
        if resolved.contains(&style_index) {
            return Ok(());
        }
        
        if resolving.contains(&style_index) {
            let style_name = &state.styles[style_index].source_name;
            return Err(CompilerError::semantic(
                0,
                format!("Circular dependency in style inheritance involving '{}'", style_name)
            ));
        }
        
        resolving.insert(style_index);
        
        // Get the extends list (need to clone to avoid borrow checker issues)
        let extends_names = state.styles[style_index].extends_style_names.clone();
        
        // Resolve base styles first
        for base_style_name in &extends_names {
            if let Some(base_index) = state.styles.iter().position(|s| s.source_name == *base_style_name) {
                self.resolve_style_recursive(base_index, state, resolved, resolving)?;
            } else {
                return Err(CompilerError::semantic(
                    0,
                    format!("Style '{}' extends undefined style '{}'", 
                           state.styles[style_index].source_name, base_style_name)
                ));
            }
        }
        
        // Now resolve this style by inheriting from base styles
        let mut inherited_properties = Vec::new();
        
        for base_style_name in &extends_names {
            if let Some(base_style) = state.styles.iter().find(|s| s.source_name == *base_style_name) {
                // Inherit properties from base style
                for prop in &base_style.properties {
                    // Only add if not already overridden
                    if !inherited_properties.iter().any(|p: &KrbProperty| p.property_id == prop.property_id) {
                        inherited_properties.push(prop.clone());
                    }
                }
            }
        }
        
        // Add own properties (they override inherited ones)
        for source_prop in &state.styles[style_index].source_properties.clone() {
            if let Ok(krb_prop) = self.convert_source_property_to_krb(source_prop, state) {
                // Remove any inherited property with the same ID
                inherited_properties.retain(|p| p.property_id != krb_prop.property_id);
                inherited_properties.push(krb_prop);
            }
        }
        
        // Update the style with resolved properties
        state.styles[style_index].properties = inherited_properties;
        state.styles[style_index].is_resolved = true;
        
        resolving.remove(&style_index);
        resolved.insert(style_index);
        
        Ok(())
    }
    
    fn validate_component_properties(&mut self, state: &CompilerState) -> Result<()> {
        for component in &state.component_defs {
            for prop_def in &component.properties {
                // Validate property type
                match prop_def.value_type_hint {
                    ValueType::String | ValueType::Int | ValueType::Float | 
                    ValueType::Bool | ValueType::Color | ValueType::Resource => {
                        // Basic types are always valid
                    }
                    ValueType::StyleId => {
                        // Could validate that default value references existing style
                        if !prop_def.default_value.is_empty() {
                            let style_name = prop_def.default_value.trim_matches('"');
                            if !state.styles.iter().any(|s| s.source_name == style_name) {
                                self.warnings.push(format!(
                                    "Component '{}' property '{}' defaults to undefined style '{}'",
                                    component.name, prop_def.name, style_name
                                ));
                            }
                        }
                    }
                    ValueType::Enum => {
                        // Could validate enum values
                    }
                    _ => {}
                }
            }
        }
        
        Ok(())
    }
    
    fn validate_elements(&mut self, ast: &AstNode, state: &CompilerState) -> Result<()> {
        match ast {
            AstNode::File { app, components, .. } => {
                // Validate main app
                if let Some(app_node) = app {
                    self.validate_element_recursive(app_node, state, None)?;
                }
                
                // Validate component templates
                for component_node in components {
                    if let AstNode::Component { template, .. } = component_node {
                        self.validate_element_recursive(template, state, None)?;
                    }
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn validate_element_recursive(
        &mut self,
        ast: &AstNode,
        state: &CompilerState,
        parent_type: Option<&str>
    ) -> Result<()> {
        if let AstNode::Element { element_type, properties, children, .. } = ast {
            // Validate element type
            let elem_type = ElementType::from_name(element_type);
            
            // Validate properties for this element type
            for prop in properties {
                self.validate_property(element_type, prop, state)?;
            }
            
            // Validate parent-child relationships
            if let Some(parent) = parent_type {
                self.validate_parent_child_relationship(parent, element_type)?;
            }
            
            // Recursively validate children
            for child in children {
                self.validate_element_recursive(child, state, Some(element_type))?;
            }
        }
        
        Ok(())
    }
    
    fn validate_property(&mut self, element_type: &str, prop: &AstProperty, _state: &CompilerState) -> Result<()> {
        // Validate property is valid for this element type
        let is_valid = match element_type {
            "App" => self.is_valid_app_property(&prop.key),
            "Text" => self.is_valid_text_property(&prop.key),
            "Button" => self.is_valid_button_property(&prop.key),
            "Input" => self.is_valid_input_property(&prop.key),
            "Image" => self.is_valid_image_property(&prop.key),
            "Container" => self.is_valid_container_property(&prop.key),
            _ => true, // Unknown element types accept any property
        };
        
        if !is_valid {
            return Err(CompilerError::semantic(
                prop.line,
                format!("Property '{}' is not valid for element type '{}'", prop.key, element_type)
            ));
        }
        
        // Validate property value format
        self.validate_property_value(&prop.key, &prop.value, prop.line)?;
        
        Ok(())
    }
    
    fn validate_property_value(&mut self, key: &str, value: &str, line: usize) -> Result<()> {
        match key {
            key if key.contains("color") => {
                if !value.starts_with('#') && !value.starts_with('$') && !value.starts_with('"') {
                    return Err(CompilerError::semantic(
                        line,
                        format!("Color property '{}' must be a hex color (#RGB), variable ($var), or string", key)
                    ));
                }
            }
            key if key.contains("width") || key.contains("height") || key.contains("size") => {
                if !value.chars().all(|c| c.is_ascii_digit()) && 
                   !value.starts_with('$') && !value.starts_with('"') {
                    return Err(CompilerError::semantic(
                        line,
                        format!("Size property '{}' must be a number, variable, or string", key)
                    ));
                }
            }
            "layout" => {
                // Validate layout values
                let cleaned_value = crate::utils::clean_and_quote_value(value).0;
                let valid_layouts = ["row", "column", "center", "start", "end", "grow", "wrap", "absolute"];
                let layout_parts: Vec<&str> = cleaned_value.split_whitespace().collect();
                
                for part in layout_parts {
                    if !valid_layouts.contains(&part) {
                        self.warnings.push(format!(
                            "Line {}: Unknown layout value '{}' in property '{}'",
                            line, part, key
                        ));
                    }
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn validate_parent_child_relationship(&mut self, parent_type: &str, child_type: &str) -> Result<()> {
        // Some basic validation rules
        match (parent_type, child_type) {
            ("Text", _) => {
                return Err(CompilerError::semantic(
                    0,
                    format!("Text elements cannot contain child elements, found '{}'", child_type)
                ));
            }
            ("Input", _) => {
                return Err(CompilerError::semantic(
                    0,
                    format!("Input elements cannot contain child elements, found '{}'", child_type)
                ));
            }
            ("Image", _) => {
                return Err(CompilerError::semantic(
                    0,
                    format!("Image elements cannot contain child elements, found '{}'", child_type)
                ));
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn check_unused_definitions(&mut self, state: &CompilerState) -> Result<()> {
        // Check for unused styles
        let mut used_styles: HashSet<String> = HashSet::new();

        // This would require traversing all elements to see which styles are referenced
        // For now, just warn about styles that don't extend anything and aren't extended
        for style in &state.styles {
            if style.extends_style_names.is_empty() {
                let is_extended = state.styles.iter()
                    .any(|s| s.extends_style_names.contains(&style.source_name));
                
                if !is_extended {
                    self.warnings.push(format!(
                        "Style '{}' is defined but may not be used", 
                        style.source_name
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    fn convert_source_property_to_krb(&self, prop: &SourceProperty, state: &CompilerState) -> Result<KrbProperty> {
        let property_id = self.get_property_id(&prop.key);
        let cleaned_value = crate::utils::clean_and_quote_value(&prop.value).0;
        
        match property_id {
            PropertyId::BackgroundColor | PropertyId::ForegroundColor | PropertyId::BorderColor => {
                if let Ok(color) = crate::utils::parse_color(&cleaned_value) {
                    Ok(KrbProperty {
                        property_id: property_id as u8,
                        value_type: ValueType::Color,
                        size: 4,
                        value: color.to_bytes().to_vec(),
                    })
                } else {
                    Err(CompilerError::semantic(
                        prop.line_num,
                        format!("Invalid color value: {}", cleaned_value)
                    ))
                }
            }
            PropertyId::TextContent | PropertyId::WindowTitle => {
                let string_index = state.strings.iter()
                    .position(|s| s.text == cleaned_value)
                    .unwrap_or(0) as u8;
                
                Ok(KrbProperty {
                    property_id: property_id as u8,
                    value_type: ValueType::String,
                    size: 1,
                    value: vec![string_index],
                })
            }
            PropertyId::BorderWidth | PropertyId::BorderRadius => {
                if let Ok(val) = cleaned_value.parse::<u8>() {
                    Ok(KrbProperty {
                        property_id: property_id as u8,
                        value_type: ValueType::Byte,
                        size: 1,
                        value: vec![val],
                    })
                } else {
                    Err(CompilerError::semantic(
                        prop.line_num,
                        format!("Invalid numeric value: {}", cleaned_value)
                    ))
                }
            }
            _ => {
                // Default handling - store as custom property
                Ok(KrbProperty {
                    property_id: PropertyId::CustomData as u8,
                    value_type: ValueType::String,
                    size: cleaned_value.len() as u8,
                    value: cleaned_value.into_bytes(),
                })
            }
        }
    }
    
    fn get_property_id(&self, key: &str) -> PropertyId {
        match key {
            "background_color" => PropertyId::BackgroundColor,
            "text_color" | "foreground_color" => PropertyId::ForegroundColor,
            "border_color" => PropertyId::BorderColor,
            "border_width" => PropertyId::BorderWidth,
            "border_radius" => PropertyId::BorderRadius,
            "padding" => PropertyId::Padding,
            "margin" => PropertyId::Margin,
            "text" => PropertyId::TextContent,
            "font_size" => PropertyId::FontSize,
            "font_weight" => PropertyId::FontWeight,
            "text_alignment" => PropertyId::TextAlignment,
            "src" | "image_source" => PropertyId::ImageSource,
            "opacity" => PropertyId::Opacity,
            "z_index" => PropertyId::ZIndex,
            "visible" | "visibility" => PropertyId::Visibility,
            "gap" => PropertyId::Gap,
            "window_width" => PropertyId::WindowWidth,
            "window_height" => PropertyId::WindowHeight,
            "window_title" => PropertyId::WindowTitle,
            "resizable" => PropertyId::Resizable,
            "cursor" => PropertyId::Cursor,
            _ => PropertyId::CustomData,
        }
    }
    
    // Property validation helpers
    fn is_valid_app_property(&self, key: &str) -> bool {
        matches!(key,
            "window_title" | "window_width" | "window_height" | "window_min_width" |
            "window_min_height" | "window_max_width" | "window_max_height" |
            "resizable" | "keep_aspect_ratio" | "scale_factor" | "icon" |
            "version" | "author" | "background_color" | "id" | "style"
        )
    }
    
    fn is_valid_text_property(&self, key: &str) -> bool {
        matches!(key,
            "text" | "text_color" | "font_size" | "font_weight" | "font_family" |
            "text_alignment" | "line_height" | "text_decoration" | "text_transform" |
            "id" | "pos_x" | "pos_y" | "width" | "height" | "style" |
            "background_color" | "border_color" | "border_width" | "border_radius" |
            "padding" | "margin" | "opacity" | "visibility" | "z_index"
        )
    }
    
    fn is_valid_button_property(&self, key: &str) -> bool {
        self.is_valid_text_property(key) || matches!(key,
            "disabled" | "onClick" | "onPress" | "onRelease" | "onHover" |
            "onFocus" | "onBlur" | "cursor"
        )
    }
    
    fn is_valid_input_property(&self, key: &str) -> bool {
        self.is_valid_text_property(key) || matches!(key,
            "placeholder" | "value" | "onChange" | "onSubmit" | "type" |
            "max_length" | "readonly" | "disabled"
        )
    }
    
    fn is_valid_image_property(&self, key: &str) -> bool {
        matches!(key,
            "src" | "alt" | "fit" | "id" | "pos_x" | "pos_y" | "width" | "height" |
            "style" | "background_color" | "border_color" | "border_width" |
            "border_radius" | "padding" | "margin" | "opacity" | "visibility" | "z_index"
        )
    }
    
    fn is_valid_container_property(&self, key: &str) -> bool {
        matches!(key,
            "layout" | "gap" | "id" | "pos_x" | "pos_y" | "width" | "height" |
            "min_width" | "min_height" | "max_width" | "max_height" |
            "style" | "background_color" | "border_color" | "border_width" |
            "border_radius" | "padding" | "margin" | "opacity" | "visibility" | "z_index"
        )
    }
    
    fn add_string_to_state(&self, text: &str, state: &mut CompilerState) -> Result<u8> {
        // Check if string already exists
        for (i, entry) in state.strings.iter().enumerate() {
            if entry.text == text {
                return Ok(i as u8);
            }
        }
        
        // Add new string
        if state.strings.len() >= MAX_STRINGS {
            return Err(CompilerError::LimitExceeded {
                limit_type: "strings".to_string(),
                limit: MAX_STRINGS,
            });
        }
        
        let index = state.strings.len() as u8;
        state.strings.push(StringEntry {
            text: text.to_string(),
            length: text.len(),
            index,
        });
        
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;
    
    #[test]
    fn test_style_inheritance_validation() {
        let mut analyzer = SemanticAnalyzer::new();
        let mut state = CompilerState::new();
        
        // Create a style that extends a non-existent style
        let mut style1 = StyleEntry {
            id: 1,
            source_name: "child".to_string(),
            name_index: 1,
            extends_style_names: vec!["nonexistent".to_string()],
            properties: Vec::new(),
            source_properties: Vec::new(),
            calculated_size: 3,
            is_resolved: false,
            is_resolving: false,
        };
        
        state.strings.push(StringEntry { text: "".to_string(), length: 0, index: 0 });
        state.strings.push(StringEntry { text: "child".to_string(), length: 5, index: 1 });
        state.styles.push(style1);
        
        let result = analyzer.resolve_style_inheritance(&mut state);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_circular_dependency_detection() {
        let mut analyzer = SemanticAnalyzer::new();
        let mut state = CompilerState::new();
        
        // Create circular dependency: style1 -> style2 -> style1
        state.strings.push(StringEntry { text: "".to_string(), length: 0, index: 0 });
        state.strings.push(StringEntry { text: "style1".to_string(), length: 6, index: 1 });
        state.strings.push(StringEntry { text: "style2".to_string(), length: 6, index: 2 });
        
        state.styles.push(StyleEntry {
            id: 1,
            source_name: "style1".to_string(),
            name_index: 1,
            extends_style_names: vec!["style2".to_string()],
            properties: Vec::new(),
            source_properties: Vec::new(),
            calculated_size: 3,
            is_resolved: false,
            is_resolving: false,
        });
        
        state.styles.push(StyleEntry {
            id: 2,
            source_name: "style2".to_string(),
            name_index: 2,
            extends_style_names: vec!["style1".to_string()],
            properties: Vec::new(),
            source_properties: Vec::new(),
            calculated_size: 3,
            is_resolved: false,
            is_resolving: false,
        });
        
        let result = analyzer.resolve_style_inheritance(&mut state);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_property_validation() {
        let mut analyzer = SemanticAnalyzer::new();
        let state = CompilerState::new();
        
        // Valid property
        let valid_prop = AstProperty::new("text".to_string(), "\"Hello\"".to_string(), 1);
        assert!(analyzer.validate_property("Text", &valid_prop, &state).is_ok());
        
        // Invalid property for element type
        let invalid_prop = AstProperty::new("onChange".to_string(), "handler".to_string(), 2);
        assert!(analyzer.validate_property("Text", &invalid_prop, &state).is_err());
    }
}
