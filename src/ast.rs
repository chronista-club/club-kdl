//! The KDL document tree — read KDL without mapping it to types.
//!
//! [`Document::parse`] turns KDL text into a tree of [`Node`]s carrying node
//! names, type annotations, arguments, properties and children. Use it
//! directly when there is no fixed schema, when a document contains nodes
//! your structs do not describe (the tree keeps them all), or when you want
//! to deserialize only part of a document with [`from_node`](crate::from_node).
//! It needs no Cargo feature.
//!
//! The same tree is what [`from_str`](crate::from_str) parses into and what
//! derived [`KdlDeserialize`](crate::KdlDeserialize) impls read from.
//!
//! # Example: walk every node
//!
//! Collect each `route` node — at any depth — with its `request=` /
//! `response=` properties:
//!
//! ```
//! use club_kdl::{Document, Node};
//!
//! fn routes<'a>(node: &'a Node<'_>, out: &mut Vec<(&'a str, Option<&'a str>)>) {
//!     if node.name() == "route" {
//!         let name = node.arg(0).and_then(|v| v.as_string()).unwrap_or("");
//!         out.push((name, node.prop("request").and_then(|v| v.as_string())));
//!     }
//!     for child in node.children() {
//!         routes(child, out);
//!     }
//! }
//!
//! let doc = Document::parse(r#"
//!     protocol "creo" {
//!         route "memory.get" request="GetReq" response="GetRes"
//!         group "admin" { route "user.ban" request="BanReq"; }
//!     }
//! "#)?;
//! let mut out = Vec::new();
//! for node in doc.nodes() {
//!     routes(node, &mut out);
//! }
//! assert_eq!(out, [("memory.get", Some("GetReq")), ("user.ban", Some("BanReq"))]);
//!
//! // Errors carry a line and column.
//! let err = Document::parse("protocol {").unwrap_err();
//! assert!(err.to_string().contains("line 1, column 11"));
//! # Ok::<(), club_kdl::Error>(())
//! ```
//!
//! # What the tree gives you
//!
//! For a node like `(typ)service "api" image="nginx" { port 80; port 443; }`:
//!
//! | You want | Call | Here it returns |
//! |----------|------|-----------------|
//! | the node name | [`node.name()`](Node::name) | `"service"` |
//! | its type annotation | [`node.ty()`](Node::ty) | `Some("typ")` |
//! | an argument by position | [`node.arg(0)`](Node::arg) | `Some(&Value::String("api"))` |
//! | all arguments | [`node.args()`](Node::args) | an iterator over `"api"` |
//! | a property | [`node.prop("image")`](Node::prop) | `Some(&Value::String("nginx"))` (if a key repeats, the last one wins) |
//! | arguments and properties in written order | [`node.entries()`](Node::entries) | `&[Entry]` |
//! | the first child with a name | [`node.child("port")`](Node::child) | the `port 80` node |
//! | every child with a name | [`node.children_named("port")`](Node::children_named) | an iterator over both `port` nodes |
//! | all children | [`node.children()`](Node::children) | `&[Node]`, empty when there is no `{ }` block |
//! | the top-level nodes | [`doc.nodes()`](Document::nodes) | `&[Node]` |
//! | a top-level node by name | `doc.as_node().child("name")` | `Option<&Node>` |
//!
//! ```
//! let doc = club_kdl::Document::parse(
//!     r#"(typ)service "api" image="nginx" { port 80; port 443; }"#,
//! )?;
//! let node = &doc.nodes()[0];
//! assert_eq!(node.name(), "service");
//! assert_eq!(node.ty(), Some("typ"));
//! assert_eq!(node.arg(0).and_then(|v| v.as_string()), Some("api"));
//! assert_eq!(node.args().count(), 1);
//! assert_eq!(node.prop("image").and_then(|v| v.as_string()), Some("nginx"));
//! assert_eq!(node.entries().len(), 2);
//! assert_eq!(node.child("port").and_then(|p| p.arg(0)).and_then(|v| v.as_integer()), Some(80));
//! assert_eq!(node.children_named("port").count(), 2);
//! assert_eq!(node.children().len(), 2);
//! assert!(doc.as_node().child("service").is_some());
//! # Ok::<(), club_kdl::Error>(())
//! ```
//!
//! A [`Value`] is a string, integer, float, boolean or `#null`; read it with
//! [`as_string`](Value::as_string), [`as_integer`](Value::as_integer),
//! [`as_float`](Value::as_float), [`as_bool`](Value::as_bool) or
//! [`is_null`](Value::is_null), each of which returns `None` / `false` for the
//! other kinds.
//!
//! Good to know:
//!
//! - **Strings are borrowed.** A `Document<'a>` points into the text it was
//!   parsed from instead of copying it. Call [`Document::into_owned`] to keep
//!   the tree after the text is gone.
//! - **Only meaning is kept.** Whitespace, comments and the original spelling
//!   of values are dropped, so the tree is for reading and generating KDL.
//!   To edit a file in place and keep its comments, use kdl-rs (the `kdl-rs`
//!   feature converts between the two trees).

