//! # club-kdl
//!
//! High-performance KDL serialization and deserialization for Rust.
//!
//! This crate provides traits and derive macros for converting between
//! KDL documents and Rust structs with minimal allocations.
//!
//! ## Features
//!
//! - **Own KDL v2 engine** - A single-pass parser builds a lightweight
//!   [`Document`] tree that borrows strings from the input; derived
//!   serializers write straight to text without building a tree
//! - **Attribute-based mapping** - Fine-grained control with `#[kdl(...)]`
//! - **Spec-checked** - Runs the full official KDL 2.0 test suite in CI
//!
//! ## Reading KDL without types
//!
//! Not every document maps neatly onto structs. [`Document::parse`] gives you
//! the tree itself — node names, arguments, properties, children — with no
//! schema and no Cargo feature:
//!
//! ```
//! let doc = club_kdl::Document::parse(r#"service "api" image="nginx" { port 80 }"#)?;
//! let service = &doc.nodes()[0];
//! assert_eq!(service.name(), "service");
//! assert_eq!(service.prop("image").and_then(|v| v.as_string()), Some("nginx"));
//! assert_eq!(service.child("port").and_then(|p| p.arg(0)).and_then(|v| v.as_integer()), Some(80));
//! # Ok::<(), club_kdl::Error>(())
//! ```
//!
//! See the [`ast`] module for a fuller example and an overview of what the
//! tree gives you.
//!
//! Comments and layout are not preserved. For format-preserving edits, hand
//! data to [kdl-rs](https://docs.rs/kdl) with `to_kdl_document` /
//! `from_kdl_document`.
//!
//! ## Cargo features
//!
//! Both are off by default.
//!
//! - `compose` - `club_kdl::compose` module: multi-file KDL via
//!   `(<)file` / `(<)glob` include directives (adds the `glob` dependency)
//! - `kdl-rs` - interop with [kdl-rs](https://docs.rs/kdl): `from_kdl_document`,
//!   `from_kdl_node`, `to_kdl_document`, `From` conversions between the two
//!   trees, and the `club_kdl::kdl` re-export (adds the `kdl` dependency)
//!
//! ## Example
//!
//! ```
//! use club_kdl::{KdlDeserialize, KdlSerialize};
//!
//! #[derive(Debug, KdlDeserialize, KdlSerialize)]
//! #[kdl(name = "service")]
//! struct Service {
//!     #[kdl(argument)]
//!     name: String,
//!
//!     #[kdl(property)]
//!     image: String,
//!
//!     #[kdl(children, name = "port")]
//!     ports: Vec<Port>,
//! }
//!
//! #[derive(Debug, KdlDeserialize, KdlSerialize)]
//! #[kdl(name = "port")]
//! struct Port {
//!     #[kdl(property)]
//!     host: u16,
//!     #[kdl(property)]
//!     container: u16,
//! }
//!
//! let kdl = r#"
//!     service "api" image="myapp" {
//!         port host=8080 container=80
//!     }
//! "#;
//!
//! let svc: Service = club_kdl::from_str(kdl).unwrap();
//! assert_eq!(svc.name, "api");
//! assert_eq!(svc.ports.len(), 1);
//!
//! let out = club_kdl::to_string_pretty(&svc).unwrap();
//! assert!(out.contains("service"));
//! ```
//!
//! ## Attributes
//!
//! ### Container attributes (`#[kdl(...)]` on struct)
//!
//! - `name = "..."` - KDL node name (defaults to struct name in snake_case)
//! - `alias = "..."` - Alternative node name accepted during deserialization (multiple allowed)
//! - `document` - Treat as document-level (multiple top-level nodes)
//!
//! ### Field attributes (`#[kdl(...)]` on fields)
//!
//! - `argument` - Map to positional argument (by index)
//! - `argument(index = N)` - Map to specific argument index
//! - `arguments` - Collect all arguments into `Vec<T>`
//! - `property` - Map to property (key=value)
//! - `property(rename = "...")` - Map to property with different name
//! - `child` - Map to single child node (auto-resolves name from child type's `#[kdl(name)]`)
//! - `child(name = "...")` - Map to child node with explicit name
//! - `child, unwrap_arg` - Extract child node's first argument as value
//! - `child, unwrap_args` - Extract child node's all arguments as `Vec<T>`
//! - `children` - Map to multiple child nodes (auto-resolves name from child type's `#[kdl(name)]`)
//! - `children(name = "...")` - Filter children by explicit node name
//! - `child_map` - Collect child nodes into `HashMap<String, String>`
//! - `child_map(name = "...")` - Collect from wrapper node into HashMap
//! - `default` - Use Default::default() if missing
//! - `skip` - Skip this field during serialization/deserialization

pub mod ast;
#[cfg(feature = "compact")]
pub mod compact;
#[cfg(feature = "compose")]
pub mod compose;
pub mod de;
pub mod error;
mod parse;
pub mod ser;
pub mod stream;
mod write;

// Re-exports
pub use ast::{Document, Entry, Node, Value};
pub use de::{FromKdlValue, KdlDeserialize, from_doc, from_node, from_str};
#[cfg(feature = "kdl-rs")]
pub use de::{KdlNodeExt, from_kdl_document, from_kdl_node};
pub use error::{Error, Result};
#[cfg(feature = "kdl-rs")]
pub use ser::to_kdl_document;
pub use ser::{
    KdlSerialize, NodeBuilder, ToKdlValue, to_doc, to_node, to_string, to_string_into,
    to_string_pretty,
};
pub use stream::append_node;

/// Items used by code generated with the derive macros. Not a stable API.
#[doc(hidden)]
pub mod __private {
    pub use crate::write::NodeWriter;
}

/// The [kdl-rs](https://docs.rs/kdl) crate, for interop: reading its trees
/// with [`from_kdl_document`] / [`from_kdl_node`], or converting club-kdl
/// trees into it (`kdl::KdlDocument::from(&doc)`, [`to_kdl_document`]) for
/// format-preserving edits. Requires the `kdl-rs` feature.
#[cfg(feature = "kdl-rs")]
pub use kdl;

// Derive macros
pub use club_kdl_derive::{KdlDeserialize, KdlSerialize};
