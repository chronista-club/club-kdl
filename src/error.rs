use std::fmt;

use serde::{de, ser};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("KDL parse error: {0}")]
    Parse(#[from] kdl::KdlError),

    #[error("{0}")]
    Message(String),

    #[error("Expected node, got document with {0} nodes")]
    ExpectedSingleNode(usize),

    #[error("Missing field: {0}")]
    MissingField(String),

    #[error("Unknown field: {0}")]
    UnknownField(String),

    #[error("Type mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: String, got: String },

    #[error("Serialization not yet supported for this type")]
    UnsupportedType,
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}