use std::borrow::Cow;
use std::fmt;

use crate::error::Result;

/// A parsed KDL document.
///
/// Internally the top-level nodes are stored as the children of an unnamed
/// root [`Node`], so document-level structs (`#[kdl(document)]`) read them
/// without copying.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document<'a> {
    pub(crate) root: Node<'a>,
}

impl<'a> Document<'a> {
    /// Parse a KDL v2 document.
    pub fn parse(src: &'a str) -> Result<Self> {
        crate::parse::parse_document(src)
    }

    /// Create a document from top-level nodes.
    pub fn from_nodes(nodes: Vec<Node<'a>>) -> Self {
        Self {
            root: Node {
                name: Cow::Borrowed(""),
                ty: None,
                entries: Vec::new(),
                children: nodes,
            },
        }
    }

    /// Top-level nodes.
    #[inline]
    pub fn nodes(&self) -> &[Node<'a>] {
        &self.root.children
    }

    /// Take the top-level nodes, consuming the document.
    #[inline]
    pub fn into_nodes(self) -> Vec<Node<'a>> {
        self.root.children
    }

    /// The unnamed root node whose children are the top-level nodes.
    #[inline]
    pub fn as_node(&self) -> &Node<'a> {
        &self.root
    }

    /// Append this document as KDL text to `out`.
    pub fn write_to(&self, out: &mut String) {
        crate::write::write_document(out, self);
    }

    /// Convert into a document that owns all of its string data.
    pub fn into_owned(self) -> Document<'static> {
        Document {
            root: self.root.into_owned(),
        }
    }
}

/// A KDL node: `(type)name arg key=value { children }`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Node<'a> {
    pub(crate) name: Cow<'a, str>,
    pub(crate) ty: Option<Cow<'a, str>>,
    pub(crate) entries: Vec<Entry<'a>>,
    pub(crate) children: Vec<Node<'a>>,
}

impl<'a> Node<'a> {
    /// Create a node with the given name and no entries or children.
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// Node name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Type annotation, e.g. `date` in `(date)node`.
    #[inline]
    pub fn ty(&self) -> Option<&str> {
        self.ty.as_deref()
    }

    /// All entries (arguments and properties) in source order.
    #[inline]
    pub fn entries(&self) -> &[Entry<'a>] {
        &self.entries
    }

    /// Child nodes (empty if the node has no children block).
    #[inline]
    pub fn children(&self) -> &[Node<'a>] {
        &self.children
    }

