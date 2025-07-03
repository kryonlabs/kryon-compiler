//! Lexical analysis for KRY source code

use crate::error::{CompilerError, Result};
use regex::Regex;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Keywords
    App,
    Container,
    Text,
    Image,
    Canvas,
    Button,
    Input,
    Checkbox,
    Radio,
    Slider,
    List,
    Grid,
    Scrollable,
    Tabs,
    Video,
    Style,
    Define,
    Properties,
    
    // Directives
    Include,
    Variables,
    Script,
    
    // Operators and punctuation
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]
    LeftParen,    // (
    RightParen,   // )
    Colon,        // :
    Semicolon,    // ;
    Comma,        // ,
    Equals,       // =
    Ampersand,    // &
    Dollar,       // $
    
    // Pseudo-selector
    PseudoSelector, // &:hover, &:active, etc.
    
    // Literals
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Color(String), // #RGB, #RRGGBB, #RRGGBBAA
    Identifier(String),
    
    // Special
    Newline,
    Comment(String),
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub line: usize,
    pub column: usize,
    pub filename: String,
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::App => write!(f, "App"),
            TokenType::Container => write!(f, "Container"),
            TokenType::Text => write!(f, "Text"),
            TokenType::Image => write!(f, "Image"),
            TokenType::Canvas => write!(f, "Canvas"),
            TokenType::Button => write!(f, "Button"),
            TokenType::Input => write!(f, "Input"),
            TokenType::Checkbox => write!(f, "Checkbox"),
            TokenType::Radio => write!(f, "Radio"),
            TokenType::Slider => write!(f, "Slider"),
            TokenType::List => write!(f, "List"),
            TokenType::Grid => write!(f, "Grid"),
            TokenType::Scrollable => write!(f, "Scrollable"),
            TokenType::Tabs => write!(f, "Tabs"),
            TokenType::Video => write!(f, "Video"),
            TokenType::Style => write!(f, "style"),
            TokenType::Define => write!(f, "Define"),
            TokenType::Properties => write!(f, "Properties"),
            TokenType::Include => write!(f, "@include"),
            TokenType::Variables => write!(f, "@variables"),
            TokenType::Script => write!(f, "@script"),
            TokenType::LeftBrace => write!(f, "{{"),
            TokenType::RightBrace => write!(f, "}}"),
            TokenType::LeftBracket => write!(f, "["),
            TokenType::RightBracket => write!(f, "]"),
            TokenType::LeftParen => write!(f, "("),
            TokenType::RightParen => write!(f, ")"),
            TokenType::Colon => write!(f, ":"),
            TokenType::Semicolon => write!(f, ";"),
            TokenType::Comma => write!(f, ","),
            TokenType::Equals => write!(f, "="),
            TokenType::Ampersand => write!(f, "&"),
            TokenType::Dollar => write!(f, "$"),
            TokenType::PseudoSelector => write!(f, "pseudo-selector"),
            TokenType::String(s) => write!(f, "string(\"{}\")", s),
            TokenType::Number(n) => write!(f, "number({})", n),
            TokenType::Integer(i) => write!(f, "integer({})", i),
            TokenType::Boolean(b) => write!(f, "boolean({})", b),
            TokenType::Color(c) => write!(f, "color({})", c),
            TokenType::Identifier(id) => write!(f, "identifier({})", id),
            TokenType::Newline => write!(f, "newline"),
            TokenType::Comment(c) => write!(f, "comment({})", c),
            TokenType::Eof => write!(f, "EOF"),
        }
    }
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    filename: String,
    
    // Regex patterns for complex tokens
    color_regex: Regex,
    identifier_regex: Regex,
    pseudo_selector_regex: Regex,
}

