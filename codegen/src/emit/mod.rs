//! Language emitters — one [`crate::Emitter`] implementation per target.
//!
//! Each emitter consumes the shared [`crate::ir::Schema`] and renders
//! target-language source text. Targets:
//!
//! - `rust` / `typescript` — ported from club-unison's codegen (Phase 1 Step 4).
//! - `zod` / `surrealql` — new (Phase 1 Step 5).
//!
//! Scaffold stage: implementations land in this module's submodules.
