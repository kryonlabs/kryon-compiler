//! Abstract Syntax Tree types for the Kryon compiler

use crate::types::*;
use std::collections::HashMap;

/// AST node types
#[derive(Debug, Clone)]
pub enum AstNode {
    /// Root file node
    File {
        directives: Vec<AstNode>,
        styles: Vec<AstNode>,
        components: Vec<AstNode>,
        scripts: Vec<AstNode>,
        app: Option<Box<AstNode>>,
    },
    
    /// @include directive
    Include {
        path: String,
    },
    
    /// @variables block
    Variables {
        variables: HashMap<String, String>,
    },
    
    /// @script directive
    Script {
        language: String,
        name: Option<String>,
        source: ScriptSource,
        mode: Option<String>,
    },
    
    /// style definition
    Style {
        name: String,
        extends: Vec<String>,
        properties: Vec<AstProperty>,
    },
    
    /// Define component
    Component {
        name: String,
        properties: Vec<ComponentProperty>,
        template: Box<AstNode>,
    },
    
    /// Properties block (in component definition)
    Properties {
        properties: Vec<ComponentProperty>,
    },
    
    /// Element (App, Container, Text, etc.)
    Element {
        element_type: String,
        properties: Vec<AstProperty>,
        pseudo_selectors: Vec<PseudoSelector>,
        children: Vec<AstNode>,
    },
}

/// Property in AST
#[derive(Debug, Clone)]
pub struct AstProperty {
    pub key: String,
    pub value: String,
    pub line: usize,
}

/// Component property definition
#[derive(Debug, Clone)]
pub struct ComponentProperty {
    pub name: String,
    pub property_type: String,
    pub default_value: Option<String>,
    pub line: usize,
}

/// Pseudo-selector (e.g., &:hover)
#[derive(Debug, Clone)]
pub struct PseudoSelector {
    pub state: String,
    pub properties: Vec<AstProperty>,
    pub line: usize,
}

/// Script source type
#[derive(Debug, Clone)]
pub enum ScriptSource {
    Inline(String),
    External(String),
}

impl AstProperty {
    pub fn new(key: String, value: String, line: usize) -> Self {
        Self { key, value, line }
    }
    
    /// Get the cleaned value (without quotes if it was quoted)
    pub fn cleaned_value(&self) -> String {
        crate::utils::clean_and_quote_value(&self.value).0
    }
    
    /// Check if this property value was quoted
    pub fn was_quoted(&self) -> bool {
        crate::utils::clean_and_quote_value(&self.value).1
    }
}

impl ComponentProperty {
    pub fn new(name: String, property_type: String, default_value: Option<String>, line: usize) -> Self {
        Self {
            name,
            property_type,
            default_value,
            line,
        }
    }
    
    /// Get the value type hint
    pub fn value_type_hint(&self) -> ValueType {
        match self.property_type.as_str() {
            "String" => ValueType::String,
            "Int" => ValueType::Int,
            "Float" => ValueType::Float,
            "Bool" => ValueType::Bool,
            "Color" => ValueType::Color,
            "StyleID" => ValueType::StyleId,
            "Resource" => ValueType::Resource,
            _ if self.property_type.starts_with("Enum(") => ValueType::Enum,
            _ => ValueType::Custom,
        }
    }
}

impl PseudoSelector {
    pub fn new(state: String, properties: Vec<AstProperty>, line: usize) -> Self {
        Self { state, properties, line }
    }
    
    /// Get the state flag for this pseudo-selector
    pub fn state_flag(&self) -> Option<u8> {
        match self.state.as_str() {
            "hover" => Some(STATE_HOVER),
            "active" => Some(STATE_ACTIVE),
            "focus" => Some(STATE_FOCUS),
            "disabled" => Some(STATE_DISABLED),
            "checked" => Some(STATE_CHECKED),
            _ => None,
        }
    }
}