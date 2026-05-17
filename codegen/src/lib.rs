//! # club-kdl-codegen
//!
//! Generate type / schema definitions in multiple languages from a single
//! KDL schema file (KDL-first). Targets implemented: **Rust** and
//! **TypeScript**. Zod and SurrealQL are planned (see `ROADMAP.md`, Phase 1).
//!
//! ## Pipeline
//!
//! ```text
//! *.kdl  ──parser──▶  Schema IR  ──emitter──▶  Rust / TypeScript
//! ```
//!
//! The intermediate [`ir::Schema`] representation decouples parsing from
//! emission: the parser is written once, and each target is one [`Emitter`]
//! implementation.
//!
//! See the design memory `mem_1Cb5mWnMTdzXfJVoNGFwup` and `ROADMAP.md`
//! (Phase 1) for the full plan.

pub mod emit;
pub mod ir;
pub mod parser;

/// A code generation target. Each language emitter (currently
/// [`emit::RustEmitter`] / [`emit::TypeScriptEmitter`]) implements this trait
/// against the shared [`ir::Schema`].
pub trait Emitter {
    /// Render the given schema into target-language source text.
    fn emit(&self, schema: &ir::Schema) -> String;
}
