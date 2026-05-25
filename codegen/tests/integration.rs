//! End-to-end tests for `club-kdl-codegen`:
//!
//! - **library pipeline** — `parse` → `Emitter` for all four targets
//!   (Rust / TypeScript / Zod / SurrealQL) on a representative schema
//!   exercising both the data and protocol dialects.
//! - **CLI** — the `club-kdl-codegen` binary via subprocess.

use kdl_codegen::Emitter;
use kdl_codegen::emit::{RustEmitter, SurrealQlEmitter, TypeScriptEmitter, ZodEmitter};
use kdl_codegen::parser::parse;

/// A representative schema exercising both dialects: standalone `struct` /
/// `enum` with an `array` and a named-type reference, plus a `protocol` with
/// a request/response and an event.
const SCHEMA: &str = r#"
    struct "User" {
        field "id" type="string"
        field "tags" type="array<string>" optional=#true
        field "role" type="Role" optional=#true
    }
    enum "Role" {
        variant "admin"
        variant "member"
    }
    protocol "chat" version="1.0.0" {
        namespace "demo.chat"
        channel "messaging" from="client" lifetime="persistent" {
            request "Send" {
                field "body" type="string"
                returns "Ack" {
                    field "id" type="string"
                }
            }
            event "Received" {
                field "body" type="string"
            }
        }
    }
"#;

// =============================================================================
// Library pipeline — parse → emit
// =============================================================================

#[test]
fn rust_pipeline_emits_all_dialect_constructs() {
    let schema = parse(SCHEMA).expect("schema parses");
    let out = RustEmitter::new().emit(&schema);

    // data dialect
    assert!(out.contains("pub struct User {"), "User struct");
    assert!(out.contains("pub enum Role {"), "Role enum");
    assert!(out.contains("Vec<String>"), "array<string> → Vec<String>");
    // `role` has no `required=` → optional; a named-type ref still resolves.
    assert!(
        out.contains("pub role: Option<Role>,"),
        "named-type reference (optional)"
    );
    // protocol dialect — request / returns / event payloads
    assert!(out.contains("pub struct Send {"), "request payload struct");
    assert!(out.contains("pub struct Ack {"), "returns payload struct");
    assert!(
        out.contains("pub struct Received {"),
        "event payload struct"
    );
    // optional field
    assert!(
        out.contains("pub tags: Option<Vec<String>>,"),
        "non-required field → Option"
    );
}

#[test]
fn typescript_pipeline_emits_all_dialect_constructs() {
    let schema = parse(SCHEMA).expect("schema parses");
    let out = TypeScriptEmitter::new().emit(&schema);

    assert!(out.contains("User"), "User type");
    assert!(out.contains("Role"), "Role type");
    assert!(out.contains("Send"), "request payload");
    assert!(out.contains("Ack"), "returns payload");
    assert!(out.contains("Received"), "event payload");
}

#[test]
fn zod_pipeline_emits_schemas_with_enum_first() {
    let schema = parse(SCHEMA).expect("schema parses");
    let out = ZodEmitter::new().emit(&schema);

    assert!(out.contains("import { z } from \"zod\";"), "zod import");
    assert!(out.contains("export const Role = z.enum("), "enum schema");
    assert!(
        out.contains("export const User = z.object({"),
        "object schema"
    );
    assert!(
        out.contains("export const Send = z.object({"),
        "protocol payload"
    );
    // enum must precede the struct that references it (Zod values can't be
    // forward-referenced).
    let role = out.find("export const Role").unwrap();
    let user = out.find("export const User").unwrap();
    assert!(role < user, "enum precedes the struct using it");
}

#[test]
fn surrealql_pipeline_emits_ddl_for_data_dialect_only() {
    let schema = parse(SCHEMA).expect("schema parses");
    let out = SurrealQlEmitter::new().emit(&schema);

    assert!(out.contains("DEFINE TABLE user SCHEMAFULL;"), "table DDL");
    assert!(
        out.contains("DEFINE FIELD id ON user TYPE string;"),
        "field DDL"
    );
    assert!(
        out.contains("ASSERT $value = NONE OR $value INSIDE ['admin', 'member']"),
        "optional enum field → NONE-guarded ASSERT clause"
    );
    // the protocol dialect has no DB representation — payload structs must
    // not leak into the DDL.
    assert!(!out.contains("Send"), "protocol payload not emitted to DDL");
}