    /// Mutable access to the entries.
    #[inline]
    pub fn entries_mut(&mut self) -> &mut Vec<Entry<'a>> {
        &mut self.entries
    }

    /// Mutable access to the children.
    #[inline]
    pub fn children_mut(&mut self) -> &mut Vec<Node<'a>> {
        &mut self.children
    }

    /// Positional arguments in order.
    #[inline]
    pub fn args(&self) -> impl Iterator<Item = &Value<'a>> {
        self.entries
            .iter()
            .filter(|e| e.name.is_none())
            .map(|e| &e.value)
    }

    /// Positional argument at `index`.
    #[inline]
    pub fn arg(&self, index: usize) -> Option<&Value<'a>> {
        self.args().nth(index)
    }

    /// Property value by key. When a key appears more than once the
    /// rightmost one wins, as the KDL spec requires.
    #[inline]
    pub fn prop(&self, key: &str) -> Option<&Value<'a>> {
        self.entries
            .iter()
            .rev()
            .find(|e| e.name.as_deref() == Some(key))
            .map(|e| &e.value)
    }

    /// First child node with the given name.
    #[inline]
    pub fn child(&self, name: &str) -> Option<&Node<'a>> {
        self.children.iter().find(|n| n.name == name)
    }

    /// All child nodes with the given name.
    #[inline]
    pub fn children_named<'s>(&'s self, name: &'s str) -> impl Iterator<Item = &'s Node<'a>> {
        self.children.iter().filter(move |n| n.name == name)
    }

    /// Append this node (and its children) as KDL text to `out`.
    pub fn write_to(&self, out: &mut String) {
        crate::write::write_node(out, self, 0);
    }

    /// Rename the node.
    #[inline]
    pub fn set_name(&mut self, name: impl Into<Cow<'a, str>>) {
        self.name = name.into();
    }

    /// Set or clear the type annotation.
    #[inline]
    pub fn set_ty(&mut self, ty: Option<Cow<'a, str>>) {
        self.ty = ty;
    }

    /// Take the child nodes, consuming the node.
    #[inline]
    pub fn into_children(self) -> Vec<Node<'a>> {
        self.children
    }

    /// Split into entries and children, consuming the node.
    #[inline]
    pub fn into_parts(self) -> (Vec<Entry<'a>>, Vec<Node<'a>>) {
        (self.entries, self.children)
    }

    /// Convert into a node that owns all of its string data.
    pub fn into_owned(self) -> Node<'static> {
        Node {
            name: Cow::Owned(self.name.into_owned()),
            ty: self.ty.map(|t| Cow::Owned(t.into_owned())),
            entries: self.entries.into_iter().map(Entry::into_owned).collect(),
            children: self.children.into_iter().map(Node::into_owned).collect(),
        }
    }
}

/// A node entry: a positional argument or a `key=value` property.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry<'a> {
    pub(crate) name: Option<Cow<'a, str>>,
    pub(crate) ty: Option<Cow<'a, str>>,
    pub(crate) value: Value<'a>,
}

impl<'a> Entry<'a> {
    /// A positional argument.
    pub fn arg(value: Value<'a>) -> Self {
        Self {
            name: None,
            ty: None,
            value,
        }
    }

    /// A `key=value` property.
    pub fn prop(name: impl Into<Cow<'a, str>>, value: Value<'a>) -> Self {
        Self {
            name: Some(name.into()),
            ty: None,
            value,
        }
    }

    /// Property key, or `None` for a positional argument.
    #[inline]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Type annotation, e.g. `u8` in `(u8)1`.
    #[inline]
    pub fn ty(&self) -> Option<&str> {
        self.ty.as_deref()
    }

    /// The entry's value.
    #[inline]
    pub fn value(&self) -> &Value<'a> {
        &self.value
    }

    /// Convert into an entry that owns all of its string data.
    pub fn into_owned(self) -> Entry<'static> {
        Entry {
            name: self.name.map(|n| Cow::Owned(n.into_owned())),
            ty: self.ty.map(|t| Cow::Owned(t.into_owned())),
            value: self.value.into_owned(),
        }
    }
}

/// A KDL value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    /// A string (identifier, quoted or raw).
    String(Cow<'a, str>),
    /// An integer.
    Integer(i128),
    /// A floating point number, including `#inf`, `#-inf` and `#nan`.
    Float(f64),
    /// `#true` or `#false`.
    Bool(bool),
    /// `#null`.
    Null,
}

impl<'a> Value<'a> {
    /// The string contents, if this is a string.
    #[inline]
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// The integer, if this is an integer.
    #[inline]
    pub fn as_integer(&self) -> Option<i128> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// The float, if this is a float.
    #[inline]
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// The boolean, if this is a boolean.
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Whether this is `#null`.
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Convert into a value that owns its string data.
    pub fn into_owned(self) -> Value<'static> {
        match self {
            Value::String(s) => Value::String(Cow::Owned(s.into_owned())),
            Value::Integer(i) => Value::Integer(i),
            Value::Float(f) => Value::Float(f),
            Value::Bool(b) => Value::Bool(b),
            Value::Null => Value::Null,
        }
    }
}

