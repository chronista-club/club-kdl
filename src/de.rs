//! KDL Deserialization
//!
//! High-performance deserialization from KDL to Rust types.
//! Prioritizes zero-copy operations where possible.

use crate::error::{Error, Result};
use kdl::{KdlDocument, KdlNode, KdlValue};

/// Trait for types that can be deserialized from KDL.
///
/// The lifetime `'de` allows zero-copy deserialization by borrowing
/// string data directly from the KDL source.
///
/// # Example
///
/// ```ignore
/// #[derive(KdlDeserialize)]
/// #[kdl(name = "service")]
/// struct Service<'a> {
///     #[kdl(argument)]
///     name: &'a str,
///     #[kdl(property)]
///     image: String,
/// }
/// ```
pub trait KdlDeserialize<'de>: Sized {
    /// Deserialize from a KDL node.
    fn from_kdl_node(node: &'de KdlNode) -> Result<Self>;

    /// Deserialize from a KDL document (uses first node).
    fn from_kdl_doc(doc: &'de KdlDocument) -> Result<Self> {
        doc.nodes()
            .first()
            .ok_or(Error::Custom("empty document".into()))
            .and_then(Self::from_kdl_node)
    }
}

/// Deserialize a type from a KDL string.
///
/// # Example
///
/// ```ignore
/// let config: Config = unison_kdl::from_str(r#"
///     config {
///         name "my-app"
///         port 8080
///     }
/// "#)?;
/// ```
pub fn from_str<T>(s: &str) -> Result<T>
where
    T: for<'de> KdlDeserialize<'de>,
{
    let doc: KdlDocument = s
        .parse()
        .map_err(|e: kdl::KdlError| Error::Parse(e.to_string()))?;
    T::from_kdl_doc(&doc)
}

/// Deserialize a type from a KDL document.
pub fn from_doc<'de, T>(doc: &'de KdlDocument) -> Result<T>
where
    T: KdlDeserialize<'de>,
{
    T::from_kdl_doc(doc)
}

/// Deserialize a type from a KDL node.
pub fn from_node<'de, T>(node: &'de KdlNode) -> Result<T>
where
    T: KdlDeserialize<'de>,
{
    T::from_kdl_node(node)
}

// ============================================================================
// Primitive implementations
// ============================================================================

/// Helper to extract a value from a KdlEntry
pub trait FromKdlValue<'de>: Sized {
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self>;
}

impl<'de> FromKdlValue<'de> for &'de str {
    #[inline]
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        value
            .as_string()
            .ok_or_else(|| Error::type_mismatch("string", value))
    }
}

impl<'de> FromKdlValue<'de> for String {
    #[inline]
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        value
            .as_string()
            .map(|s| s.to_owned())
            .ok_or_else(|| Error::type_mismatch("string", value))
    }
}

impl<'de> FromKdlValue<'de> for i64 {
    #[inline]
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        value
            .as_integer()
            .and_then(|v| v.try_into().ok())
            .ok_or_else(|| Error::type_mismatch("integer", value))
    }
}

impl<'de> FromKdlValue<'de> for i128 {
    #[inline]
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        value
            .as_integer()
            .ok_or_else(|| Error::type_mismatch("integer", value))
    }
}

impl<'de> FromKdlValue<'de> for i32 {
    #[inline]
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        value
            .as_integer()
            .and_then(|v| v.try_into().ok())
            .ok_or_else(|| Error::type_mismatch("i32", value))
    }
}

impl<'de> FromKdlValue<'de> for u64 {
    #[inline]
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        value
            .as_integer()
            .and_then(|v| v.try_into().ok())
            .ok_or_else(|| Error::type_mismatch("u64", value))
    }
}

impl<'de> FromKdlValue<'de> for u32 {
    #[inline]
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        value
            .as_integer()
            .and_then(|v| v.try_into().ok())
            .ok_or_else(|| Error::type_mismatch("u32", value))
    }
}

