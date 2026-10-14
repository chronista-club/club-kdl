//! KDL Serialization
//!
//! High-performance serialization from Rust types to KDL.
//!
//! Types serialize into the lightweight [`crate::ast`] tree, borrowing string
//! data from `self` rather than copying it, and the tree is written out as
//! KDL v2 text by club-kdl's own writer.

use std::borrow::Cow;

use crate::ast::{Document, Entry, Node, Value};
use crate::error::Result;

/// Trait for types that can be serialized to KDL.
///
/// # Example
///
/// ```
/// use club_kdl::KdlSerialize;
///
/// #[derive(KdlSerialize)]
/// #[kdl(name = "service")]
/// struct Service {
///     #[kdl(argument)]
///     name: String,
///     #[kdl(property)]
///     image: String,
/// }
///
/// let svc = Service { name: "api".into(), image: "my app".into() };
/// let kdl = club_kdl::to_string(&svc).unwrap();
/// assert_eq!(kdl, "service api image=\"my app\"\n");
/// ```
pub trait KdlSerialize {
    /// Serialize to a KDL node. The node may borrow strings from `self`.
    fn to_kdl_node(&self) -> Result<Node<'_>>;

    /// Serialize to a KDL document.
    fn to_kdl_doc(&self) -> Result<Document<'_>> {
        Ok(Document::from_nodes(vec![self.to_kdl_node()?]))
    }

    /// Write this value as a node straight to text, optionally renamed.
    ///
    /// Derived impls write directly without building a [`Node`]; the
    /// default goes through [`to_kdl_node`](Self::to_kdl_node).
    #[doc(hidden)]
    fn __write_node(&self, out: &mut String, depth: usize, name: Option<&str>) -> Result<()> {
        let mut node = self.to_kdl_node()?;
        if let Some(name) = name {
            node.set_name(name);
        }
        crate::write::write_node(out, &node, depth);
        Ok(())
    }

    /// Append this value's entries to a node being written (`#[kdl(flatten)]`).
    #[doc(hidden)]
    fn __write_entries(&self, w: &mut crate::__private::NodeWriter<'_>) -> Result<()> {
        for entry in self.to_kdl_node()?.entries() {
            w.entry(entry);
        }
        Ok(())
    }

    /// Append this value's children to a node being written (`#[kdl(flatten)]`).
    #[doc(hidden)]
    fn __write_children(&self, w: &mut crate::__private::NodeWriter<'_>) -> Result<()> {
        for child in self.to_kdl_node()?.children() {
            w.child_node(child);
        }
        Ok(())
    }

    /// Write this value as a whole document straight to text.
    #[doc(hidden)]
    fn __write_doc(&self, out: &mut String) -> Result<()> {
        crate::write::write_document(out, &self.to_kdl_doc()?);
        Ok(())
    }
}

/// Serialize a type to a KDL string.
///
/// The output is normalized KDL v2: one node per line, children indented by
/// four spaces, strings written bare when that is valid.
///
/// # Example
///
/// ```
/// use club_kdl::KdlSerialize;
///
/// #[derive(KdlSerialize)]
/// #[kdl(name = "config")]
/// struct Config {
///     #[kdl(argument)]
///     name: String,
/// }
///
/// let config = Config { name: "my-app".into() };
/// let kdl_string = club_kdl::to_string(&config).unwrap();
/// assert_eq!(kdl_string, "config my-app\n");
/// ```
pub fn to_string<T: KdlSerialize + ?Sized>(value: &T) -> Result<String> {
    let mut out = String::with_capacity(256);
    value.__write_doc(&mut out)?;
    Ok(out)
}

/// Serialize a type and append the KDL text to `out`.
///
/// Lets callers reuse one buffer across many values (e.g. writing a stream
/// of records) instead of allocating a `String` per value.
pub fn to_string_into<T: KdlSerialize + ?Sized>(value: &T, out: &mut String) -> Result<()> {
    value.__write_doc(out)
}

/// Serialize a type to a KDL string.
///
/// Same as [`to_string`]; the output is always indented.
pub fn to_string_pretty<T: KdlSerialize + ?Sized>(value: &T) -> Result<String> {
    to_string(value)
}

/// Serialize a type to a KDL document.
pub fn to_doc<T: KdlSerialize + ?Sized>(value: &T) -> Result<Document<'_>> {
    value.to_kdl_doc()
}

/// Serialize a type to a KDL node.
pub fn to_node<T: KdlSerialize + ?Sized>(value: &T) -> Result<Node<'_>> {
    value.to_kdl_node()
}

/// Serialize a type to a [`kdl::KdlDocument`] for further editing with kdl-rs
/// (e.g. `autoformat`, comment insertion).
///
/// Requires the `kdl-rs` feature.
#[cfg(feature = "kdl-rs")]
pub fn to_kdl_document<T: KdlSerialize + ?Sized>(value: &T) -> Result<kdl::KdlDocument> {
    Ok(kdl::KdlDocument::from(&value.to_kdl_doc()?))
}

