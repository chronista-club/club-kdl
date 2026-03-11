//! KDL Serialization
//!
//! High-performance serialization from Rust types to KDL.

use crate::error::Result;
use kdl::{KdlDocument, KdlEntry, KdlIdentifier, KdlNode, KdlValue};

/// Trait for types that can be serialized to KDL.
///
/// # Example
///
/// ```ignore
/// #[derive(KdlSerialize)]
/// #[kdl(name = "service")]
/// struct Service {
///     #[kdl(argument)]
///     name: String,
///     #[kdl(property)]
///     image: String,
/// }
/// ```
pub trait KdlSerialize {
    /// Serialize to a KDL node.
    fn to_kdl_node(&self) -> Result<KdlNode>;

    /// Serialize to a KDL document.
    fn to_kdl_doc(&self) -> Result<KdlDocument> {
        let node = self.to_kdl_node()?;
        let mut doc = KdlDocument::new();
        doc.nodes_mut().push(node);
        Ok(doc)
    }
}

/// Serialize a type to a KDL string.
///
/// # Example
///
/// ```ignore
/// let kdl_string = unison_kdl::to_string(&config)?;
/// ```
pub fn to_string<T: KdlSerialize>(value: &T) -> Result<String> {
    let doc = value.to_kdl_doc()?;
    Ok(doc.to_string())
}

/// Serialize a type to a KDL string with custom formatting.
pub fn to_string_pretty<T: KdlSerialize>(value: &T) -> Result<String> {
    let doc = value.to_kdl_doc()?;
    // KDL's default formatting is already pretty
    Ok(doc.to_string())
}

/// Serialize a type to a KDL document.
pub fn to_doc<T: KdlSerialize>(value: &T) -> Result<KdlDocument> {
    value.to_kdl_doc()
}

/// Serialize a type to a KDL node.
pub fn to_node<T: KdlSerialize>(value: &T) -> Result<KdlNode> {
    value.to_kdl_node()
}

// ============================================================================
// Primitive implementations
// ============================================================================

/// Helper trait to convert Rust values to KDL values
pub trait ToKdlValue {
    fn to_kdl_value(&self) -> KdlValue;
}

impl ToKdlValue for str {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::String(self.to_string())
    }
}

impl ToKdlValue for String {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::String(self.clone())
    }
}

impl ToKdlValue for i64 {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::Integer(*self as i128)
    }
}

impl ToKdlValue for i128 {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::Integer(*self)
    }
}

impl ToKdlValue for i32 {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::Integer(*self as i128)
    }
}

impl ToKdlValue for u64 {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::Integer(*self as i128)
    }
}

impl ToKdlValue for u32 {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::Integer(*self as i128)
    }
}

impl ToKdlValue for u16 {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::Integer(*self as i128)
    }
}

impl ToKdlValue for usize {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::Integer(*self as i128)
    }
}

impl ToKdlValue for f64 {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::Float(*self)
    }
}

impl ToKdlValue for bool {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::Bool(*self)
    }
}

// PathBuf support
impl ToKdlValue for std::path::PathBuf {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::String(self.to_string_lossy().into_owned())
    }
}

impl ToKdlValue for &std::path::PathBuf {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::String(self.to_string_lossy().into_owned())
    }
}

impl ToKdlValue for std::path::Path {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::String(self.to_string_lossy().into_owned())
    }
}

impl ToKdlValue for &std::path::Path {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::String(self.to_string_lossy().into_owned())
    }
}

impl<T: ToKdlValue> ToKdlValue for Option<T> {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        match self {
            Some(v) => v.to_kdl_value(),
            None => KdlValue::Null,
        }
    }
}

// Reference implementations for derive macro support
macro_rules! impl_ref_to_kdl_value {
    ($($ty:ty),*) => {
        $(
            impl ToKdlValue for &$ty {
                #[inline]
                fn to_kdl_value(&self) -> KdlValue {
                    (*self).to_kdl_value()
                }
            }
        )*
    };
}

impl_ref_to_kdl_value!(i64, i128, i32, u64, u32, u16, usize, f64, bool);

impl ToKdlValue for &String {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::String((*self).clone())
    }
}

impl ToKdlValue for &str {
    #[inline]
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::String(self.to_string())
    }
}

// ============================================================================
// Node builder helpers
// ============================================================================

/// Builder for constructing KDL nodes
pub struct NodeBuilder {
    node: KdlNode,
}

impl NodeBuilder {
    /// Create a new node builder with the given name
    #[inline]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            node: KdlNode::new(KdlIdentifier::from(name.into())),
        }
    }

    /// Add an argument (positional value)
    #[inline]
    pub fn arg(mut self, value: impl ToKdlValue) -> Self {
        self.node.push(KdlEntry::new(value.to_kdl_value()));
        self
    }

    /// Add a property (key=value)
    #[inline]
    pub fn prop(mut self, name: impl Into<String>, value: impl ToKdlValue) -> Self {
        self.node.push(KdlEntry::new_prop(
            KdlIdentifier::from(name.into()),
            value.to_kdl_value(),
        ));
        self
    }

    /// Add a child node
    #[inline]
    pub fn child(mut self, node: KdlNode) -> Self {
        let children = self
            .node
            .children_mut()
            .get_or_insert_with(KdlDocument::new);
        children.nodes_mut().push(node);
        self
    }

    /// Add multiple child nodes
    #[inline]
    pub fn children(mut self, nodes: impl IntoIterator<Item = KdlNode>) -> Self {
        let children = self
            .node
            .children_mut()
            .get_or_insert_with(KdlDocument::new);
        children.nodes_mut().extend(nodes);
        self
    }

    /// Add a raw entry (for flatten support)
    #[inline]
    pub fn entry(mut self, entry: KdlEntry) -> Self {
        self.node.push(entry);
        self
    }

    /// Build the node
    #[inline]
    pub fn build(self) -> KdlNode {
        self.node
    }
}
