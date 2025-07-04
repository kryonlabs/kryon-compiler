//! Recursive descent parser for the KRY language

use crate::ast::*;
use crate::error::{CompilerError, Result};
use crate::lexer::{Token, TokenType};
use std::collections::HashMap;

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
        let mut components = Vec::new();
        let mut scripts = Vec::new();
        let mut app = None;
        
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
                TokenType::Define => {
                    components.push(self.parse_component()?);
                }
                TokenType::App => {
                    if app.is_some() {
                        return Err(CompilerError::parse(
                            self.peek().line,
                            "Multiple App elements found. Only one App element is allowed."
                        ));
                    }
                    app = Some(Box::new(self.parse_element()?));
                }
                _ => {
                    // Try parsing as element (for component usage at root level)
                    if self.is_element_start() {
                        if app.is_some() {
                            return Err(CompilerError::parse(
                                self.peek().line,
                                "Only one root element (App or component) is allowed."
                            ));
                        }
                        app = Some(Box::new(self.parse_element()?));
                    } else {
                        return Err(CompilerError::parse(
                            self.peek().line,
                            format!("Unexpected token: {}", self.peek().token_type)
                        ));
                    }
                }
            }
        }
        
        Ok(AstNode::File {
            directives,
            styles,
            components,
            scripts,
            app,
        })
    }
    
    fn parse_include(&mut self) -> Result<AstNode> {
        self.consume(TokenType::Include, "Expected @include")?;
        
        let path = match &self.advance().token_type {
            TokenType::String(s) => s.clone(),
            _ => return Err(CompilerError::parse(
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
                _ => return Err(CompilerError::parse(
                    self.previous().line,
                    "Expected variable name"
                )),
            };
            
            self.consume(TokenType::Colon, "Expected ':' after variable name")?;
            
            let value = self.parse_value()?;
            variables.insert(name, value);
        }
        
        self.consume(TokenType::RightBrace, "Expected '}' after variables")?;
        
        Ok(AstNode::Variables { variables })
    }
    
    fn parse_script(&mut self) -> Result<AstNode> {
        self.consume(TokenType::Script, "Expected @script")?;
        
        let language = match &self.advance().token_type {
            TokenType::String(lang) => lang.clone(),
            _ => return Err(CompilerError::parse(
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
                        name = Some(self.parse_value()?);
                    }
                    "mode" => {
                        self.advance();
                        self.consume(TokenType::Equals, "Expected '=' after mode")?;
                        mode = Some(self.parse_value()?);
                    }
                    "from" => {
                        self.advance();
                        let file_path = self.parse_value()?;
                        source = Some(ScriptSource::External(file_path));
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
        let mut code = String::new();
        let mut brace_count = 1;
        
        while !self.is_at_end() && brace_count > 0 {
            match &self.advance().token_type {
                TokenType::LeftBrace => {
                    brace_count += 1;
                    code.push('{');
                }
                TokenType::RightBrace => {
                    brace_count -= 1;
                    if brace_count > 0 {
                        code.push('}');
                    }
                }
                TokenType::String(s) => {
                    code.push_str(&format!("\"{}\"", s));
                }
                TokenType::Identifier(id) => {
                    code.push_str(id);
                }
                TokenType::Number(n) => {
                    code.push_str(&n.to_string());
                }
                TokenType::Integer(i) => {
                    code.push_str(&i.to_string());
                }
                TokenType::Newline => {
                    code.push('\n');
                }
                token => {
                    code.push_str(&format!("{}", token));
                }
            }
            code.push(' ');
        }
        
        Ok(code.trim().to_string())
    }
    
    fn parse_function(&mut self) -> Result<AstNode> {
        self.consume(TokenType::Function, "Expected @function")?;
        
        let language = match &self.advance().token_type {
            TokenType::String(lang) => lang.clone(),
            _ => return Err(CompilerError::parse(
                self.previous().line,
                "Expected language string after @function"
            )),
        };
        
        // Parse function name and parameters
        let function_name = match &self.advance().token_type {
            TokenType::Identifier(name) => name.clone(),
            _ => return Err(CompilerError::parse(
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
                return Err(CompilerError::parse(
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
            return Err(CompilerError::parse(
                self.peek().line,
                format!("Expected style name string, but found {}", self.peek().token_type)
            ));
        };
        self.advance();

        self.consume(TokenType::LeftBrace, "Expected '{' after style name")?;
        
        let mut extends = Vec::new();
        let mut properties = Vec::new();
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            if self.match_token(&TokenType::Newline) {
                continue;
            }
            if matches!(self.peek().token_type, TokenType::Comment(_)) {
                self.advance();
                continue;
            }
            
            let prop = self.parse_property()?;
            
            // Handle extends specially
            if prop.key == "extends" {
                extends = self.parse_extends_value(&prop.value)?;
            } else {
                properties.push(prop);
            }
        }
        
        self.consume(TokenType::RightBrace, "Expected '}' after style properties")?;
        
        Ok(AstNode::Style {
            name,
            extends,
            properties,
        })
    }
    
    fn parse_extends_value(&self, value: &str) -> Result<Vec<String>> {
        let trimmed = value.trim();
        
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
    
    fn parse_component(&mut self) -> Result<AstNode> {
        self.consume(TokenType::Define, "Expected 'Define'")?;
        
        let name = if let TokenType::Identifier(name) = &self.peek().token_type {
            name.clone()
        } else {
            return Err(CompilerError::parse(
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
                    return Err(CompilerError::parse(
                        self.peek().line,
                        "Component can only have one root template element"
                    ));
                }
                template = Some(Box::new(self.parse_element()?));
            } else {
                return Err(CompilerError::parse(
                    self.peek().line,
                    "Expected 'Properties' block or template element in component"
                ));
            }
        }
        
        self.consume(TokenType::RightBrace, "Expected '}' after component definition")?;
        
        let template = template.ok_or_else(|| CompilerError::parse(
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
                _ => return Err(CompilerError::parse(
                    self.previous().line,
                    "Expected property name"
                )),
            };
            
            self.consume(TokenType::Colon, "Expected ':' after property name")?;
            
            let property_type = match &self.advance().token_type {
                TokenType::Identifier(type_name) => type_name.clone(),
                _ => return Err(CompilerError::parse(
                    self.previous().line,
                    "Expected property type"
                )),
            };
            
            let mut default_value = None;
            if self.match_token(&TokenType::Equals) {
                default_value = Some(self.parse_value()?);
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
            _ => return Err(CompilerError::parse(
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

                if self.match_token(&TokenType::PseudoSelector) {
                    pseudo_selectors.push(self.parse_pseudo_selector()?);
                } else if self.is_property() {
                    properties.push(self.parse_property()?);
                } else if self.is_element_start() {
                    children.push(self.parse_element()?);
                } else {
                    return Err(CompilerError::parse(
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
        // Parse &:state syntax
        let state = "hover".to_string(); // Simplified - would need to parse actual state
        
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
                return Err(CompilerError::parse(
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
        
        Ok(AstProperty::new(key, value, self.previous().line))
    }
    
    fn parse_value(&mut self) -> Result<String> {
        let result = match &self.peek().token_type {
            TokenType::String(s) => {
                let value = format!("\"{}\"", s);
                self.advance();
                Ok(value)
            }
            TokenType::Number(n) => {
                let value = n.to_string();
                self.advance();
                Ok(value)
            }
            TokenType::Integer(i) => {
                let value = i.to_string();
                self.advance();
                Ok(value)
            }
            TokenType::Boolean(b) => {
                let value = b.to_string();
                self.advance();
                Ok(value)
            }
            TokenType::Color(c) => {
                let value = c.clone();
                self.advance();
                Ok(value)
            }
            TokenType::Identifier(id) => {
                // Handle multiple identifiers separated by spaces (e.g., "column center")
                let mut value_parts = vec![id.clone()];
                self.advance();
                
                // Keep collecting identifiers until we hit a newline, semicolon, or closing brace
                while !self.is_at_end() {
                    match &self.peek().token_type {
                        TokenType::Identifier(next_id) => {
                            value_parts.push(next_id.clone());
                            self.advance();
                        }
                        TokenType::Newline | TokenType::Semicolon | TokenType::RightBrace => {
                            break;
                        }
                        _ => break,
                    }
                }
                
                Ok(value_parts.join(" "))
            }
            TokenType::Dollar => {
                self.advance(); // Consume the '$'
                if let TokenType::Identifier(name) = &self.peek().token_type {
                    let value = format!("${}", name);
                    self.advance();
                    Ok(value)
                } else {
                    Err(CompilerError::parse(
                        self.peek().line,
                        "Expected variable name after '$'"
                    ))
                }
            }
            _ => Err(CompilerError::parse(
                self.peek().line,
                format!("Expected a value, but found {}", self.peek().token_type)
            )),
        };

        result
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
            Err(CompilerError::parse(
                self.peek().line,
                format!("{}, got {}", message, self.peek().token_type)
            ))
        }
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
}