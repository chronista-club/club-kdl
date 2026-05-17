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
        field "id" type="string" required=#true
        field "tags" type="array<string>"
        field "role" type="Role"
    }
    enum "Role" {
        variant "admin"
        variant "member"
    }
    protocol "chat" version="1.0.0" {
        namespace "demo.chat"
        channel "messaging" from="client" lifetime="persistent" {
            request "Send" {
                field "body" type="string" required=#true
                returns "Ack" {
                    field "id" type="string" required=#true
                }
            }
            event "Received" {
                field "body" type="string" required=#true
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
        out.contains("ASSERT $value IN ['admin', 'member']"),
        "enum field → ASSERT clause"
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
        field "name"       type="string" required=#true
        field "parent"     type="link<Atlas>"
        field "visibility" type="'public' | 'private'" default="private"
        field "metadata"   type="object" flexible=#true
    }
    record "Memory" {
        field "body" type="string" required=#true
    }
    relation "derivedFrom" from="Memory" to="Memory" unique=#true {
        field "confidence" type="float"
        field "reason"     type="string"
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
    // literal union → string + ASSERT, plus a quoted DEFAULT.
    assert!(out.contains("ASSERT $value IN ['public', 'private']"));
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
