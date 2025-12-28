//! Schema parsing and representation.

use std::fs;
use std::path::Path;

use kdl::{KdlDocument, KdlNode};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("KDL parse error: {0}")]
    Parse(#[from] kdl::KdlError),

    #[error("Schema error: {0}")]
    Schema(String),
}

/// A protocol schema definition.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Schema {
    /// Schema metadata.
    pub info: SchemaInfo,
    /// Message definitions.
    pub messages: Vec<MessageDef>,
}

/// Schema metadata.
#[derive(Debug, Clone, Default)]
pub struct SchemaInfo {
    pub title: Option<String>,
    pub version: Option<String>,
}

/// A message definition.
#[derive(Debug, Clone)]
pub struct MessageDef {
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<FieldDef>,
}

/// A field definition.
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub ty: FieldType,
    pub required: bool,
}

/// Field types.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    String,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
}

impl Schema {
    /// Load a schema from a file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, SchemaError> {
        let content = fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse a schema from a string.
    pub fn parse(input: &str) -> Result<Self, SchemaError> {
        let doc: KdlDocument = input.parse()?;
        Self::from_document(&doc)
    }

    fn from_document(doc: &KdlDocument) -> Result<Self, SchemaError> {
        let mut info = SchemaInfo::default();
        let mut messages = Vec::new();

        for node in doc.nodes() {
            match node.name().value() {
                "info" => {
                    info = Self::parse_info(node)?;
                }
                "message" => {
                    messages.push(Self::parse_message(node)?);
                }
                name => {
                    return Err(SchemaError::Schema(format!(
                        "Unknown top-level node: {}",
                        name
                    )));
                }
            }
        }

        Ok(Schema { info, messages })
    }

    fn parse_info(node: &KdlNode) -> Result<SchemaInfo, SchemaError> {
        let children = node.children().map(|c| c.nodes()).unwrap_or_default();
        let mut info = SchemaInfo::default();

        for child in children {
            match child.name().value() {
                "title" => {
                    info.title = child
                        .entries()
                        .first()
                        .and_then(|e| e.value().as_string())
                        .map(|s| s.to_string());
                }
                "version" => {
                    info.version = child
                        .entries()
                        .first()
                        .and_then(|e| e.value().as_string())
                        .map(|s| s.to_string());
                }
                _ => {}
            }
        }

        Ok(info)
    }

    fn parse_message(node: &KdlNode) -> Result<MessageDef, SchemaError> {
        let name = node
            .entries()
            .first()
            .and_then(|e| e.value().as_string())
            .ok_or_else(|| SchemaError::Schema("Message must have a name".into()))?
            .to_string();

        let mut fields = Vec::new();

        if let Some(children) = node.children() {
            for child in children.nodes() {
                if child.name().value() == "field" {
                    fields.push(Self::parse_field(child)?);
                }
            }
        }

        Ok(MessageDef {
            name,
            description: None,
            fields,
        })
    }

    fn parse_field(node: &KdlNode) -> Result<FieldDef, SchemaError> {
        let name = node
            .entries()
            .first()
            .and_then(|e| e.value().as_string())
            .ok_or_else(|| SchemaError::Schema("Field must have a name".into()))?
            .to_string();

        let ty = node
            .entries()
            .iter()
            .find(|e| e.name().map(|n| n.value()) == Some("type"))
            .and_then(|e| e.value().as_string())
            .map(Self::parse_type)
            .unwrap_or(FieldType::String);

        let required = node
            .entries()
            .iter()
            .find(|e| e.name().map(|n| n.value()) == Some("required"))
            .and_then(|e| e.value().as_bool())
            .unwrap_or(false);

        Ok(FieldDef { name, ty, required })
    }

    fn parse_type(s: &str) -> FieldType {
        match s {
            "string" => FieldType::String,
            "i8" => FieldType::I8,
            "i16" => FieldType::I16,
            "i32" => FieldType::I32,
            "i64" => FieldType::I64,
            "u8" => FieldType::U8,
            "u16" => FieldType::U16,
            "u32" => FieldType::U32,
            "u64" => FieldType::U64,
            "f32" => FieldType::F32,
            "f64" => FieldType::F64,
            "bool" => FieldType::Bool,
            _ => FieldType::String,
        }
    }
}
