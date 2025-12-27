//! # unison-protocol
//!
//! KDL-based protocol definition with schema validation and code generation.
//!
//! ## Example
//!
//! ```ignore
//! use unison_protocol::{Schema, CodeGen};
//!
//! // Load schema
//! let schema = Schema::load("protocol.kdl")?;
//!
//! // Validate a message
//! let msg: kdl::KdlNode = r#"Connect client_id="abc" version=1"#.parse()?;
//! schema.validate(&msg)?;
//!
//! // Generate code
//! CodeGen::rust(&schema).write_to("src/protocol.rs")?;
//! CodeGen::typescript(&schema).write_to("src/protocol.ts")?;
//! ```

pub mod schema;
pub mod codegen;
mod error;

pub use error::Error;
pub use schema::Schema;

// Re-export kdl types for convenience
pub use kdl::{KdlDocument, KdlNode, KdlEntry, KdlValue, KdlIdentifier};