#[test]
fn all_four_targets_are_non_empty_and_stable() {
    let schema = parse(SCHEMA).expect("schema parses");
    // Determinism: emitting twice yields byte-identical output.
    let rust = RustEmitter::new().emit(&schema);
    let ts = TypeScriptEmitter::new().emit(&schema);
    let zod = ZodEmitter::new().emit(&schema);
    let surql = SurrealQlEmitter::new().emit(&schema);
    assert_eq!(rust, RustEmitter::new().emit(&schema));
    assert_eq!(ts, TypeScriptEmitter::new().emit(&schema));
    assert_eq!(zod, ZodEmitter::new().emit(&schema));
    assert_eq!(surql, SurrealQlEmitter::new().emit(&schema));
    assert!(
        ![&rust, &ts, &zod, &surql].iter().any(|s| s.is_empty()),
        "every target produces non-empty output"
    );
}

// =============================================================================
// CLI — subprocess
// =============================================================================

use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_club-kdl-codegen"))
}

#[test]
fn cli_help_succeeds() {
    let out = cli().arg("--help").output().expect("runs");
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("usage:"));
}

#[test]
fn cli_missing_target_fails() {
    // A schema file but no --target.
    let out = cli().arg("some.kdl").output().expect("runs");
    assert!(!out.status.success(), "missing --target must fail");
}

#[test]
fn cli_unknown_target_fails() {
    let path = write_temp_schema("unknown-target");
    let out = cli()
        .arg(&path)
        .args(["--target", "cobol"])
        .output()
        .expect("runs");
    assert!(!out.status.success(), "unknown target must fail");
    std::fs::remove_file(&path).ok();
}

