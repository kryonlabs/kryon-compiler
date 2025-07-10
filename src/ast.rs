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
        fonts: Vec<AstNode>,
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
        pseudo_selectors: Vec<PseudoSelector>,
    },
    
    /// font declaration
    Font {
        name: String,
        path: String,
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
    pub value: PropertyValue,
    pub line: usize,
    pub template_variables: Vec<String>, // Variables used in {{}} syntax
    pub has_templates: bool, // Quick check if this property has template variables
}

/// Property value types
#[derive(Debug, Clone)]
pub enum PropertyValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Color(String),
    Pixels(f64),
    Em(f64),
    Rem(f64),
    ViewportWidth(f64),
    ViewportHeight(f64),
    Percentage(f64),
    Degrees(f64),
    Radians(f64),
    Turns(f64),
    Object(HashMap<String, PropertyValue>),
    Array(Vec<PropertyValue>),
    Variable(String), // Template variable reference
}

impl PropertyValue {
    /// Convert PropertyValue to string representation
    pub fn to_string(&self) -> String {
        match self {
            PropertyValue::String(s) => s.clone(),
            PropertyValue::Number(n) => n.to_string(),
            PropertyValue::Integer(i) => i.to_string(),
            PropertyValue::Boolean(b) => b.to_string(),
            PropertyValue::Color(c) => c.clone(),
            PropertyValue::Pixels(p) => format!("{}px", p),
            PropertyValue::Em(e) => format!("{}em", e),
            PropertyValue::Rem(r) => format!("{}rem", r),
            PropertyValue::ViewportWidth(vw) => format!("{}vw", vw),
            PropertyValue::ViewportHeight(vh) => format!("{}vh", vh),
            PropertyValue::Percentage(p) => format!("{}%", p),
            PropertyValue::Degrees(d) => format!("{}deg", d),
            PropertyValue::Radians(r) => format!("{}rad", r),
            PropertyValue::Turns(t) => format!("{}turn", t),
            PropertyValue::Object(_) => "[Object]".to_string(),
            PropertyValue::Array(_) => "[Array]".to_string(),
            PropertyValue::Variable(v) => format!("${}", v),
        }
    }
    
    /// Check if this value contains template variables
    pub fn has_variables(&self) -> bool {
        match self {
            PropertyValue::Variable(_) => true,
            PropertyValue::String(s) => s.contains('$'),
            PropertyValue::Object(obj) => obj.values().any(|v| v.has_variables()),
            PropertyValue::Array(arr) => arr.iter().any(|v| v.has_variables()),
            _ => false,
        }
    }
    
    /// Extract template variables from this value
    pub fn extract_variables(&self) -> Vec<String> {
        let mut variables = Vec::new();
        match self {
            PropertyValue::Variable(v) => variables.push(v.clone()),
            PropertyValue::String(s) => {
                // Extract $variable patterns from string
                // This is simplified - would need proper regex parsing
                if s.contains('$') {
                    // TODO: Implement proper variable extraction
                }
            },
            PropertyValue::Object(obj) => {
                for value in obj.values() {
                    variables.extend(value.extract_variables());
                }
            },
            PropertyValue::Array(arr) => {
                for value in arr {
                    variables.extend(value.extract_variables());
                }
            },
            _ => {}
        }
        variables
    }
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
    pub fn new(key: String, value: PropertyValue, line: usize) -> Self {
        Self { 
            key, 
            value, 
            line,
            template_variables: Vec::new(),
            has_templates: false,
        }
    }
    
    pub fn new_with_templates(key: String, value: PropertyValue, line: usize, template_variables: Vec<String>) -> Self {
        let has_templates = !template_variables.is_empty();
        Self { 
            key, 
            value, 
            line,
            template_variables,
            has_templates,
        }
    }
    
    /// Get the cleaned value (without quotes if it was quoted)
    pub fn cleaned_value(&self) -> String {
        match &self.value {
            PropertyValue::String(s) => crate::utils::clean_and_quote_value(s).0,
            _ => self.value.to_string(),
        }
    }
    
    /// Check if this property value was quoted
    pub fn was_quoted(&self) -> bool {
        match &self.value {
            PropertyValue::String(s) => crate::utils::clean_and_quote_value(s).1,
            _ => false,
        }
    }
    
    /// Get raw string value (for compatibility)
    pub fn value_string(&self) -> String {
        self.value.to_string()
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