// ============================================================================
// Primitive implementations
// ============================================================================

/// Helper trait to convert Rust values to KDL values.
///
/// The returned [`Value`] may borrow from `self` (strings are not copied).
pub trait ToKdlValue {
    fn to_kdl_value(&self) -> Value<'_>;
}

impl<T: ToKdlValue + ?Sized> ToKdlValue for &T {
    #[inline]
    fn to_kdl_value(&self) -> Value<'_> {
        (**self).to_kdl_value()
    }
}

impl ToKdlValue for str {
    #[inline]
    fn to_kdl_value(&self) -> Value<'_> {
        Value::String(Cow::Borrowed(self))
    }
}

impl ToKdlValue for String {
    #[inline]
    fn to_kdl_value(&self) -> Value<'_> {
        Value::String(Cow::Borrowed(self))
    }
}

impl ToKdlValue for Cow<'_, str> {
    #[inline]
    fn to_kdl_value(&self) -> Value<'_> {
        Value::String(Cow::Borrowed(self))
    }
}

impl ToKdlValue for Value<'_> {
    #[inline]
    fn to_kdl_value(&self) -> Value<'_> {
        match self {
            Value::String(s) => Value::String(Cow::Borrowed(s)),
            Value::Integer(i) => Value::Integer(*i),
            Value::Float(f) => Value::Float(*f),
            Value::Bool(b) => Value::Bool(*b),
            Value::Null => Value::Null,
        }
    }
}

macro_rules! impl_int_to_kdl_value {
    ($($ty:ty),*) => {
        $(
            impl ToKdlValue for $ty {
                #[inline]
                fn to_kdl_value(&self) -> Value<'_> {
                    Value::Integer(*self as i128)
                }
            }
        )*
    };
}

impl_int_to_kdl_value!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, usize);

impl ToKdlValue for f64 {
    #[inline]
    fn to_kdl_value(&self) -> Value<'_> {
        Value::Float(*self)
    }
}

impl ToKdlValue for f32 {
    #[inline]
    fn to_kdl_value(&self) -> Value<'_> {
        Value::Float(f64::from(*self))
    }
}

impl ToKdlValue for bool {
    #[inline]
    fn to_kdl_value(&self) -> Value<'_> {
        Value::Bool(*self)
    }
}

impl ToKdlValue for std::path::Path {
    #[inline]
    fn to_kdl_value(&self) -> Value<'_> {
        Value::String(self.to_string_lossy())
    }
}

impl ToKdlValue for std::path::PathBuf {
    #[inline]
    fn to_kdl_value(&self) -> Value<'_> {
        Value::String(self.to_string_lossy())
    }
}

impl<T: ToKdlValue> ToKdlValue for Option<T> {
    #[inline]
    fn to_kdl_value(&self) -> Value<'_> {
        match self {
            Some(v) => v.to_kdl_value(),
            None => Value::Null,
        }
    }
}

// ============================================================================
// Node builder helpers
// ============================================================================

/// Builder for constructing KDL nodes.
///
/// Values are taken by reference so that string data is borrowed, not
/// copied, into the resulting [`Node`].
pub struct NodeBuilder<'a> {
    node: Node<'a>,
}

impl<'a> NodeBuilder<'a> {
    /// Create a new node builder with the given name
    #[inline]
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            node: Node::new(name),
        }
    }

    /// Add an argument (positional value)
    #[inline]
    pub fn arg<V: ToKdlValue + ?Sized>(mut self, value: &'a V) -> Self {
        self.node
            .entries_mut()
            .push(Entry::arg(value.to_kdl_value()));
        self
    }

    /// Add a property (key=value)
    #[inline]
    pub fn prop<V: ToKdlValue + ?Sized>(
        mut self,
        name: impl Into<Cow<'a, str>>,
        value: &'a V,
    ) -> Self {
        self.node
            .entries_mut()
            .push(Entry::prop(name, value.to_kdl_value()));
        self
    }

    /// Add a child node
    #[inline]
    pub fn child(mut self, node: Node<'a>) -> Self {
        self.node.children_mut().push(node);
        self
    }

    /// Add multiple child nodes
    #[inline]
    pub fn children(mut self, nodes: impl IntoIterator<Item = Node<'a>>) -> Self {
        self.node.children_mut().extend(nodes);
        self
    }

    /// Add a raw entry
    #[inline]
    pub fn entry(mut self, entry: Entry<'a>) -> Self {
        self.node.entries_mut().push(entry);
        self
    }

    /// Append another node's entries and children (for `#[kdl(flatten)]`).
    #[inline]
    pub fn flatten(mut self, node: Node<'a>) -> Self {
        let (entries, children) = node.into_parts();
        self.node.entries_mut().extend(entries);
        self.node.children_mut().extend(children);
        self
    }

    /// Build the node
    #[inline]
    pub fn build(self) -> Node<'a> {
        self.node
    }
}
