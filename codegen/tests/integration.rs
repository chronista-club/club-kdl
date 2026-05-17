//! End-to-end tests for `club-kdl-codegen`:
//!
//! - **library pipeline** — `parse` → `Emitter` for the Rust / TypeScript
//!   targets on a representative schema (data + protocol dialect).
//! - **CLI** — the `club-kdl-codegen` binary via subprocess.

use kdl_codegen::Emitter;
use kdl_codegen::emit::{RustEmitter, TypeScriptEmitter};
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
fn rust_and_typescript_outputs_are_non_empty_and_stable() {
    let schema = parse(SCHEMA).expect("schema parses");
    // Determinism: emitting twice yields byte-identical output.
    let rust = RustEmitter::new().emit(&schema);
    let ts = TypeScriptEmitter::new().emit(&schema);
    assert_eq!(rust, RustEmitter::new().emit(&schema));
    assert_eq!(ts, TypeScriptEmitter::new().emit(&schema));
    assert!(!rust.is_empty() && !ts.is_empty());
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
