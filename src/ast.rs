//! KDL Abstract Syntax Tree types.

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use crate::error::KdlError;
use crate::parser::Parser;

/// A KDL document containing zero or more nodes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KdlDocument {
    nodes: Vec<KdlNode>,
}

impl KdlDocument {
    /// Creates an empty document.
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Returns the nodes in this document.
    pub fn nodes(&self) -> &[KdlNode] {
        &self.nodes
    }

    /// Returns a mutable reference to the nodes.
    pub fn nodes_mut(&mut self) -> &mut Vec<KdlNode> {
        &mut self.nodes
    }

    /// Gets a node by name.
    pub fn get(&self, name: &str) -> Option<&KdlNode> {
        self.nodes.iter().find(|n| n.name.value() == name)
    }

    /// Gets all nodes with the given name.
    pub fn get_all(&self, name: &str) -> impl Iterator<Item = &KdlNode> {
        self.nodes.iter().filter(move |n| n.name.value() == name)
    }
}

impl FromStr for KdlDocument {
    type Err = KdlError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Parser::new(s).parse_document()
    }
}

impl fmt::Display for KdlDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for node in &self.nodes {
            writeln!(f, "{}", node)?;
        }
        Ok(())
    }
}

/// A KDL node with a name, type annotation, entries, and optional children.
#[derive(Debug, Clone, PartialEq)]
pub struct KdlNode {
    /// Optional type annotation, e.g., `(date)node`
    pub ty: Option<KdlIdentifier>,
    /// The node name.
    pub name: KdlIdentifier,
    /// Arguments and properties.
    pub entries: Vec<KdlEntry>,
    /// Optional child document.
    pub children: Option<KdlDocument>,
}

impl KdlNode {
    /// Creates a new node with the given name.
    pub fn new(name: impl Into<KdlIdentifier>) -> Self {
        Self {
            ty: None,
            name: name.into(),
            entries: Vec::new(),
            children: None,
        }
    }

    /// Returns all arguments (entries without keys).
    pub fn arguments(&self) -> impl Iterator<Item = &KdlValue> {
        self.entries.iter().filter_map(|e| {
            if e.name.is_none() {
                Some(&e.value)
            } else {
                None
            }
        })
    }

    /// Returns all properties (entries with keys) as a map.
    pub fn properties(&self) -> HashMap<&str, &KdlValue> {
        self.entries
            .iter()
            .filter_map(|e| e.name.as_ref().map(|n| (n.value(), &e.value)))
            .collect()
    }

    /// Gets a property value by name.
    pub fn get(&self, key: &str) -> Option<&KdlValue> {
        self.entries
            .iter()
            .find(|e| e.name.as_ref().map(|n| n.value()) == Some(key))
            .map(|e| &e.value)
    }
}

impl fmt::Display for KdlNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ty) = &self.ty {
            write!(f, "({})", ty)?;
        }
        write!(f, "{}", self.name)?;

        for entry in &self.entries {
            write!(f, " {}", entry)?;
        }

        if let Some(children) = &self.children {
            writeln!(f, " {{")?;
            for node in children.nodes() {
                for line in node.to_string().lines() {
                    writeln!(f, "    {}", line)?;
                }
            }
            write!(f, "}}")?;
        }

        Ok(())
    }
}

/// A KDL entry: either an argument (positional value) or a property (key=value).
#[derive(Debug, Clone, PartialEq)]
pub struct KdlEntry {
    /// Optional type annotation for the value.
    pub ty: Option<KdlIdentifier>,
    /// Property name (None for arguments).
    pub name: Option<KdlIdentifier>,
    /// The value.
    pub value: KdlValue,
}

impl KdlEntry {
    /// Creates a new argument entry.
    pub fn argument(value: impl Into<KdlValue>) -> Self {
        Self {
            ty: None,
            name: None,
            value: value.into(),
        }
    }

    /// Creates a new property entry.
    pub fn property(name: impl Into<KdlIdentifier>, value: impl Into<KdlValue>) -> Self {
        Self {
            ty: None,
            name: Some(name.into()),
            value: value.into(),
        }
    }

