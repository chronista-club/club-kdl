//! # unison-kdl
//!
//! A KDL (KDL Document Language) v2 parser with serde support.
//!
//! ## Example
//!
//! ```
//! use unison_kdl::KdlDocument;
//!
//! let input = r#"
//! node "arg1" "arg2" key="value" {
//!     child 1 2 3
//! }
//! "#;
//!
//! let doc: KdlDocument = input.parse().unwrap();
//! ```

mod ast;
mod error;
mod lexer;
mod parser;

pub use ast::*;
pub use error::*;