impl fmt::Display for Value<'_> {
    /// Formats the value as KDL source.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        crate::write::write_value(&mut s, self);
        f.write_str(&s)
    }
}

impl fmt::Display for Node<'_> {
    /// Formats the node (and its children) as KDL source, ending in a newline.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        crate::write::write_node(&mut s, self, 0);
        f.write_str(&s)
    }
}

impl fmt::Display for Document<'_> {
    /// Formats the document as KDL source.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        crate::write::write_document(&mut s, self);
        f.write_str(&s)
    }
}

// ============================================================================
// kdl-rs interop
// ============================================================================

#[cfg(feature = "kdl-rs")]
mod kdl_rs_interop {
    use std::borrow::Cow;

    use super::{Document, Entry, Node, Value};

    impl<'a> From<&'a kdl::KdlDocument> for Document<'a> {
        fn from(doc: &'a kdl::KdlDocument) -> Self {
            Document::from_nodes(doc.nodes().iter().map(Node::from).collect())
        }
    }

    impl<'a> From<&'a kdl::KdlNode> for Node<'a> {
        fn from(node: &'a kdl::KdlNode) -> Self {
            Node {
                name: Cow::Borrowed(node.name().value()),
                ty: node.ty().map(|t| Cow::Borrowed(t.value())),
                entries: node
                    .entries()
                    .iter()
                    .map(|e| Entry {
                        name: e.name().map(|n| Cow::Borrowed(n.value())),
                        ty: e.ty().map(|t| Cow::Borrowed(t.value())),
                        value: Value::from(e.value()),
                    })
                    .collect(),
                children: node
                    .children()
                    .map(|doc| doc.nodes().iter().map(Node::from).collect())
                    .unwrap_or_default(),
            }
        }
    }

    impl<'a> From<&'a kdl::KdlValue> for Value<'a> {
        fn from(value: &'a kdl::KdlValue) -> Self {
            match value {
                kdl::KdlValue::String(s) => Value::String(Cow::Borrowed(s)),
                kdl::KdlValue::Integer(i) => Value::Integer(*i),
                kdl::KdlValue::Float(f) => Value::Float(*f),
                kdl::KdlValue::Bool(b) => Value::Bool(*b),
                kdl::KdlValue::Null => Value::Null,
            }
        }
    }

    impl From<&Document<'_>> for kdl::KdlDocument {
        fn from(doc: &Document<'_>) -> Self {
            let mut out = kdl::KdlDocument::new();
            out.nodes_mut()
                .extend(doc.nodes().iter().map(kdl::KdlNode::from));
            out
        }
    }

    impl From<&Node<'_>> for kdl::KdlNode {
        fn from(node: &Node<'_>) -> Self {
            let mut out = kdl::KdlNode::new(kdl::KdlIdentifier::from(node.name()));
            if let Some(ty) = node.ty() {
                out.set_ty(kdl::KdlIdentifier::from(ty));
            }
            for e in node.entries() {
                let value = kdl::KdlValue::from(e.value());
                let mut entry = match e.name() {
                    Some(name) => kdl::KdlEntry::new_prop(kdl::KdlIdentifier::from(name), value),
                    None => kdl::KdlEntry::new(value),
                };
                if let Some(ty) = e.ty() {
                    entry.set_ty(kdl::KdlIdentifier::from(ty));
                }
                out.push(entry);
            }
            if !node.children().is_empty() {
                let mut children = kdl::KdlDocument::new();
                children
                    .nodes_mut()
                    .extend(node.children().iter().map(kdl::KdlNode::from));
                out.set_children(children);
            }
            out
        }
    }

    impl From<&Value<'_>> for kdl::KdlValue {
        fn from(value: &Value<'_>) -> Self {
            match value {
                Value::String(s) => kdl::KdlValue::String(s.to_string()),
                Value::Integer(i) => kdl::KdlValue::Integer(*i),
                Value::Float(f) => kdl::KdlValue::Float(*f),
                Value::Bool(b) => kdl::KdlValue::Bool(*b),
                Value::Null => kdl::KdlValue::Null,
            }
        }
    }
}