    /// Returns true if this is an argument.
    pub fn is_argument(&self) -> bool {
        self.name.is_none()
    }

    /// Returns true if this is a property.
    pub fn is_property(&self) -> bool {
        self.name.is_some()
    }
}

impl fmt::Display for KdlEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ty) = &self.ty {
            write!(f, "({})", ty)?;
        }
        if let Some(name) = &self.name {
            write!(f, "{}=", name)?;
        }
        write!(f, "{}", self.value)
    }
}

/// A KDL value: string, number, boolean, or null.
#[derive(Debug, Clone, PartialEq)]
pub enum KdlValue {
    /// A string value.
    String(String),
    /// An integer value (stored as i128 for maximum range).
    Integer(i128),
    /// A floating-point value.
    Float(f64),
    /// A boolean value.
    Bool(bool),
    /// A null value.
    Null,
}

impl KdlValue {
    /// Returns the value as a string, if it is one.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            KdlValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as an integer, if it is one.
    pub fn as_integer(&self) -> Option<i128> {
        match self {
            KdlValue::Integer(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns the value as a float, if it is one.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            KdlValue::Float(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns the value as a bool, if it is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            KdlValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns true if the value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, KdlValue::Null)
    }
}

impl fmt::Display for KdlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KdlValue::String(s) => write!(f, "\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            KdlValue::Integer(n) => write!(f, "{}", n),
            KdlValue::Float(n) => {
                if n.is_nan() {
                    write!(f, "#nan")
                } else if n.is_infinite() {
                    if n.is_sign_positive() {
                        write!(f, "#inf")
                    } else {
                        write!(f, "#-inf")
                    }
                } else {
                    write!(f, "{}", n)
                }
            }
            KdlValue::Bool(b) => write!(f, "#{}", b),
            KdlValue::Null => write!(f, "#null"),
        }
    }
}

impl From<String> for KdlValue {
    fn from(s: String) -> Self {
        KdlValue::String(s)
    }
}

impl From<&str> for KdlValue {
    fn from(s: &str) -> Self {
        KdlValue::String(s.to_string())
    }
}

impl From<i128> for KdlValue {
    fn from(n: i128) -> Self {
        KdlValue::Integer(n)
    }
}

impl From<i64> for KdlValue {
    fn from(n: i64) -> Self {
        KdlValue::Integer(n as i128)
    }
}

impl From<i32> for KdlValue {
    fn from(n: i32) -> Self {
        KdlValue::Integer(n as i128)
    }
}

impl From<f64> for KdlValue {
    fn from(n: f64) -> Self {
        KdlValue::Float(n)
    }
}

impl From<bool> for KdlValue {
    fn from(b: bool) -> Self {
        KdlValue::Bool(b)
    }
}

/// A KDL identifier (node name, property name, or type annotation).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KdlIdentifier {
    value: String,
}

impl KdlIdentifier {
    /// Creates a new identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// Returns the identifier value.
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl From<String> for KdlIdentifier {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for KdlIdentifier {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for KdlIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Check if the identifier needs quoting
        let needs_quoting = self.value.is_empty()
            || self.value.chars().any(|c| {
                c.is_whitespace()
                    || matches!(c, '\\' | '/' | '(' | ')' | '{' | '}' | '<' | '>' | ';' | '[' | ']' | '=' | ',' | '"')
            });

        if needs_quoting {
            write!(f, "\"{}\"", self.value.replace('\\', "\\\\").replace('"', "\\\""))
        } else {
            write!(f, "{}", self.value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kdl_value_display() {
        assert_eq!(KdlValue::String("hello".into()).to_string(), "\"hello\"");
        assert_eq!(KdlValue::Integer(42).to_string(), "42");
        assert_eq!(KdlValue::Bool(true).to_string(), "#true");
        assert_eq!(KdlValue::Null.to_string(), "#null");
    }

    #[test]
    fn test_kdl_entry_display() {
        let arg = KdlEntry::argument("value");
        assert_eq!(arg.to_string(), "\"value\"");

        let prop = KdlEntry::property("key", "value");
        assert_eq!(prop.to_string(), "key=\"value\"");
    }
}
