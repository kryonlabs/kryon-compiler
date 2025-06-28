//! Error types for the Kryon compiler

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error at line {line}: {message}")]
    Parse { line: usize, message: String },

    #[error("Semantic error at line {line}: {message}")]
    Semantic { line: usize, message: String },

    #[error("Include error: {message}")]
    Include { message: String },

    #[error("Variable error at line {line}: {message}")]
    Variable { line: usize, message: String },

    #[error("Style error: {message}")]
    Style { message: String },

    #[error("Component error at line {line}: {message}")]
    Component { line: usize, message: String },

    #[error("Script error at line {line}: {message}")]
    Script { line: usize, message: String },

    #[error("Code generation error: {message}")]
    CodeGen { message: String },

    #[error("Resource error: {message}")]
    Resource { message: String },

    #[error("Maximum limit exceeded: {limit_type} (limit: {limit})")]
    LimitExceeded { limit_type: String, limit: usize },

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Invalid format: {message}")]
    InvalidFormat { message: String },
}

pub type Result<T> = std::result::Result<T, CompilerError>;

impl CompilerError {
    pub fn parse(line: usize, message: impl Into<String>) -> Self {
        Self::Parse {
            line,
            message: message.into(),
        }
    }

    pub fn semantic(line: usize, message: impl Into<String>) -> Self {
        Self::Semantic {
            line,
            message: message.into(),
        }
    }

    pub fn component(line: usize, message: impl Into<String>) -> Self {
        Self::Component {
            line,
            message: message.into(),
        }
    }

    pub fn script(line: usize, message: impl Into<String>) -> Self {
        Self::Script {
            line,
            message: message.into(),
        }
    }

    pub fn variable(line: usize, message: impl Into<String>) -> Self {
        Self::Variable {
            line,
            message: message.into(),
        }
    }
}