impl<'de> FromKdlValue<'de> for u16 {
    #[inline]
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        value
            .as_integer()
            .and_then(|v| v.try_into().ok())
            .ok_or_else(|| Error::type_mismatch("u16", value))
    }
}

impl<'de> FromKdlValue<'de> for usize {
    #[inline]
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        value
            .as_integer()
            .and_then(|v| v.try_into().ok())
            .ok_or_else(|| Error::type_mismatch("usize", value))
    }
}

impl<'de> FromKdlValue<'de> for f64 {
    #[inline]
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        value
            .as_float()
            .ok_or_else(|| Error::type_mismatch("float", value))
    }
}

impl<'de> FromKdlValue<'de> for bool {
    #[inline]
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        value
            .as_bool()
            .ok_or_else(|| Error::type_mismatch("boolean", value))
    }
}

impl<'de, T: FromKdlValue<'de>> FromKdlValue<'de> for Option<T> {
    #[inline]
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        if value.is_null() {
            Ok(None)
        } else {
            T::from_kdl_value(value).map(Some)
        }
    }
}

// PathBuf support
impl<'de> FromKdlValue<'de> for std::path::PathBuf {
    #[inline]
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        value
            .as_string()
            .map(std::path::PathBuf::from)
            .ok_or_else(|| Error::type_mismatch("path string", value))
    }
}

// ============================================================================
// Document-level helpers
// ============================================================================

/// Create a wrapper KdlNode that contains all document nodes as children.
/// Used by `#[kdl(document)]` derive to support document-level deserialization.
pub fn doc_to_wrapper_node(doc: &KdlDocument) -> KdlNode {
    let mut wrapper = KdlNode::new(kdl::KdlIdentifier::from("__document__"));
    let mut children = KdlDocument::new();
    for node in doc.nodes() {
        children.nodes_mut().push(node.clone());
    }
    wrapper.set_children(children);
    wrapper
}

// ============================================================================
// Node access helpers
// ============================================================================

/// Extension trait for KdlNode with convenient accessor methods
pub trait KdlNodeExt {
    /// Get argument at index
    fn arg(&self, index: usize) -> Option<&KdlValue>;

    /// Get all arguments as a vector
    fn args(&self) -> Vec<&KdlValue>;

    /// Get property by name
    fn prop(&self, name: &str) -> Option<&KdlValue>;

    /// Get child node by name
    fn child(&self, name: &str) -> Option<&KdlNode>;

    /// Get all child nodes with a specific name
    fn children_by_name(&self, name: &str) -> Vec<&KdlNode>;

    /// Get all child nodes
    fn all_children(&self) -> Vec<&KdlNode>;
}

impl KdlNodeExt for KdlNode {
    #[inline]
    fn arg(&self, index: usize) -> Option<&KdlValue> {
        self.entries()
            .iter()
            .filter(|e| e.name().is_none())
            .nth(index)
            .map(|e| e.value())
    }

    #[inline]
    fn args(&self) -> Vec<&KdlValue> {
        self.entries()
            .iter()
            .filter(|e| e.name().is_none())
            .map(|e| e.value())
            .collect()
    }

    #[inline]
    fn prop(&self, name: &str) -> Option<&KdlValue> {
        self.entry(name).map(|e| e.value())
    }

    #[inline]
    fn child(&self, name: &str) -> Option<&KdlNode> {
        self.children()
            .and_then(|doc| doc.nodes().iter().find(|n| n.name().value() == name))
    }

    #[inline]
    fn children_by_name(&self, name: &str) -> Vec<&KdlNode> {
        self.children()
            .map(|doc| {
                doc.nodes()
                    .iter()
                    .filter(|n| n.name().value() == name)
                    .collect()
            })
            .unwrap_or_default()
    }

    #[inline]
    fn all_children(&self) -> Vec<&KdlNode> {
        self.children()
            .map(|doc| doc.nodes().iter().collect())
            .unwrap_or_default()
    }
}