impl Lexer {
    pub fn new(input: &str, filename: String) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            filename,
            color_regex: Regex::new(r"^#[0-9A-Fa-f]{3,8}$").unwrap(),
            identifier_regex: Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap(),
            pseudo_selector_regex: Regex::new(r"^&:(hover|active|focus|disabled|checked)$").unwrap(),
        }
    }
    
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        
        while !self.is_at_end() {
            if let Some(token) = self.next_token()? {
                tokens.push(token);
            }
        }
        
        tokens.push(Token {
            token_type: TokenType::Eof,
            line: self.line,
            column: self.column,
            filename: self.filename.clone(),
        });
        
        Ok(tokens)
    }
    
    fn next_token(&mut self) -> Result<Option<Token>> {
        self.skip_whitespace_except_newlines();
        
        if self.is_at_end() {
            return Ok(None);
        }
        
        let start_line = self.line;
        let start_column = self.column;
        let ch = self.advance();
        
        let token_type = match ch {
            '\n' => {
                self.line += 1;
                self.column = 1;
                TokenType::Newline
            }
            '\r' => {
                if self.peek() == Some('\n') {
                    self.advance(); // consume \n
                }
                self.line += 1;
                self.column = 1;
                TokenType::Newline
            }
            '{' => TokenType::LeftBrace,
            '}' => TokenType::RightBrace,
            '[' => TokenType::LeftBracket,
            ']' => TokenType::RightBracket,
            '(' => TokenType::LeftParen,
            ')' => TokenType::RightParen,
            ':' => TokenType::Colon,
            ';' => TokenType::Semicolon,
            ',' => TokenType::Comma,
            '=' => TokenType::Equals,
            '$' => TokenType::Dollar,
            '&' => {
                if self.peek() == Some(':') {
                    // Handle pseudo-selectors like &:hover
                    let pseudo = self.read_pseudo_selector()?;
                    if self.pseudo_selector_regex.is_match(&pseudo) {
                        TokenType::PseudoSelector
                    } else {
                        return Err(CompilerError::parse(
                            self.line,
                            format!("Invalid pseudo-selector: {}", pseudo)
                        ));
                    }
                } else {
                    TokenType::Ampersand
                }
            }
            '#' => {
                if self.is_hex_digit(self.peek()) {
                    // Color literal
                    let color = self.read_color()?;
                    TokenType::Color(color)
                } else {
                    // Comment
                    let comment = self.read_comment();
                    TokenType::Comment(comment)
                }
            }
            '/' => {
                if self.peek() == Some('/') {
                    // Double-slash comment
                    self.advance(); // consume second '/'
                    let comment = self.read_comment();
                    TokenType::Comment(comment)
                } else {
                    return Err(CompilerError::parse(
                        self.line,
                        format!("Unexpected character: '{}' (single slash not supported)", ch)
                    ));
                }
            }
            '"' => {
                let string_value = self.read_string()?;
                TokenType::String(string_value)
            }
            '@' => {
                let directive = self.read_directive()?;
                match directive.as_str() {
                    "@include" => TokenType::Include,
                    "@variables" => TokenType::Variables,
                    "@script" => TokenType::Script,
                    _ => return Err(CompilerError::parse(
                        self.line,
                        format!("Unknown directive: {}", directive)
                    )),
                }
            }
            ch if ch.is_ascii_digit() || (ch == '-' && self.peek().map_or(false, |c| c.is_ascii_digit())) => {
                let number_str = self.read_number(ch)?;
                if number_str.contains('.') {
                    TokenType::Number(number_str.parse().map_err(|_| {
                        CompilerError::parse(self.line, format!("Invalid number: {}", number_str))
                    })?)
                } else {
                    TokenType::Integer(number_str.parse().map_err(|_| {
                        CompilerError::parse(self.line, format!("Invalid integer: {}", number_str))
                    })?)
                }
            }
            ch if ch.is_alphabetic() || ch == '_' => {
                let identifier = self.read_identifier(ch)?;
                self.identify_keyword_or_identifier(identifier)
            }
            _ => {
                return Err(CompilerError::parse(
                    self.line,
                    format!("Unexpected character: '{}'", ch)
                ));
            }
        };
        
        Ok(Some(Token {
            token_type,
            line: start_line,
            column: start_column,
            filename: self.filename.clone(),
        }))
    }
    
    fn skip_whitespace_except_newlines(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() && ch != '\n' && ch != '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }
    
    fn advance(&mut self) -> char {
        if self.position < self.input.len() {
            let ch = self.input[self.position];
            self.position += 1;
            if ch != '\n' && ch != '\r' {
                self.column += 1;
            }
            ch
        } else {
            '\0'
        }
    }
    
    fn peek(&self) -> Option<char> {
        if self.position < self.input.len() {
            Some(self.input[self.position])
        } else {
            None
        }
    }
    
    fn peek_next(&self) -> Option<char> {
        if self.position + 1 < self.input.len() {
            Some(self.input[self.position + 1])
        } else {
            None
        }
    }
    
    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }
    
    fn is_hex_digit(&self, ch: Option<char>) -> bool {
        ch.map_or(false, |c| c.is_ascii_hexdigit())
    }
    
    fn read_string(&mut self) -> Result<String> {
        let mut value = String::new();
        let mut escaped = false;
        
        while let Some(ch) = self.peek() {
            if escaped {
                match ch {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    '\'' => value.push('\''),
                    _ => {
                        value.push('\\');
                        value.push(ch);
                    }
                }
                escaped = false;
                self.advance();
            } else if ch == '\\' {
                escaped = true;
                self.advance();
            } else if ch == '"' {
                self.advance(); // consume closing quote
                return Ok(value);
            } else if ch == '\n' || ch == '\r' {
                return Err(CompilerError::parse(
                    self.line,
                    "Unterminated string literal"
                ));
            } else {
                value.push(ch);
                self.advance();
            }
        }
        
        Err(CompilerError::parse(
            self.line,
            "Unterminated string literal"
        ))
    }
    
    fn read_color(&mut self) -> Result<String> {
        let mut color = String::from("#");
        
        while let Some(ch) = self.peek() {
            if ch.is_ascii_hexdigit() {
                color.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        // Validate color format
        if !self.color_regex.is_match(&color) {
            return Err(CompilerError::parse(
                self.line,
                format!("Invalid color format: {}", color)
            ));
        }
        
        // Validate length (3, 4, 6, or 8 hex digits after #)
        let hex_part = &color[1..];
        match hex_part.len() {
            3 | 4 | 6 | 8 => Ok(color),
            _ => Err(CompilerError::parse(
                self.line,
                format!("Invalid color format: {} (expected 3, 4, 6, or 8 hex digits)", color)
            )),
        }
    }
    
    fn read_comment(&mut self) -> String {
        let mut comment = String::new();
        
        while let Some(ch) = self.peek() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            comment.push(ch);
            self.advance();
        }
        
        comment
    }
    
    fn read_directive(&mut self) -> Result<String> {
        let mut directive = String::from("@");
        
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                directive.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(directive)
    }
    
    fn read_pseudo_selector(&mut self) -> Result<String> {
        let mut pseudo = String::from("&");
        
        // We already know the next char is ':'
        pseudo.push(self.advance());
        
        // Read the selector name
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                pseudo.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(pseudo)
    }
    
    fn read_number(&mut self, first_char: char) -> Result<String> {
        let mut number = String::new();
        number.push(first_char);
        
        let mut has_dot = false;
        
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                number.push(ch);
                self.advance();
            } else if ch == '.' && !has_dot {
                has_dot = true;
                number.push(ch);
                self.advance();
                
                // Must have digit after decimal point
                if !self.peek().map_or(false, |c| c.is_ascii_digit()) {
                    return Err(CompilerError::parse(
                        self.line,
                        format!("Invalid number format: {}", number)
                    ));
                }
            } else {
                break;
            }
        }
        
        Ok(number)
    }
    
    fn read_identifier(&mut self, first_char: char) -> Result<String> {
        let mut identifier = String::new();
        identifier.push(first_char);
        
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                identifier.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(identifier)
    }
    
    fn identify_keyword_or_identifier(&self, text: String) -> TokenType {
        match text.as_str() {
            // Element types
            "App" => TokenType::App,
            "Container" => TokenType::Container,
            "Text" => TokenType::Text,
            "Image" => TokenType::Image,
            "Canvas" => TokenType::Canvas,
            "Button" => TokenType::Button,
            "Input" => TokenType::Input,
            "Checkbox" => TokenType::Checkbox,
            "Radio" => TokenType::Radio,
            "Slider" => TokenType::Slider,
            "List" => TokenType::List,
            "Grid" => TokenType::Grid,
            "Scrollable" => TokenType::Scrollable,
            "Tabs" => TokenType::Tabs,
            "Video" => TokenType::Video,
            
            // Keywords
            "style" => TokenType::Style,
            "Define" => TokenType::Define,
            "Properties" => TokenType::Properties,
            
            // Boolean literals
            "true" => TokenType::Boolean(true),
            "false" => TokenType::Boolean(false),
            
            // Default to identifier
            _ => TokenType::Identifier(text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_tokens() {
        let mut lexer = Lexer::new("{ } : ; = $ &", "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::LeftBrace);
        assert_eq!(tokens[1].token_type, TokenType::RightBrace);
        assert_eq!(tokens[2].token_type, TokenType::Colon);
        assert_eq!(tokens[3].token_type, TokenType::Semicolon);
        assert_eq!(tokens[4].token_type, TokenType::Equals);
        assert_eq!(tokens[5].token_type, TokenType::Dollar);
        assert_eq!(tokens[6].token_type, TokenType::Ampersand);
    }
    
    #[test]
    fn test_string_literal() {
        let mut lexer = Lexer::new(r#""hello world""#, "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        
        match &tokens[0].token_type {
            TokenType::String(s) => assert_eq!(s, "hello world"),
            _ => panic!("Expected string token"),
        }
    }
    
    #[test]
    fn test_string_escaping() {
        let mut lexer = Lexer::new(r#""hello\nworld\t\"test\"""#, "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        
        match &tokens[0].token_type {
            TokenType::String(s) => assert_eq!(s, "hello\nworld\t\"test\""),
            _ => panic!("Expected string token"),
        }
    }
    
    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 3.14 -10", "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        
        match tokens[0].token_type {
            TokenType::Integer(i) => assert_eq!(i, 42),
            _ => panic!("Expected integer token"),
        }
        
        match tokens[1].token_type {
            TokenType::Number(n) => assert_eq!(n, 3.14),
            _ => panic!("Expected number token"),
        }
        
        match tokens[2].token_type {
            TokenType::Integer(i) => assert_eq!(i, -10),
            _ => panic!("Expected integer token"),
        }
    }
    
    #[test]
    fn test_colors() {
        let mut lexer = Lexer::new("#FF0000 #ABC #12345678", "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        
        match &tokens[0].token_type {
            TokenType::Color(c) => assert_eq!(c, "#FF0000"),
            _ => panic!("Expected color token"),
        }
        
        match &tokens[1].token_type {
            TokenType::Color(c) => assert_eq!(c, "#ABC"),
            _ => panic!("Expected color token"),
        }
        
        match &tokens[2].token_type {
            TokenType::Color(c) => assert_eq!(c, "#12345678"),
            _ => panic!("Expected color token"),
        }
    }
    
    #[test]
    fn test_pseudo_selector() {
        let mut lexer = Lexer::new("&:hover &:active &:focus", "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::PseudoSelector);
        assert_eq!(tokens[1].token_type, TokenType::PseudoSelector);
        assert_eq!(tokens[2].token_type, TokenType::PseudoSelector);
    }
    
    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("App Container style Define @include @variables", "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::App);
        assert_eq!(tokens[1].token_type, TokenType::Container);
        assert_eq!(tokens[2].token_type, TokenType::Style);
        assert_eq!(tokens[3].token_type, TokenType::Define);
        assert_eq!(tokens[4].token_type, TokenType::Include);
        assert_eq!(tokens[5].token_type, TokenType::Variables);
    }
    
    #[test]
    fn test_comments() {
        let mut lexer = Lexer::new("# This is a comment\nApp", "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        
        match &tokens[0].token_type {
            TokenType::Comment(c) => assert_eq!(c, " This is a comment"),
            _ => panic!("Expected comment token"),
        }
        assert_eq!(tokens[1].token_type, TokenType::Newline);
        assert_eq!(tokens[2].token_type, TokenType::App);
    }
    
    #[test]
    fn test_double_slash_comments() {
        let mut lexer = Lexer::new("// This is a double-slash comment\nContainer", "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        
        match &tokens[0].token_type {
            TokenType::Comment(c) => assert_eq!(c, " This is a double-slash comment"),
            _ => panic!("Expected comment token"),
        }
        assert_eq!(tokens[1].token_type, TokenType::Newline);
        assert_eq!(tokens[2].token_type, TokenType::Container);
    }
    
    #[test]
    fn test_both_comment_styles() {
        let input = "# Hash comment\n// Double-slash comment\nApp";
        let mut lexer = Lexer::new(input, "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        
        match &tokens[0].token_type {
            TokenType::Comment(c) => assert_eq!(c, " Hash comment"),
            _ => panic!("Expected hash comment token"),
        }
        assert_eq!(tokens[1].token_type, TokenType::Newline);
        
        match &tokens[2].token_type {
            TokenType::Comment(c) => assert_eq!(c, " Double-slash comment"),
            _ => panic!("Expected double-slash comment token"),
        }
        assert_eq!(tokens[3].token_type, TokenType::Newline);
        assert_eq!(tokens[4].token_type, TokenType::App);
    }
    
    #[test]
    fn test_complex_example() {
        let input = r##"
App {
    window_title: "My App"
    background_color: #FF0000
    
    Container {
        id: "main_container"
        padding: 16
        
        Button {
            text: "Click me"
            &:hover {
                background_color: #00FF00
            }
        }
    }
}
"##;
        
        let mut lexer = Lexer::new(input, "test.kry".to_string());
        let tokens = lexer.tokenize().unwrap();
        
        // Just verify we can tokenize without errors and get reasonable tokens
        assert!(tokens.len() > 10);
        assert_eq!(tokens.last().unwrap().token_type, TokenType::Eof);
    }
}