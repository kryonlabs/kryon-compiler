//! Recursive descent parser for the KRY language

use crate::ast::*;
use crate::error::{CompilerError, Result};
use crate::lexer::{Token, TokenType};
use std::collections::HashMap;
use regex::Regex;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
        }
    }
    
    pub fn parse(&mut self) -> Result<AstNode> {
        let mut directives = Vec::new();
        let mut styles = Vec::new();
        let mut fonts = Vec::new();
        let mut components = Vec::new();
        let mut scripts = Vec::new();
        let mut app = None;
        let mut standalone_elements = Vec::new(); // Collect standalone elements for auto-wrapping
        
        while !self.is_at_end() {
            // Skip newlines
            if self.match_token(&TokenType::Newline) {
                continue;
            }
            
            // Skip comments
            if matches!(self.peek().token_type, TokenType::Comment(_)) {
                self.advance();
                continue;
            }
            
            match &self.peek().token_type {
                TokenType::Include => {
                    directives.push(self.parse_include()?);
                }
                TokenType::Variables => {
                    directives.push(self.parse_variables()?);
                }
                TokenType::Script => {
                    scripts.push(self.parse_script()?);
                }
                TokenType::Function => {
                    scripts.push(self.parse_function()?);
                }
                TokenType::Style => {
                    styles.push(self.parse_style()?);
                }
                TokenType::Font => {
                    fonts.push(self.parse_font()?);
                }
                TokenType::Define => {
                    components.push(self.parse_component()?);
                }
                TokenType::App => {
                    if app.is_some() {
                        return Err(CompilerError::parse_legacy(
                            self.peek().line,
                            "Multiple App elements found. Only one App element is allowed."
                        ));
                    }
                    app = Some(Box::new(self.parse_element()?));
                }
                _ => {
                    // Try parsing as element (for component usage at root level or standalone elements)
                    if self.is_element_start() {
                        if app.is_some() {
                            return Err(CompilerError::parse_legacy(
                                self.peek().line,
                                "Only one root element (App or component) is allowed."
                            ));
                        }
                        
                        let element = self.parse_element()?;
                        // Check if this is a standalone element that needs App wrapping
                        if let AstNode::Element { element_type, .. } = &element {
                            if element_type == "App" {
                                app = Some(Box::new(element));
                            } else {
                                // This is a standalone element - collect it for auto-wrapping
                                standalone_elements.push(element);
                            }
                        } else {
                            app = Some(Box::new(element));
                        }
                    } else {
                        return Err(CompilerError::parse_legacy(
                            self.peek().line,
                            format!("Unexpected token: {}", self.peek().token_type)
                        ));
                    }
                }
            }
        }
        
        // Auto-create App wrapper if none exists and we have standalone elements at root level
        let app = if app.is_none() && !standalone_elements.is_empty() {
            self.create_default_app_wrapper(standalone_elements)?
        } else {
            app
        };
        
        Ok(AstNode::File {
            directives,
            styles,
            fonts,
            components,
            scripts,
            app,
        })
    }
    
    fn parse_include(&mut self) -> Result<AstNode> {
        self.consume(TokenType::Include, "Expected @include")?;
        
        let path = match &self.advance().token_type {
            TokenType::String(s) => s.clone(),
            _ => return Err(CompilerError::parse_legacy(
                self.previous().line,
                "Expected string path after @include"
            )),
        };
        
        Ok(AstNode::Include { path })
    }

    fn parse_variables(&mut self) -> Result<AstNode> {
        self.consume(TokenType::Variables, "Expected @variables")?;
        self.consume(TokenType::LeftBrace, "Expected '{' after @variables")?;
        
        let mut variables = HashMap::new();
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {            
            if self.match_token(&TokenType::Newline) {
                continue;
            }
            if matches!(self.peek().token_type, TokenType::Comment(_)) {
                self.advance();
                continue;
            }
            let name = match &self.advance().token_type {
                TokenType::Identifier(name) => name.clone(),
                _ => return Err(CompilerError::parse_legacy(
                    self.previous().line,
                    "Expected variable name"
                )),
            };
            
            self.consume(TokenType::Colon, "Expected ':' after variable name")?;
            
            let value = self.parse_value()?;
            variables.insert(name, value.to_string());
        }
        
        self.consume(TokenType::RightBrace, "Expected '}' after variables")?;
        
        Ok(AstNode::Variables { variables })
    }
    
    fn parse_script(&mut self) -> Result<AstNode> {
        self.consume(TokenType::Script, "Expected @script")?;
        
        let language = match &self.advance().token_type {
            TokenType::String(lang) => lang.clone(),
            _ => return Err(CompilerError::parse_legacy(
                self.previous().line,
                "Expected language string after @script"
            )),
        };
        
        let mut name = None;
        let mut mode = None;
        let mut source = None;
        
        // Parse optional attributes and source
        while !self.is_at_end() && !self.check(&TokenType::LeftBrace) {
            if self.match_token(&TokenType::Newline) {
                continue;
            }
            
            if let TokenType::Identifier(attr) = &self.peek().token_type {
                match attr.as_str() {
                    "name" => {
                        self.advance();
                        self.consume(TokenType::Equals, "Expected '=' after name")?;
                        name = Some(self.parse_value()?.to_string());
                    }
                    "mode" => {
                        self.advance();
                        self.consume(TokenType::Equals, "Expected '=' after mode")?;
                        mode = Some(self.parse_value()?.to_string());
                    }
                    "from" => {
                        self.advance();
                        let file_path = self.parse_value()?;
                        source = Some(ScriptSource::External(file_path.to_string()));
                    }
                    _ => break,
                }
            } else {
                break;
            }
        }
        
        // Parse inline code if no external source
        if source.is_none() {
            self.consume(TokenType::LeftBrace, "Expected '{' for script code")?;
            let code = self.parse_script_code()?;
            source = Some(ScriptSource::Inline(code));
            // Note: parse_script_code already handles the closing brace
        }
        
        Ok(AstNode::Script {
            language,
            name,
            source: source.unwrap(),
            mode,
        })
    }
    
    fn parse_script_code(&mut self) -> Result<String> {
        // Check if we have a ScriptContent token (new approach)
        if let TokenType::ScriptContent(content) = &self.peek().token_type {
            let content = content.clone();
            self.advance(); // consume the ScriptContent token
            // Also consume the closing brace that should follow
            self.consume(TokenType::RightBrace, "Expected '}' after script content")?;
            return Ok(content);
        }
        
        // Fallback to old approach for compatibility
        let mut code = String::new();
        let mut brace_count = 1;
        let mut last_was_space = false;
        
        while !self.is_at_end() && brace_count > 0 {
            match &self.advance().token_type {
                TokenType::LeftBrace => {
                    brace_count += 1;
                    code.push('{');
                    last_was_space = false;
                }
                TokenType::RightBrace => {
                    brace_count -= 1;
                    if brace_count > 0 {
                        code.push('}');
                        last_was_space = false;
                    }
                }
                TokenType::String(s) => {
                    code.push_str(&format!("\"{}\"", s));
                    last_was_space = false;
                }
                TokenType::Identifier(id) => {
                    if !last_was_space && !code.is_empty() && !code.ends_with(|c: char| c.is_whitespace() || c == '(' || c == '{' || c == ',' || c == '=' || c == '.') {
                        code.push(' ');
                    }
                    code.push_str(id);
                    last_was_space = false;
                }
                TokenType::Number(n) => {
                    if !last_was_space && !code.is_empty() && !code.ends_with(|c: char| c.is_whitespace() || c == '(' || c == '{' || c == ',' || c == '=' || c == '.') {
                        code.push(' ');
                    }
                    code.push_str(&n.to_string());
                    last_was_space = false;
                }
                TokenType::Integer(i) => {
                    if !last_was_space && !code.is_empty() && !code.ends_with(|c: char| c.is_whitespace() || c == '(' || c == '{' || c == ',' || c == '=' || c == '.') {
                        code.push(' ');
                    }
                    code.push_str(&i.to_string());
                    last_was_space = false;
                }
                TokenType::Newline => {
                    code.push('\n');
                    last_was_space = true;
                }
                TokenType::Dot => {
                    code.push('.');
                    last_was_space = false;
                }
                TokenType::LeftParen => {
                    code.push('(');
                    last_was_space = false;
                }
                TokenType::RightParen => {
                    code.push(')');
                    last_was_space = false;
                }
                TokenType::Comma => {
                    code.push(',');
                    last_was_space = false;
                }
                TokenType::Equals => {
                    code.push_str(" = ");
                    last_was_space = true;
                }
                TokenType::Comment(comment) => {
                    code.push_str(&format!("--{}", comment));
                    last_was_space = false;
                }
                token => {
                    // Handle any other tokens by converting to string
                    let token_str = format!("{}", token);
                    if !token_str.is_empty() {
                        if !last_was_space && !code.is_empty() && !code.ends_with(|c: char| c.is_whitespace()) {
                            code.push(' ');
                        }
                        code.push_str(&token_str);
                        last_was_space = false;
                    }
                }
            }
        }
        
        Ok(code.trim().to_string())
    }
    
    fn parse_function(&mut self) -> Result<AstNode> {
        self.consume(TokenType::Function, "Expected @function")?;
        
        let language = match &self.advance().token_type {
            TokenType::String(lang) => lang.clone(),
            _ => return Err(CompilerError::parse_legacy(
                self.previous().line,
                "Expected language string after @function"
            )),
        };
        
        // Parse function name and parameters
        let function_name = match &self.advance().token_type {
            TokenType::Identifier(name) => name.clone(),
            _ => return Err(CompilerError::parse_legacy(
                self.previous().line,
                "Expected function name"
            )),
        };
        
        // Parse parameter list
        self.consume(TokenType::LeftParen, "Expected '(' after function name")?;
        let mut parameters = Vec::new();
        
        while !self.check(&TokenType::RightParen) && !self.is_at_end() {
            if let TokenType::Identifier(param) = &self.advance().token_type {
                parameters.push(param.clone());
                if self.match_token(&TokenType::Comma) {
                    continue;
                }
            } else {
                return Err(CompilerError::parse_legacy(
                    self.previous().line,
                    "Expected parameter name"
                ));
            }
        }
        
        self.consume(TokenType::RightParen, "Expected ')' after parameters")?;
        
        // Parse function body
        self.consume(TokenType::LeftBrace, "Expected '{' for function body")?;
        let body = self.parse_script_code()?;
        
        // Generate the full function code
        let param_list = parameters.join(", ");
        let full_code = format!("function {}({})\n{}\nend", function_name, param_list, body);
        
        // Return as a script node with the function name as the script name
        Ok(AstNode::Script {
            language,
            name: Some(function_name.clone()),
            source: ScriptSource::Inline(full_code),
            mode: None,
        })
    }
    
    fn parse_style(&mut self) -> Result<AstNode> {
        self.consume(TokenType::Style, "Expected 'style'")?;
        

        let name = if let TokenType::String(name) = &self.peek().token_type {
            name.clone()
        } else {
            return Err(CompilerError::parse_legacy(
                self.peek().line,
                format!("Expected style name string, but found {}", self.peek().token_type)
            ));
        };
        self.advance();

        self.consume(TokenType::LeftBrace, "Expected '{' after style name")?;
        
        let mut extends = Vec::new();
        let mut properties = Vec::new();
        let mut pseudo_selectors = Vec::new();
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            if self.match_token(&TokenType::Newline) {
                continue;
            }
            if matches!(self.peek().token_type, TokenType::Comment(_)) {
                self.advance();
                continue;
            }
            
            // Check for pseudo-selectors
            if matches!(self.peek().token_type, TokenType::PseudoSelector(_)) {
                pseudo_selectors.push(self.parse_pseudo_selector()?);
            } else {
                let prop = self.parse_property()?;
                
                // Handle extends specially
                if prop.key == "extends" {
                    extends = self.parse_extends_value(&prop.value)?;
                } else {
                    properties.push(prop);
                }
            }
        }
        
        self.consume(TokenType::RightBrace, "Expected '}' after style properties")?;
        
        Ok(AstNode::Style {
            name,
            extends,
            properties,
            pseudo_selectors,
        })
    }
    
    fn parse_extends_value(&self, value: &PropertyValue) -> Result<Vec<String>> {
        match value {
            PropertyValue::Array(arr) => {
                let mut extends = Vec::new();
                for item in arr {
                    match item {
                        PropertyValue::String(s) => {
                            // Remove quotes if present
                            let cleaned = if s.starts_with('"') && s.ends_with('"') {
                                s[1..s.len()-1].to_string()
                            } else {
                                s.clone()
                            };
                            extends.push(cleaned);
                        }
                        _ => extends.push(item.to_string()),
                    }
                }
                Ok(extends)
            }
            PropertyValue::String(s) => {
                let trimmed = s.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    // Array format: ["style1", "style2"]
                    let content = &trimmed[1..trimmed.len()-1];
                    let mut extends = Vec::new();
                    
                    for item in content.split(',') {
                        let item = item.trim();
                        if item.starts_with('"') && item.ends_with('"') {
                            extends.push(item[1..item.len()-1].to_string());
                        } else if !item.is_empty() {
                            extends.push(item.to_string());
                        }
                    }
                    
                    Ok(extends)
                } else if trimmed.starts_with('"') && trimmed.ends_with('"') {
                    // Single quoted string
                    Ok(vec![trimmed[1..trimmed.len()-1].to_string()])
                } else {
                    // Single unquoted string
                    Ok(vec![trimmed.to_string()])
                }
            }
            _ => Ok(vec![value.to_string()]),
        }
    }
    
    fn parse_font(&mut self) -> Result<AstNode> {
        self.consume(TokenType::Font, "Expected 'font'")?;
        
        // Parse font name (first string)
        let name = if let TokenType::String(name) = &self.peek().token_type {
            name.clone()
        } else {
            return Err(CompilerError::parse_legacy(
                self.peek().line,
                format!("Expected font name string, but found {}", self.peek().token_type)
            ));
        };
        self.advance();
        
        // Parse font path (second string)
        let path = if let TokenType::String(path) = &self.peek().token_type {
            path.clone()
        } else {
            return Err(CompilerError::parse_legacy(
                self.peek().line,
                format!("Expected font path string, but found {}", self.peek().token_type)
            ));
        };
        self.advance();
        
        Ok(AstNode::Font {
            name,
            path,
        })
    }
    
    fn parse_component(&mut self) -> Result<AstNode> {
        self.consume(TokenType::Define, "Expected 'Define'")?;
        
        let name = if let TokenType::Identifier(name) = &self.peek().token_type {
            name.clone()
        } else {
            return Err(CompilerError::parse_legacy(
                self.peek().line,
                "Expected component name after 'Define'"
            ));
        };
        self.advance();
        
        self.consume(TokenType::LeftBrace, "Expected '{' after component name")?;
        
        let mut properties = Vec::new();
        let mut template = None;
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            if self.match_token(&TokenType::Newline) {
                continue;
            }
            if matches!(self.peek().token_type, TokenType::Comment(_)) {
                self.advance();
                continue;
            }

            
            if self.match_token(&TokenType::Properties) {
                properties = self.parse_component_properties()?;
            } else if self.is_element_start() {
                if template.is_some() {
                    return Err(CompilerError::parse_legacy(
                        self.peek().line,
                        "Component can only have one root template element"
                    ));
                }
                template = Some(Box::new(self.parse_element()?));
            } else {
                return Err(CompilerError::parse_legacy(
                    self.peek().line,
                    "Expected 'Properties' block or template element in component"
                ));
            }
        }
        
        self.consume(TokenType::RightBrace, "Expected '}' after component definition")?;
        
        let template = template.ok_or_else(|| CompilerError::parse_legacy(
            self.previous().line,
            "Component must have a template element"
        ))?;
        
        Ok(AstNode::Component {
            name,
            properties,
            template,
        })
    }
    
    fn parse_component_properties(&mut self) -> Result<Vec<ComponentProperty>> {
        self.consume(TokenType::LeftBrace, "Expected '{' after 'Properties'")?;
        
        let mut properties = Vec::new();
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            if self.match_token(&TokenType::Newline) {
                continue;
            }
            if matches!(self.peek().token_type, TokenType::Comment(_)) {
                self.advance();
                continue;
            }
            
            let name = match &self.advance().token_type {
                TokenType::Identifier(name) => name.clone(),
                _ => return Err(CompilerError::parse_legacy(
                    self.previous().line,
                    "Expected property name"
                )),
            };
            
            self.consume(TokenType::Colon, "Expected ':' after property name")?;
            
            let property_type = match &self.advance().token_type {
                TokenType::Identifier(type_name) => type_name.clone(),
                _ => return Err(CompilerError::parse_legacy(
                    self.previous().line,
                    "Expected property type"
                )),
            };
            
            let mut default_value = None;
            if self.match_token(&TokenType::Equals) {
                default_value = Some(self.parse_value()?.to_string());
            }
            
            properties.push(ComponentProperty::new(
                name,
                property_type,
                default_value,
                self.previous().line,
            ));
        }
        
        self.consume(TokenType::RightBrace, "Expected '}' after component properties")?;
        
        Ok(properties)
    }
    
    fn parse_element(&mut self) -> Result<AstNode> {
        let element_type = match &self.peek().token_type {
            TokenType::App => "App".to_string(),
            TokenType::Container => "Container".to_string(),
            TokenType::Text => "Text".to_string(),
            TokenType::Image => "Image".to_string(),
            TokenType::Button => "Button".to_string(),
            TokenType::Input => "Input".to_string(),
            TokenType::Identifier(name) => name.clone(),
            _ => return Err(CompilerError::parse_legacy(
                self.peek().line,
                "Expected element type"
            )),
        };
        self.advance(); 
        
        let mut properties = Vec::new();
        let mut pseudo_selectors = Vec::new();
        let mut children = Vec::new();
        
        if self.match_token(&TokenType::LeftBrace) {
            while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
                if self.match_token(&TokenType::Newline) {
                    continue;
                }
                if matches!(self.peek().token_type, TokenType::Comment(_)) {
                    self.advance();
                    continue;
                }

                if matches!(self.peek().token_type, TokenType::PseudoSelector(_)) {
                    pseudo_selectors.push(self.parse_pseudo_selector()?);
                } else if self.is_property() {
                    properties.push(self.parse_property()?);
                } else if self.is_element_start() {
                    children.push(self.parse_element()?);
                } else {
                    return Err(CompilerError::parse_legacy(
                        self.peek().line,
                        format!("Unexpected token in element body: {}", self.peek().token_type)
                    ));
                }
            }
            
            self.consume(TokenType::RightBrace, "Expected '}' after element body")?;
        }
        
        Ok(AstNode::Element {
            element_type,
            properties,
            pseudo_selectors,
            children,
        })
    }
    
    fn parse_pseudo_selector(&mut self) -> Result<PseudoSelector> {
        // Extract the state from the pseudo-selector token
        let state = match &self.advance().token_type {
            TokenType::PseudoSelector(state) => state.clone(),
            _ => return Err(CompilerError::parse_legacy(
                self.previous().line,
                "Expected pseudo-selector"
            )),
        };
        
        self.consume(TokenType::LeftBrace, "Expected '{' after pseudo-selector")?;
        
        let mut properties = Vec::new();
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            if self.match_token(&TokenType::Newline) {
                continue;
            }
            properties.push(self.parse_property()?);
        }
        
        self.consume(TokenType::RightBrace, "Expected '}' after pseudo-selector properties")?;
        
        Ok(PseudoSelector::new(state, properties, self.previous().line))
    }

    fn parse_property(&mut self) -> Result<AstProperty> {
        let key = match &self.peek().token_type {
            TokenType::Identifier(name) => name.clone(),
            // Allow keywords as property names
            TokenType::Style => "style".to_string(),
            TokenType::Text => "text".to_string(), 
            TokenType::Image => "image".to_string(),
            TokenType::Button => "button".to_string(),
            TokenType::Input => "input".to_string(),
            TokenType::Container => "container".to_string(),
            TokenType::App => "app".to_string(),
            _ => {
                return Err(CompilerError::parse_legacy(
                    self.peek().line,
                    format!("Expected property name, but found token: {}", self.peek().token_type)
                ));
            }
        };
        self.advance(); // Now consume the token we just processed
    
        self.consume(TokenType::Colon, "Expected ':' after property name")?;
        
        let value = self.parse_value()?;
        
        // Optional semicolon
        self.match_token(&TokenType::Semicolon);
        
        // Extract template variables from the value
        let template_variables = value.extract_variables();
        
        if template_variables.is_empty() {
            Ok(AstProperty::new(key, value, self.previous().line))
        } else {
            Ok(AstProperty::new_with_templates(key, value, self.previous().line, template_variables))
        }
    }
    
    fn parse_value(&mut self) -> Result<PropertyValue> {
        let result = match &self.peek().token_type {
            TokenType::String(s) => {
                let value = format!("\"{}\"", s);
                self.advance();
                Ok(PropertyValue::String(value))
            }
            TokenType::Number(n) => {
                let value = *n;
                self.advance();
                Ok(PropertyValue::Number(value))
            }
            TokenType::Integer(i) => {
                let value = *i;
                self.advance();
                Ok(PropertyValue::Integer(value))
            }
            TokenType::Pixels(p) => {
                let value = *p;
                self.advance();
                Ok(PropertyValue::Pixels(value))
            }
            TokenType::Em(e) => {
                let value = *e;
                self.advance();
                Ok(PropertyValue::Em(value))
            }
            TokenType::Rem(r) => {
                let value = *r;
                self.advance();
                Ok(PropertyValue::Rem(value))
            }
            TokenType::ViewportWidth(vw) => {
                let value = *vw;
                self.advance();
                Ok(PropertyValue::ViewportWidth(value))
            }
            TokenType::ViewportHeight(vh) => {
                let value = *vh;
                self.advance();
                Ok(PropertyValue::ViewportHeight(value))
            }
            TokenType::Percentage(p) => {
                let value = *p;
                self.advance();
                Ok(PropertyValue::Percentage(value))
            }
            TokenType::Degrees(d) => {
                let value = *d;
                self.advance();
                Ok(PropertyValue::Degrees(value))
            }
            TokenType::Radians(r) => {
                let value = *r;
                self.advance();
                Ok(PropertyValue::Radians(value))
            }
            TokenType::Turns(t) => {
                let value = *t;
                self.advance();
                Ok(PropertyValue::Turns(value))
            }
            TokenType::Boolean(b) => {
                let value = *b;
                self.advance();
                Ok(PropertyValue::Boolean(value))
            }
            TokenType::Color(c) => {
                let value = c.clone();
                self.advance();
                Ok(PropertyValue::Color(value))
            }
            TokenType::TemplateVariable(var) => {
                let value = var.clone();
                self.advance();
                Ok(PropertyValue::Variable(value))
            }
            TokenType::LeftBrace => {
                // Parse object literal
                self.parse_object_literal()
            }
            TokenType::LeftBracket => {
                // Parse array literal
                self.parse_array_literal()
            }
            TokenType::Identifier(id) => {
                let value = id.clone();
                self.advance();
                Ok(PropertyValue::String(value))
            }
            _ => {
                Err(CompilerError::parse_legacy(
                    self.peek().line,
                    format!("Expected a value, but found {}", self.peek().token_type)
                ))
            }
        };
        result
    }
    
    /// Parse object literal: { key: value, key: value }
    fn parse_object_literal(&mut self) -> Result<PropertyValue> {
        self.consume(TokenType::LeftBrace, "Expected '{'")?;
        let mut object = HashMap::new();
        
        // Skip newlines after opening brace
        while self.match_token(&TokenType::Newline) {}
        
        // Parse key-value pairs
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            // Skip newlines
            if self.match_token(&TokenType::Newline) {
                continue;
            }
            
            // Parse key
            let key = match &self.peek().token_type {
                TokenType::Identifier(id) => {
                    let key = id.clone();
                    self.advance();
                    key
                }
                _ => return Err(CompilerError::parse_legacy(
                    self.peek().line,
                    "Expected property name in object"
                )),
            };
            
            self.consume(TokenType::Colon, "Expected ':' after object property name")?;
            
            // Parse value
            let value = self.parse_value()?;
            
            object.insert(key, value);
            
            // Optional comma
            self.match_token(&TokenType::Comma);
            
            // Skip newlines after comma
            while self.match_token(&TokenType::Newline) {}
        }
        
        self.consume(TokenType::RightBrace, "Expected '}'")?;
        Ok(PropertyValue::Object(object))
    }
    
    /// Parse array literal: [value, value, value]
    fn parse_array_literal(&mut self) -> Result<PropertyValue> {
        self.consume(TokenType::LeftBracket, "Expected '['")?;
        let mut array = Vec::new();
        
        // Skip newlines after opening bracket
        while self.match_token(&TokenType::Newline) {}
        
        // Parse values
        while !self.check(&TokenType::RightBracket) && !self.is_at_end() {
            // Skip newlines
            if self.match_token(&TokenType::Newline) {
                continue;
            }
            
            let value = self.parse_value()?;
            array.push(value);
            
            // Optional comma
            self.match_token(&TokenType::Comma);
            
            // Skip newlines after comma
            while self.match_token(&TokenType::Newline) {}
        }
        
        self.consume(TokenType::RightBracket, "Expected ']'")?;
        Ok(PropertyValue::Array(array))
    }
    
    // Utility methods

    fn is_property(&self) -> bool {
        // Check if current token is an identifier or keyword followed by a colon
        let is_property_name = match &self.peek().token_type {
            TokenType::Identifier(_) => true,
            // Allow keywords to be used as property names
            TokenType::Style => true,
            TokenType::Text => true,
            TokenType::Image => true,
            TokenType::Button => true,
            TokenType::Input => true,
            TokenType::Container => true,
            TokenType::App => true,
            _ => false,
        };
        
        if is_property_name {
            // Look ahead to see if the next token is a colon
            if self.current + 1 < self.tokens.len() {
                matches!(self.tokens[self.current + 1].token_type, TokenType::Colon)
            } else {
                false
            }
        } else {
            false
        }
    }

    fn is_element_start(&self) -> bool {
        match &self.peek().token_type {
            // Known element types
            TokenType::App | TokenType::Container | TokenType::Text |
            TokenType::Image | TokenType::Button | TokenType::Input => true,
            
            // For identifiers, check if they're followed by an opening brace (element)
            // rather than a colon (property)
            TokenType::Identifier(_) => {
                if self.current + 1 < self.tokens.len() {
                    matches!(self.tokens[self.current + 1].token_type, TokenType::LeftBrace)
                } else {
                    false
                }
            }
            
            _ => false
        }
    }
    
    fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }
    
    fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.peek().token_type) == std::mem::discriminant(token_type)
        }
    }
    
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }
    
    fn is_at_end(&self) -> bool {
        matches!(self.peek().token_type, TokenType::Eof)
    }
    
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }
    
    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }
    
    fn consume(&mut self, token_type: TokenType, message: &str) -> Result<&Token> {
        if self.check(&token_type) {
            Ok(self.advance())
        } else {
            Err(CompilerError::parse_legacy(
                self.peek().line,
                format!("{}, got {}", message, self.peek().token_type)
            ))
        }
    }
    
    /// Create a default App wrapper when no App element is present in the KRY file
    /// This matches the behavior of the renderer which creates an App wrapper automatically
    fn create_default_app_wrapper(&mut self, standalone_elements: Vec<AstNode>) -> Result<Option<Box<AstNode>>> {
        if standalone_elements.is_empty() {
            return Ok(None);
        }
        
        let element_count = standalone_elements.len();
        let default_app = AstNode::Element {
            element_type: "App".to_string(),
            properties: vec![
                AstProperty::new("window_title".to_string(), PropertyValue::String("\"Auto-generated App\"".to_string()), 1),
                AstProperty::new("window_width".to_string(), PropertyValue::Integer(800), 1),
                AstProperty::new("window_height".to_string(), PropertyValue::Integer(600), 1),
            ],
            pseudo_selectors: Vec::new(),
            children: standalone_elements, // Wrap all standalone elements as children
        };
        
        log::info!("Auto-created App wrapper for {} standalone elements", element_count);
        Ok(Some(Box::new(default_app)))
    }
    
    /// Extract template variables from a string value
    /// Finds patterns like $variable_name and returns a list of variable names
    fn extract_template_variables(&self, value: &str) -> Vec<String> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    
    #[test]
    fn test_parse_simple_app() {
        let source = r#"
            App {
                window_title: "Test App"
                Text {
                    text: "Hello World"
                }
            }
        "#;
        
        let mut lexer = Lexer::new(source, "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        
        match ast {
            AstNode::File { app: Some(app), .. } => {
                match app.as_ref() {
                    AstNode::Element { element_type, .. } => {
                        assert_eq!(element_type, "App");
                    }
                    _ => panic!("Expected App element"),
                }
            }
            _ => panic!("Expected File with App"),
        }
    }
    
    #[test]
    fn test_auto_app_wrapper_creation() {
        // Test that standalone elements are automatically wrapped in an App
        let source = r#"
            Text {
                text: "Hello World"
                font_size: 18
            }
            
            Container {
                layout: "column"
                
                Button {
                    text: "Click Me"
                }
            }
        "#;
        
        let mut lexer = Lexer::new(source, "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        
        // Verify that an App element was auto-created
        match ast {
            AstNode::File { app: Some(app), .. } => {
                match app.as_ref() {
                    AstNode::Element { element_type, properties, children, .. } => {
                        assert_eq!(element_type, "App");
                        
                        // Check for default App properties
                        assert!(properties.iter().any(|p| p.key == "window_title"));
                        assert!(properties.iter().any(|p| p.key == "window_width"));
                        assert!(properties.iter().any(|p| p.key == "window_height"));
                        
                        // Check that standalone elements were wrapped as children
                        assert_eq!(children.len(), 2); // Text and Container
                        
                        // Verify first child is Text
                        if let AstNode::Element { element_type, .. } = &children[0] {
                            assert_eq!(element_type, "Text");
                        } else {
                            panic!("Expected first child to be Text element");
                        }
                        
                        // Verify second child is Container with its own children
                        if let AstNode::Element { element_type, children: container_children, .. } = &children[1] {
                            assert_eq!(element_type, "Container");
                            assert_eq!(container_children.len(), 1); // Button
                            
                            if let AstNode::Element { element_type, .. } = &container_children[0] {
                                assert_eq!(element_type, "Button");
                            } else {
                                panic!("Expected Container child to be Button element");
                            }
                        } else {
                            panic!("Expected second child to be Container element");
                        }
                    }
                    _ => panic!("Expected auto-generated App element"),
                }
            }
            _ => panic!("Expected File with auto-generated App"),
        }
    }

    #[test]
    fn test_no_auto_app_wrapper_when_app_exists() {
        // Test that no auto-wrapper is created when App already exists
        let source = r#"
            App {
                window_title: "Existing App"
                
                Text {
                    text: "Hello World"
                }
            }
        "#;
        
        let mut lexer = Lexer::new(source, "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        
        // Verify the existing App is preserved, not wrapped
        match ast {
            AstNode::File { app: Some(app), .. } => {
                match app.as_ref() {
                    AstNode::Element { element_type, properties, children, .. } => {
                        assert_eq!(element_type, "App");
                        
                        // Should have the original window_title, not auto-generated
                        let title_prop = properties.iter().find(|p| p.key == "window_title").unwrap();
                        assert_eq!(title_prop.value.to_string(), "\"Existing App\"");
                        
                        // Should have one child (the Text element)
                        assert_eq!(children.len(), 1);
                    }
                    _ => panic!("Expected original App element"),
                }
            }
            _ => panic!("Expected File with original App"),
        }
    }

    #[test]
    fn test_parse_component() {
        let source = r#"
            Define Button {
                Properties {
                    text: String = "Click me"
                    enabled: Bool = true
                }
                Container {
                    Text { text: $text }
                }
            }
        "#;
        
        let mut lexer = Lexer::new(source, "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        
        match ast {
            AstNode::File { components, .. } => {
                assert_eq!(components.len(), 1);
                match &components[0] {
                    AstNode::Component { name, properties, .. } => {
                        assert_eq!(name, "Button");
                        assert_eq!(properties.len(), 2);
                    }
                    _ => panic!("Expected Component"),
                }
            }
            _ => panic!("Expected File with components"),
        }
    }

    #[test]
    fn test_template_variable_extraction() {
        let parser = Parser::new(Vec::new());
        
        // Test simple template variable
        let vars = parser.extract_template_variables("Count: $counter_value");
        assert_eq!(vars, vec!["counter_value"]);
        
        // Test multiple template variables
        let vars = parser.extract_template_variables("Hello $user_name, count: $counter_value");
        assert_eq!(vars, vec!["user_name", "counter_value"]);
        
        // Test no template variables
        let vars = parser.extract_template_variables("Plain text");
        assert!(vars.is_empty());
        
        // Test duplicate variables (should only appear once)
        let vars = parser.extract_template_variables("$var and $var again");
        assert_eq!(vars, vec!["var"]);
    }

    #[test]
    fn test_parse_input_with_type() {
        let source = r#"
            App {
                Input {
                    type: "text"
                    placeholder: "Enter text"
                    value: "default"
                }
                Input {
                    type: "checkbox"
                    checked: true
                    text: "Accept terms"
                }
                Input {
                    type: "range"
                    min: 0
                    max: 100
                    step: 1
                    value: 50
                }
            }
        "#;
        
        let mut lexer = Lexer::new(source, "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        
        match ast {
            AstNode::File { app: Some(app), .. } => {
                match app.as_ref() {
                    AstNode::Element { element_type, children, .. } => {
                        assert_eq!(element_type, "App");
                        assert_eq!(children.len(), 3);
                        
                        // Check first Input (text type)
                        if let AstNode::Element { element_type, properties, .. } = &children[0] {
                            assert_eq!(element_type, "Input");
                            assert!(properties.iter().any(|p| p.key == "type" && p.value.to_string() == "\"text\""));
                            assert!(properties.iter().any(|p| p.key == "placeholder"));
                            assert!(properties.iter().any(|p| p.key == "value"));
                        } else {
                            panic!("Expected first child to be Input element");
                        }
                        
                        // Check second Input (checkbox type)
                        if let AstNode::Element { element_type, properties, .. } = &children[1] {
                            assert_eq!(element_type, "Input");
                            assert!(properties.iter().any(|p| p.key == "type" && p.value.to_string() == "\"checkbox\""));
                            assert!(properties.iter().any(|p| p.key == "checked"));
                            assert!(properties.iter().any(|p| p.key == "text"));
                        } else {
                            panic!("Expected second child to be Input element");
                        }
                        
                        // Check third Input (range type)
                        if let AstNode::Element { element_type, properties, .. } = &children[2] {
                            assert_eq!(element_type, "Input");
                            assert!(properties.iter().any(|p| p.key == "type" && p.value.to_string() == "\"range\""));
                            assert!(properties.iter().any(|p| p.key == "min"));
                            assert!(properties.iter().any(|p| p.key == "max"));
                            assert!(properties.iter().any(|p| p.key == "step"));
                            assert!(properties.iter().any(|p| p.key == "value"));
                        } else {
                            panic!("Expected third child to be Input element");
                        }
                    }
                    _ => panic!("Expected App element"),
                }
            }
            _ => panic!("Expected File with App"),
        }
    }
}