#[test]
fn cli_generates_rust_to_stdout() {
    let path = write_temp_schema("gen-rust");
    let out = cli()
        .arg(&path)
        .args(["--target", "rust"])
        .output()
        .expect("runs");
    assert!(out.status.success(), "rust generation should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("pub struct User {"));
    assert!(stdout.contains("pub enum Role {"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn cli_generates_zod_and_surrealql() {
    let path = write_temp_schema("gen-multi");
    for (target, needle) in [
        ("zod", "z.object("),
        ("surrealql", "DEFINE TABLE user SCHEMAFULL;"),
    ] {
        let out = cli()
            .arg(&path)
            .args(["--target", target])
            .output()
            .expect("runs");
        assert!(out.status.success(), "{target} generation should succeed");
        assert!(
            String::from_utf8_lossy(&out.stdout).contains(needle),
            "{target} output should contain {needle:?}"
        );
    }
    std::fs::remove_file(&path).ok();
}

/// Write [`SCHEMA`] to a process-unique temp file and return its path.
fn write_temp_schema(name: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "club-kdl-codegen-it-{}-{name}.kdl",
        std::process::id()
    ));
    std::fs::write(&path, SCHEMA).expect("write temp schema");
    path
}

// =============================================================================
// Tier 1 — entity dialect (record / relation / link / union) end-to-end
// =============================================================================

/// A schema exercising the Tier 1 entity dialect: a `record` with a
/// self-link, a literal-union field and a flexible object, plus a
/// property-carrying `relation`.
const ENTITY_SCHEMA: &str = r#"
    record "Atlas" {
        id strategy="uuidv7"
        field "name"       type="string"
        field "parent"     type="link<Atlas>" optional=#true
        field "visibility" type="'public' | 'private'" default="private" optional=#true
        field "metadata"   type="object" flexible=#true optional=#true
    }
    record "Memory" {
        field "body" type="string"
    }
    relation "derivedFrom" from="Memory" to="Memory" unique=#true {
        field "confidence" type="float" optional=#true
        field "reason"     type="string" optional=#true
    }
"#;

#[test]
fn entity_schema_parses_into_records_and_relations() {
    let schema = parse(ENTITY_SCHEMA).expect("entity schema parses");
    assert_eq!(schema.records.len(), 2);
    assert_eq!(schema.relations.len(), 1);
    assert_eq!(schema.relations[0].from, "Memory");
}

#[test]
fn surrealql_pipeline_emits_entity_ddl() {
    let schema = parse(ENTITY_SCHEMA).expect("entity schema parses");
    let out = SurrealQlEmitter::new().emit(&schema);
    // record → TYPE NORMAL table.
    assert!(out.contains("DEFINE TABLE atlas TYPE NORMAL SCHEMAFULL;"));
    // self-link → record<atlas>.
    assert!(out.contains("DEFINE FIELD parent ON atlas TYPE option<record<atlas>>;"));
    // literal union → string + ASSERT (NONE-guarded, the field is optional),
    // plus a quoted DEFAULT.
    assert!(out.contains("ASSERT $value = NONE OR $value INSIDE ['public', 'private']"));
    assert!(out.contains("DEFAULT 'private'"));
    // flexible object — the field is optional, so the type is wrapped.
    assert!(out.contains("DEFINE FIELD metadata ON atlas FLEXIBLE TYPE option<object>;"));
    // relation → TYPE RELATION table + UNIQUE index.
    assert!(
        out.contains("DEFINE TABLE derived_from TYPE RELATION IN memory OUT memory SCHEMAFULL;")
    );
    assert!(
        out.contains(
            "DEFINE INDEX derived_from_unique_edge ON derived_from FIELDS in, out UNIQUE;"
        )
    );
}

#[test]
fn rust_typescript_zod_pipelines_emit_entity_types() {
    let schema = parse(ENTITY_SCHEMA).expect("entity schema parses");

    let rust = RustEmitter::new().emit(&schema);
    assert!(rust.contains("pub struct Atlas {"));
    assert!(rust.contains("pub struct DerivedFrom {"));
    assert!(rust.contains("pub id: String,"));

    let ts = TypeScriptEmitter::new().emit(&schema);
    assert!(ts.contains("export interface Atlas {"));
    assert!(ts.contains("export interface DerivedFrom {"));
    // `visibility` has no `required=`, so it is optional in the interface.
    assert!(ts.contains("  visibility?: 'public' | 'private';"));

    let zod = ZodEmitter::new().emit(&schema);
    assert!(zod.contains("export const Atlas = z.object({"));
    assert!(zod.contains("export const DerivedFrom = z.object({"));
    assert!(zod.contains("z.enum([\"public\", \"private\"])"));
}

// =============================================================================
// Tier 2 — description / constraints end-to-end
// =============================================================================

/// A schema exercising Tier 2: type / field descriptions and the full
/// constraint set (numeric range, string length, pattern).
const TIER2_SCHEMA: &str = r#"
    record "Memory" description="User memory with content and metadata" {
        field "content"    type="string" description="Memory content text" min_length=1
        field "confidence" type="float" min=0 max=1
        field "slug"       type="string" pattern="^[a-z]+$" max_length=32
    }
"#;

#[test]
fn tier2_surrealql_emits_comment_and_assert() {
    let schema = parse(TIER2_SCHEMA).expect("tier 2 schema parses");
    let out = SurrealQlEmitter::new().emit(&schema);
    // description -> COMMENT on table and field.
    assert!(out.contains("COMMENT 'User memory with content and metadata';"));
    assert!(out.contains("COMMENT 'Memory content text';"));
    // numeric range -> ASSERT.
    assert!(out.contains("ASSERT $value >= 0 AND $value <= 1"));
    // string length / pattern -> string::len / string::matches ASSERT.
    assert!(out.contains("ASSERT string::len($value) >= 1"));
    assert!(out.contains("string::matches($value, '^[a-z]+$')"));
}

#[test]
fn tier2_zod_emits_describe_and_constraints() {
    let schema = parse(TIER2_SCHEMA).expect("tier 2 schema parses");
    let out = ZodEmitter::new().emit(&schema);
    assert!(out.contains(".describe(\"User memory with content and metadata\")"));
    assert!(out.contains(".describe(\"Memory content text\")"));
    assert!(out.contains("z.number().min(0).max(1)"));
    assert!(out.contains("z.string().min(1)"));
    assert!(out.contains(".regex(/^[a-z]+$/)"));
}

#[test]
fn tier2_rust_and_typescript_carry_docs_but_not_constraints() {
    let schema = parse(TIER2_SCHEMA).expect("tier 2 schema parses");

    let rust = RustEmitter::new().emit(&schema);
    assert!(rust.contains("/// User memory with content and metadata"));
    assert!(rust.contains("/// Memory content text"));
    assert!(!rust.contains("minimum"), "no constraint metadata in Rust");

    let ts = TypeScriptEmitter::new().emit(&schema);
    assert!(ts.contains("/** User memory with content and metadata */"));
    assert!(ts.contains("/** Memory content text */"));
    assert!(!ts.contains("@minimum"), "no constraint metadata in TS");
}

// =============================================================================
// protocol dialect — envelope enum + identifier sanitize end-to-end
// =============================================================================

/// The VP sidebar-IPC spike: a `channel` with `envelope="t"`, `:`-bearing
/// request names, and a fieldless request. Mirrors handoff
/// `mem_1CbBXDUTbzbRpn1dnQzZt6`.
const ENVELOPE_SCHEMA: &str = r#"
    protocol "sidebar" version="1.0.0" {
        namespace "vp.sidebar"
        channel "ipc" from="client" lifetime="transient" envelope="t" {
            request "process:toggle" {
                field "path" type="string"
                field "expanded" type="bool"
            }
            request "lane:delete" {
                field "path" type="string"
                field "address" type="string"
            }
            request "process:add" {}
        }
    }
"#;

#[test]
fn envelope_schema_parses_with_envelope_tag() {
    let schema = parse(ENVELOPE_SCHEMA).expect("envelope schema parses");
    let channel = &schema.protocol.unwrap().channels[0];
    assert_eq!(channel.envelope.as_deref(), Some("t"));
    assert_eq!(channel.requests.len(), 3);
}

#[test]
fn envelope_schema_emits_compilable_rust_enum() {
    let schema = parse(ENVELOPE_SCHEMA).expect("envelope schema parses");
    let out = RustEmitter::new().emit(&schema);
    // `:`-bearing request names are sanitized into valid identifiers.
    assert!(out.contains("pub struct ProcessToggle {"));
    assert!(out.contains("pub struct LaneDelete {"));
    assert!(out.contains("pub struct ProcessAdd;"));
    assert!(
        !out.contains("process:toggle {"),
        "raw `:` name must not leak"
    );
    // the envelope enum is an internally tagged discriminated union.
    assert!(out.contains("#[serde(tag = \"t\")]"));
    assert!(out.contains("pub enum IpcEnvelope {"));
    assert!(out.contains("ProcessToggle(ProcessToggle),"));
    assert!(out.contains("LaneDelete(LaneDelete),"));
    assert!(
        out.contains("    ProcessAdd,\n"),
        "fieldless request → unit variant"
    );
    assert!(out.contains("#[serde(rename = \"lane:delete\")]"));
}

#[test]
fn envelope_schema_emits_typescript_and_zod_unions() {
    let schema = parse(ENVELOPE_SCHEMA).expect("envelope schema parses");

    let ts = TypeScriptEmitter::new().emit(&schema);
    assert!(ts.contains("export type IpcEnvelope ="));
    assert!(ts.contains("({ t: \"process:toggle\" } & ProcessToggle)"));
    assert!(ts.contains("({ t: \"lane:delete\" } & LaneDelete)"));

    let zod = ZodEmitter::new().emit(&schema);
    assert!(zod.contains("z.discriminatedUnion(\"t\", ["));
    assert!(zod.contains("ProcessToggle.extend({ t: z.literal(\"process:toggle\") })"));
    assert!(zod.contains("LaneDelete.extend({ t: z.literal(\"lane:delete\") })"));
}

// =============================================================================
// protocol dialect — generated output really compiles
// =============================================================================
//
// The tests above assert on substrings of the generated text. That catches a
// missing token but not a malformed one — a stray `#[serde(rename = ...)]`, an
// enum variant rustc rejects, an interface body `tsc` rejects. The two tests
// below close that gap: they feed the generated source to the real compiler
// (`cargo build` for Rust, `tsc --noEmit` for TypeScript), so a schema that
// emits syntactically broken code fails CI rather than passing a text assert.

/// The VP sidebar-IPC schema — all 11 `SidebarIpcMsg` variants, from handoff
/// `mem_1CbBXDUTbzbRpn1dnQzZt6` annotation `mem_1CbBaDWBeK7FeapgAQXyb3`. Used
/// as the compile fixture for its coverage in one schema: a single- and a
/// double-colon wire name (`lane:delete`, `project:clone:pickFolder`), two
/// fieldless requests, a request with optional fields, an `array<string>`
/// field, and multi-field requests — every identifier-sanitize and
/// envelope-variant code path the emitters have.
const SIDEBAR_IPC_SCHEMA: &str = r#"
    protocol "sidebar" version="1.0.0" {
        namespace "vp.sidebar"
        channel "ipc" from="client" lifetime="transient" envelope="t" {
            request "process:toggle" {
                field "path" type="string"
                field "expanded" type="bool"
            }
            request "process:reorder" {
                field "order" type="array<string>"
            }
            request "process:restart" {
                field "path" type="string"
            }
            request "process:add" {}
            request "lane:select" {
                field "path" type="string"
                field "address" type="string"
            }
            request "lane:delete" {
                field "path" type="string"
                field "address" type="string"
            }
            request "lane:restart" {
                field "path" type="string"
                field "address" type="string"
            }
            request "lane:add_wing" {
                field "path" type="string"
                field "name" type="string"
                field "branch" type="string" optional=#true
                field "stand" type="string" optional=#true
            }
            request "stands:fetch" {
                field "path" type="string"
            }
            request "stand:select" {
                field "path" type="string"
                field "kind" type="string"
            }
            request "project:clone:pickFolder" {}
        }
    }
"#;

/// The `cargo` binary that launched this test, for spawning a nested build.
fn cargo_bin() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

/// Compile `generated` Rust as a standalone library crate; return
/// `(succeeded, build_log)`.
///
/// The throwaway crate lives under `CARGO_TARGET_TMPDIR`. It carries an empty
/// `[workspace]` table so cargo does not fold it into the club-kdl workspace
/// it physically nests inside, and declares the crates the emitter's import
/// header names (`serde` / `anyhow` / `chrono` / `uuid`). A leading
/// `#![allow(unused_imports, dead_code)]`, plus dropping any inherited
/// `RUSTFLAGS`, keeps a small schema's unused imports from tripping the
/// `-D warnings` CI sets.
fn rust_source_compiles(generated: &str) -> (bool, String) {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("rust-compile-probe");
    let src_dir = dir.join("src");
    std::fs::create_dir_all(&src_dir).expect("create probe crate dir");
    std::fs::write(
        dir.join("Cargo.toml"),
        r#"[package]
name = "codegen-rust-compile-probe"
version = "0.0.0"
edition = "2024"

[lib]
path = "src/lib.rs"

[workspace]

[dependencies]
serde = { version = "1", features = ["derive"] }
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["serde"] }
"#,
    )
    .expect("write probe Cargo.toml");
    std::fs::write(
        src_dir.join("lib.rs"),
        format!("#![allow(unused_imports, dead_code)]\n{generated}"),
    )
    .expect("write probe lib.rs");

    let out = Command::new(cargo_bin())
        .current_dir(&dir)
        .args(["build", "--quiet"])
        .env_remove("RUSTFLAGS")
        .output()
        .expect("spawn cargo build");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    (out.status.success(), log)
}

#[test]
fn sidebar_ipc_rust_output_really_compiles() {
    let schema = parse(SIDEBAR_IPC_SCHEMA).expect("sidebar schema parses");
    let generated = RustEmitter::new().emit(&schema);
    let (ok, log) = rust_source_compiles(&generated);
    assert!(
        ok,
        "generated Rust must compile — `cargo build` reported:\n{log}"
    );
}

/// Type-check `generated` TypeScript with `tsc --noEmit` via Bun; return
/// `(succeeded, log)`, or `None` when Bun is absent.
///
/// Bun is the project's TypeScript toolchain (never npm). A local checkout
/// without Bun skips the check instead of failing; CI installs Bun via
/// `setup-bun`, so the gate still holds there.
fn typescript_source_typechecks(generated: &str) -> Option<(bool, String)> {
    let bun_present = Command::new("bun")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !bun_present {
        return None;
    }
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("ts-compile-probe");
    std::fs::create_dir_all(&dir).expect("create ts probe dir");
    std::fs::write(dir.join("generated.ts"), generated).expect("write generated.ts");

    let out = Command::new("bunx")
        .current_dir(&dir)
        .args(["tsc", "--noEmit", "--strict", "generated.ts"])
        .output()
        .expect("spawn bunx tsc");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    Some((out.status.success(), log))
}

#[test]
fn sidebar_ipc_typescript_output_really_compiles() {
    let schema = parse(SIDEBAR_IPC_SCHEMA).expect("sidebar schema parses");
    let generated = TypeScriptEmitter::new().emit(&schema);
    match typescript_source_typechecks(&generated) {
        Some((ok, log)) => assert!(
            ok,
            "generated TypeScript must type-check — `tsc` reported:\n{log}"
        ),
        None => eprintln!(
            "skipping TypeScript compile check: Bun not found \
             (CI installs it via setup-bun)"
        ),
    }
}

// =============================================================================
// `parse_path` — multi-file schemas via club-kdl-compose `(<)` directive
// =============================================================================
//
// `parse_path` runs the entry file through `kdl_compose::compose` before
// lowering, so a schema split across files via `(<)file` resolves
// transparently. We assert that the multi-file form produces byte-equivalent
// Rust to its inline equivalent, then check the CLI end-to-end.

/// Inline form of the `multi-file/` fixture: the two `struct` definitions
/// concatenated in the order the directive splices them.
const INLINE_EQUIVALENT_OF_MULTI_FILE: &str = r#"
    struct "User" {
        field "id" type="string"
    }

    struct "Item" {
        field "name" type="string"
    }
"#;

fn multi_file_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("multi-file")
        .join("root.kdl")
}

