//! Code generation for Rust and TypeScript.

mod rust;
mod typescript;

pub use rust::RustCodeGen;
pub use typescript::TypeScriptCodeGen;

use crate::schema::Schema;

/// Code generator entry point.
pub struct CodeGen;

impl CodeGen {
    /// Create a Rust code generator.
    pub fn rust(schema: &Schema) -> RustCodeGen<'_> {
        RustCodeGen::new(schema)
    }

    /// Create a TypeScript code generator.
    pub fn typescript(schema: &Schema) -> TypeScriptCodeGen<'_> {
        TypeScriptCodeGen::new(schema)
    }
}
