//! `club-kdl-codegen` CLI — generate target-language code from a KDL schema.
//!
//! ```text
//! club-kdl-codegen <schema.kdl> --target <rust|typescript|zod|surrealql>
//! ```
//!
//! Reads the KDL schema file, parses it into the [`Schema`](kdl_codegen::ir::Schema)
//! IR, runs the requested emitter, and writes the generated source to stdout.
//!
//! This is a thin wrapper around the library API ([`kdl_codegen`]) for ad-hoc
//! generation and CI diffing. The library API remains the primary entry point
//! for consumers that integrate codegen into their build.

use kdl_codegen::Emitter;
use kdl_codegen::emit::{RustEmitter, SurrealQlEmitter, TypeScriptEmitter, ZodEmitter};
use std::process::ExitCode;

const USAGE: &str = "usage: club-kdl-codegen <schema.kdl> --target <rust|typescript|zod|surrealql>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    match run(&args) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(msg) => {
            eprintln!("club-kdl-codegen: {msg}");
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// Parse arguments, run the codegen pipeline, and return the generated source.
fn run(args: &[String]) -> Result<String, String> {
    let mut file: Option<&str> = None;
    let mut target: Option<&str> = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--target" | "-t" => {
                target = Some(iter.next().ok_or("--target needs a value")?);
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}"));
            }
            other => {
                if file.is_some() {
                    return Err(format!("unexpected extra argument: {other}"));
                }
                file = Some(other);
            }
        }
    }

    let file = file.ok_or("missing <schema.kdl> argument")?;
    let target = target.ok_or("missing --target option")?;

    let src = std::fs::read_to_string(file).map_err(|e| format!("cannot read {file}: {e}"))?;
    let schema = kdl_codegen::parser::parse(&src).map_err(|e| format!("parse error: {e}"))?;

    match target {
        "rust" | "rs" => Ok(RustEmitter::new().emit(&schema)),
        "typescript" | "ts" => Ok(TypeScriptEmitter::new().emit(&schema)),
        "zod" => Ok(ZodEmitter::new().emit(&schema)),
        "surrealql" | "surql" => Ok(SurrealQlEmitter::new().emit(&schema)),
        other => Err(format!(
            "unknown target: {other} (expected rust|typescript|zod|surrealql)"
        )),
    }
}
