//! Schema parsing and validation.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use kdl::{KdlDocument, KdlNode};

use crate::Error;

/// A protocol schema definition.
#[derive(Debug, Clone)]
pub struct Schema {
    /// Schema metadata.
    pub info: SchemaInfo,
    /// Message definitions.
    pub messages: HashMap<String, MessageDef>,
}

/// Schema metadata.
#[derive(Debug, Clone, Default)]
pub struct SchemaInfo {
    pub title: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

/// A message definition.
#[derive(Debug, Clone)]
pub struct MessageDef {
    /// Message name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Properties.
    pub props: Vec<PropDef>,
    /// Values (positional arguments).
    pub values: Vec<ValueDef>,
}

/// A property definition.
#[derive(Debug, Clone)]
pub struct PropDef {
    pub name: String,
    pub ty: ValueType,
    pub required: bool,
    pub description: Option<String>,
}

/// A value definition.
#[derive(Debug, Clone)]
pub struct ValueDef {
    pub ty: ValueType,
    pub description: Option<String>,
}

/// Value types.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
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
    Optional(Box<ValueType>),
}

impl Schema {
    /// Load a schema from a file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let content = fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse a schema from a string.
    pub fn parse(input: &str) -> Result<Self, Error> {
        let doc: KdlDocument = input.parse()?;
        Self::from_document(&doc)
    }

    /// Build schema from KDL document.
    fn from_document(doc: &KdlDocument) -> Result<Self, Error> {
        let mut info = SchemaInfo::default();
        let mut messages = HashMap::new();

        for node in doc.nodes() {
            match node.name().value() {
                "info" => {
                    info = Self::parse_info(node)?;
                }
                "message" => {
                    let msg = Self::parse_message(node)?;
                    messages.insert(msg.name.clone(), msg);
                }
                name => {
                    return Err(Error::Schema(format!("Unknown top-level node: {}", name)));
                }
            }
        }

        Ok(Schema { info, messages })
    }

    fn parse_info(node: &KdlNode) -> Result<SchemaInfo, Error> {
        let children = node.children().map(|c| c.nodes()).unwrap_or_default();
        let mut info = SchemaInfo::default();

        for child in children {
            match child.name().value() {
                "title" => {
                    info.title = child.entries().first()
                        .and_then(|e| e.value().as_string())
                        .map(|s| s.to_string());
                }
                "version" => {
                    info.version = child.entries().first()
                        .and_then(|e| e.value().as_string())
                        .map(|s| s.to_string());
                }
                "description" => {
                    info.description = child.entries().first()
                        .and_then(|e| e.value().as_string())
                        .map(|s| s.to_string());
                }
                _ => {}
            }
        }

        Ok(info)
    }

    fn parse_message(node: &KdlNode) -> Result<MessageDef, Error> {
        let name = node.entries().first()
            .and_then(|e| e.value().as_string())
            .ok_or_else(|| Error::Schema("Message must have a name".into()))?
            .to_string();

        let mut props = Vec::new();
        let mut values = Vec::new();
        let mut description = None;

        if let Some(children) = node.children() {
            for child in children.nodes() {
                match child.name().value() {
                    "description" => {
                        description = child.entries().first()
                            .and_then(|e| e.value().as_string())
                            .map(|s| s.to_string());
                    }
                    "prop" => {
                        props.push(Self::parse_prop(child)?);
                    }
                    "value" => {
                        values.push(Self::parse_value(child)?);
                    }
                    _ => {}
                }
            }
        }

        Ok(MessageDef {
            name,
            description,
            props,
            values,
        })
    }

    fn parse_prop(node: &KdlNode) -> Result<PropDef, Error> {
        let name = node.entries().first()
            .and_then(|e| e.value().as_string())
            .ok_or_else(|| Error::Schema("Property must have a name".into()))?
            .to_string();

        let ty = node.entries().iter()
            .find(|e| e.name().map(|n| n.value()) == Some("type"))
            .and_then(|e| e.value().as_string())
            .map(Self::parse_type)
            .unwrap_or(ValueType::String);

        let required = node.entries().iter()
            .find(|e| e.name().map(|n| n.value()) == Some("required"))
            .and_then(|e| e.value().as_bool())
            .unwrap_or(false);

        Ok(PropDef {
            name,
            ty,
            required,
            description: None,
        })
    }

    fn parse_value(node: &KdlNode) -> Result<ValueDef, Error> {
        let ty = node.entries().iter()
            .find(|e| e.name().map(|n| n.value()) == Some("type"))
            .and_then(|e| e.value().as_string())
            .map(Self::parse_type)
            .unwrap_or(ValueType::String);

        Ok(ValueDef {
            ty,
            description: None,
        })
    }

    fn parse_type(s: &str) -> ValueType {
        match s {
            "string" => ValueType::String,
            "i8" => ValueType::I8,
            "i16" => ValueType::I16,
            "i32" => ValueType::I32,
            "i64" => ValueType::I64,
            "u8" => ValueType::U8,
            "u16" => ValueType::U16,
            "u32" => ValueType::U32,
            "u64" => ValueType::U64,
            "f32" => ValueType::F32,
            "f64" => ValueType::F64,
            "bool" => ValueType::Bool,
            _ => ValueType::String,
        }
    }

    /// Validate a KDL node against this schema.
    pub fn validate(&self, node: &KdlNode) -> Result<(), Error> {
        let name = node.name().value();
        let msg_def = self.messages.get(name)
            .ok_or_else(|| Error::Validation(format!("Unknown message type: {}", name)))?;

        // Check required properties
        for prop in &msg_def.props {
            if prop.required {
                let has_prop = node.entries().iter()
                    .any(|e| e.name().map(|n| n.value()) == Some(&prop.name));
                if !has_prop {
                    return Err(Error::Validation(format!(
                        "Missing required property '{}' in message '{}'",
                        prop.name, name
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get a message definition by name.
    pub fn get_message(&self, name: &str) -> Option<&MessageDef> {
        self.messages.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_schema() {
        let input = r#"
            info {
                title "Test Protocol"
                version "1.0.0"
            }

            message "Connect" {
                prop "client_id" type="string" required=#true
                prop "version" type="u32"
            }

            message "Message" {
                prop "id" type="u64" required=#true
                value type="string"
            }
        "#;

        let schema = Schema::parse(input).unwrap();
        assert_eq!(schema.info.title, Some("Test Protocol".into()));
        assert_eq!(schema.messages.len(), 2);
        assert!(schema.messages.contains_key("Connect"));
        assert!(schema.messages.contains_key("Message"));
    }
}
