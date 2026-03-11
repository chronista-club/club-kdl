//! Error types for unison-kdl

use std::fmt;

/// Result type alias for unison-kdl operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for KDL deserialization/serialization
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// KDL parsing error
    #[error("parse error: {0}")]
    Parse(String),

    /// Missing required field
    #[error("missing required field: {0}")]
    MissingField(&'static str),

    /// Missing required argument
    #[error("missing required argument at index {0}")]
    MissingArgument(usize),

    /// Missing required child node
    #[error("missing required child node: {0}")]
    MissingChild(&'static str),

    /// Type mismatch during deserialization
    #[error("type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        expected: &'static str,
        found: String,
    },

    /// Invalid value
    #[error("invalid value for {field}: {message}")]
    InvalidValue {
        field: &'static str,
        message: String,
    },

    /// Unexpected node name
    #[error("unexpected node name: expected {expected}, found {found}")]
    UnexpectedNode {
        expected: &'static str,
        found: String,
    },

    /// Duplicate node
    #[error("duplicate node: {0}")]
    DuplicateNode(String),

    /// Custom error message
    #[error("{0}")]
    Custom(String),

    /// Error with context (struct::field path)
    #[error("in {context}: {source}")]
    InContext {
        context: String,
        #[source]
        source: Box<Error>,
    },
}

impl Error {
    /// Create a custom error
    #[inline]
    pub fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }

    /// Create a type mismatch error
    #[inline]
    pub fn type_mismatch(expected: &'static str, found: impl fmt::Display) -> Self {
        Error::TypeMismatch {
            expected,
            found: found.to_string(),
        }
    }

    /// Wrap this error with context information
    #[inline]
    pub fn in_context(self, context: impl Into<String>) -> Self {
        Error::InContext {
            context: context.into(),
            source: Box::new(self),
        }
    }
}
