//! Language emitters — one [`crate::Emitter`] implementation per target.
//!
//! Each emitter consumes the shared [`crate::ir::Schema`] and renders
//! target-language source text. Targets:
//!
//! - [`RustEmitter`] / [`TypeScriptEmitter`] — ported from club-unison's
//!   codegen.
//! - [`ZodEmitter`] — runtime validators (TypeScript / Zod).
//! - [`SurrealQlEmitter`] — SurrealDB schema DDL (data dialect only).
//!
//! Emitters are pure `ir::Schema -> String` functions and depend only on
//! `std` (the case-conversion helpers in `case` replace the `convert_case`
//! crate used by club-unison).

mod case;
pub mod rust;
pub mod surrealql;
pub mod typescript;
pub mod zod;

pub use rust::RustEmitter;
pub use surrealql::SurrealQlEmitter;
pub use typescript::TypeScriptEmitter;
pub use zod::ZodEmitter;
