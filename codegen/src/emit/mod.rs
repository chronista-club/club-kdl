//! Language emitters — one [`crate::Emitter`] implementation per target.
//!
//! Each emitter consumes the shared [`crate::ir::Schema`] and renders
//! target-language source text. Targets:
//!
//! - [`RustEmitter`] / [`TypeScriptEmitter`] — ported from club-unison's
//!   codegen (Phase 1 Step 4).
//! - `zod` / `surrealql` — new (Phase 1 Step 5).
//!
//! Emitters are pure `ir::Schema -> String` functions and depend only on
//! `std` (the case-conversion helpers in `case` replace the `convert_case`
//! crate used by club-unison).

mod case;
pub mod rust;
pub mod typescript;

pub use rust::RustEmitter;
pub use typescript::TypeScriptEmitter;
