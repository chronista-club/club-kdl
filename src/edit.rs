//! Source-preserving edits to existing KDL values (requires `edit`).
//!
//! The document owns its original text. An edit replaces only one value token;
//! comments, whitespace, type annotations, and other value spellings stay intact.
//! Each edit reparses the current text. This API does not insert, remove, or move
//! nodes or entries, merge structs, resolve imports, or access files.
//!
//! ```
//! use club_kdl::edit::EditableDocument;
//!
//! let mut doc = EditableDocument::parse(
//!     "// connection\nserver \"localhost\" port=8080 // development\n",
//! )?;
//! doc.set_property(&[0], "port", 9090)?;
//! assert_eq!(
//!     doc.as_str(),
//!     "// connection\nserver \"localhost\" port=9090 // development\n",
//! );
//! # Ok::<(), club_kdl::edit::EditError>(())
//! ```
//!
//! Paths are nonempty slices of zero-based node indices: `[1, 0]` means the
//! first child of the second top-level node. Only active nodes and entries count;
//! `/-` elements are retained as text but cannot be edited. Missing targets return
//! [`EditError`] without modifying the text.

use std::fmt;

use crate::{Document, Node, ToKdlValue, Value};

/// An invalid document or an edit that does not select an existing value.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EditError {
    /// The source is not a valid KDL v2 document.
    #[error(transparent)]
    Parse(#[from] crate::Error),
    /// A node path is empty or contains an out-of-bounds index.
    #[error("node path {path:?} does not select an existing node")]
    MissingNode { path: Vec<usize> },
    /// The selected node has no positional argument at this index.
    #[error("node {path:?} has no argument at index {index}")]
    MissingArgument { path: Vec<usize>, index: usize },
    /// The selected node has no active property with this name.
    #[error("node {path:?} has no property {name:?}")]
    MissingProperty { path: Vec<usize>, name: String },
}

/// A validated KDL v2 document that retains its original text.
#[derive(Debug, Clone)]
pub struct EditableDocument {
    source: String,
}

impl EditableDocument {
    /// Validate and retain a KDL v2 document, including comments and formatting.
    pub fn parse(source: &str) -> Result<Self, EditError> {
        Document::parse(source)?;
        Ok(Self {
            source: source.to_owned(),
        })
    }

    /// The current document text, with untouched bytes preserved exactly.
    pub fn as_str(&self) -> &str {
        &self.source
    }

    /// Consume the document and return its text without copying.
    pub fn into_string(self) -> String {
        self.source
    }

    /// Replace an existing positional argument. Node paths and argument indices
    /// are zero-based; slashdashed nodes and entries do not count.
    ///
    /// Properties do not count towards the argument index. The existing type
    /// annotation is retained without schema validation. Only the selected value
    /// token is normalized, even when setting the same semantic value.
    pub fn set_argument(
        &mut self,
        path: &[usize],
        index: usize,
        value: impl ToKdlValue,
    ) -> Result<(), EditError> {
        self.replace_value(path, Target::Argument(index), value.to_kdl_value())
    }

    /// Replace the rightmost active property with `name` in the selected node.
    /// A missing property is an error; it is not inserted.
    ///
    /// The property key and type annotation are retained verbatim. Only the value
    /// token is normalized. Errors leave the document unchanged.
    pub fn set_property(
        &mut self,
        path: &[usize],
        name: &str,
        value: impl ToKdlValue,
    ) -> Result<(), EditError> {
        self.replace_value(path, Target::Property(name), value.to_kdl_value())
    }

    fn replace_value(
        &mut self,
        path: &[usize],
        target: Target<'_>,
        value: Value<'_>,
    ) -> Result<(), EditError> {
        // Reparse the current text so successive length-changing edits cannot
        // reuse stale offsets. The AST borrows only for the duration of this edit.
        let (doc, spans) = crate::parse::parse_document_with_spans(&self.source)?;
        let (node, offset) =
            locate_node(doc.nodes(), path).ok_or_else(|| EditError::MissingNode {
                path: path.to_vec(),
            })?;
        let entry_index = match target {
            Target::Argument(index) => node
                .entries()
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.name().is_none())
                .nth(index)
                .map(|(i, _)| i)
                .ok_or_else(|| EditError::MissingArgument {
                    path: path.to_vec(),
                    index,
                })?,
            Target::Property(name) => node
                .entries()
                .iter()
                .rposition(|entry| entry.name() == Some(name))
                .ok_or_else(|| EditError::MissingProperty {
                    path: path.to_vec(),
                    name: name.to_owned(),
                })?,
        };
        // The parser records exactly one range per active entry, in AST order.
        let range = spans[offset + entry_index].clone();
        let replacement = value.to_string();
        self.source.replace_range(range, &replacement);
        Ok(())
    }
}

enum Target<'a> {
    Argument(usize),
    Property(&'a str),
}

/// Return the node and the number of active entries preceding it in depth-first
/// order. Entries precede children in valid KDL, so this matches the span list.
fn locate_node<'n, 's>(nodes: &'n [Node<'s>], path: &[usize]) -> Option<(&'n Node<'s>, usize)> {
    let (&index, tail) = path.split_first()?;
    let node = nodes.get(index)?;
    let preceding: usize = nodes[..index].iter().map(entry_count).sum();
    if tail.is_empty() {
        Some((node, preceding))
    } else {
        locate_node(node.children(), tail)
            .map(|(child, offset)| (child, preceding + node.entries().len() + offset))
    }
}

fn entry_count(node: &Node<'_>) -> usize {
    node.entries().len() + node.children().iter().map(entry_count).sum::<usize>()
}

impl fmt::Display for EditableDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.source)
    }
}
