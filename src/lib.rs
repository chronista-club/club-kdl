//! # unison-kdl
//!
//! High-performance KDL serialization and deserialization for Rust.
//!
//! This crate provides traits and derive macros for converting between
//! KDL documents and Rust structs with minimal allocations.
//!
//! ## Features
//!
//! - **Zero-copy deserialization** - Borrow strings directly from KDL source
//! - **Attribute-based mapping** - Fine-grained control with `#[kdl(...)]`
//! - **High performance** - Optimized for speed with minimal allocations
//!
//! ## Example
//!
//! ```ignore
//! use unison_kdl::{KdlDeserialize, KdlSerialize};
//!
//! #[derive(KdlDeserialize, KdlSerialize)]
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
//! #[derive(KdlDeserialize, KdlSerialize)]
//! #[kdl(name = "port")]
//! struct Port {
//!     #[kdl(property)]
//!     host: u16,
//!     #[kdl(property)]
//!     container: u16,
//! }
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

pub mod de;
pub mod error;
pub mod ser;

// Re-exports
pub use de::{
    FromKdlValue, KdlDeserialize, KdlNodeExt, doc_to_wrapper_node, from_doc, from_node, from_str,
};
pub use error::{Error, Result};
pub use ser::{
    KdlSerialize, NodeBuilder, ToKdlValue, to_doc, to_node, to_string, to_string_pretty,
};

// Re-export kdl types for convenience
pub use kdl::{KdlDocument, KdlNode, KdlValue};

// Derive macros
pub use unison_kdl_derive::{KdlDeserialize, KdlSerialize};
