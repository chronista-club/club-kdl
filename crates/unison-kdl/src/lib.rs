//! # unison-kdl
//!
//! KDL serialization/deserialization with serde support.
//!
//! ## Example
//!
//! ```
//! use serde::{Deserialize, Serialize};
//! use unison_kdl::{from_str, to_string};
//!
//! #[derive(Debug, Serialize, Deserialize, PartialEq)]
//! struct Config {
//!     name: String,
//!     port: u16,
//!     debug: bool,
//! }
//!
//! // KDL -> Rust
//! let kdl = r#"config name="my-app" port=8080 debug=#true"#;
//! let config: Config = from_str(kdl).unwrap();
//!
//! assert_eq!(config.name, "my-app");
//! assert_eq!(config.port, 8080);
//! assert_eq!(config.debug, true);
//! ```

mod de;
mod error;
mod ser;

pub use de::from_str;
pub use error::Error;
pub use ser::{to_string, to_string_with_name};

// Re-export kdl types
pub use kdl;