#[test]
fn parse_path_resolves_include_to_same_schema_as_inline() {
    let inline = parse(INLINE_EQUIVALENT_OF_MULTI_FILE).expect("inline parses");
    let composed = kdl_codegen::parser::parse_path(multi_file_root()).expect("multi-file resolves");

    // The IR derives `PartialEq`, so a structural compare is the strongest
    // assertion: same types in the same order, same fields, identical.
    assert_eq!(
        composed.types, inline.types,
        "multi-file schema must lower to the same IR as its inline equivalent"
    );
}

#[test]
fn parse_path_emits_same_rust_as_inline() {
    // Even if the IR ever diverges in non-semantic ways, the *emitted* code
    // is what the user actually sees; assert byte-equality of the Rust
    // output between the two parse paths.
    let inline = parse(INLINE_EQUIVALENT_OF_MULTI_FILE).expect("inline parses");
    let composed = kdl_codegen::parser::parse_path(multi_file_root()).expect("multi-file resolves");

    let rust_inline = RustEmitter::new().emit(&inline);
    let rust_composed = RustEmitter::new().emit(&composed);
    assert_eq!(rust_inline, rust_composed);
}

#[test]
fn parse_doc_works_on_composed_kdl_document() {
    // Library consumers who hold a `KdlDocument` directly (custom IO or
    // in-memory build) should be able to skip the string round-trip.
    let doc = kdl_compose::compose(multi_file_root()).expect("compose");
    let schema = kdl_codegen::parser::parse_doc(&doc).expect("parse_doc");
    let names: Vec<&str> = schema
        .types
        .iter()
        .map(|t| match t {
            kdl_codegen::ir::TypeDef::Struct { name, .. } => name.as_str(),
            kdl_codegen::ir::TypeDef::Enum { name, .. } => name.as_str(),
        })
        .collect();
    assert_eq!(names, vec!["User", "Item"]);
}

#[test]
fn cli_resolves_include_directives_in_multi_file_schema() {
    // End-to-end: the binary should accept a root file whose includes are
    // not on disk in the same node, and emit Rust covering both files'
    // top-level definitions.
    let out = cli()
        .arg(multi_file_root())
        .args(["--target", "rust"])
        .output()
        .expect("runs");
    assert!(
        out.status.success(),
        "cli failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("pub struct User {"),
        "missing User struct: {stdout}"
    );
    assert!(
        stdout.contains("pub struct Item {"),
        "missing Item struct: {stdout}"
    );
}
