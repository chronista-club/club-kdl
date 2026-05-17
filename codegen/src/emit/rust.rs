//! Rust emitter — renders [`ir::Schema`] into Rust source text.
//!
//! Ported from club-unison's `codegen/rust.rs`. The original used
//! `proc_macro2` + `quote` to build a `TokenStream` and a hand-rolled
//! `format_code` pass; this port writes pre-formatted Rust text directly so
//! `club-kdl-codegen` stays dependency-free during Phase 1.
//!
//! ## What it emits
//!
//! - data dialect: every [`ir::TypeDef`] — `struct` (with fields) and `enum`
//!   (string-valued variants).
//! - protocol dialect: for every [`ir::Channel`], a `struct` per request
//!   payload, per `returns` message, and per event payload.
//!
//! Each generated `struct` / `enum` carries `#[derive(...)]` attributes and
//! `serde` annotations matching club-unison's generator. Optional fields
//! become `Option<T>` with `#[serde(skip_serializing_if = "Option::is_none")]`.
//!
//! ## Differences from club-unison (IR-driven port)
//!
//! - The IR has no inline `_inline_*` messages, no `service` / `method` /
//!   `stream` / `send` / `recv` legacy constructs, and no field-level
//!   constraints / defaults — so the corresponding branches are dropped.
//! - The IR's [`ir::Prim::Datetime`] maps to `chrono::DateTime<Utc>`; named
//!   type references emit the bare identifier (no `TypeRegistry` indirection).

use crate::Emitter;
use crate::ir;

use super::case::{to_pascal_case, to_snake_case};

/// The Rust code generation target.
#[derive(Debug, Default, Clone, Copy)]
pub struct RustEmitter;

impl RustEmitter {
    /// Create a new [`RustEmitter`].
    pub fn new() -> Self {
        Self
    }
}

impl Emitter for RustEmitter {
    fn emit(&self, schema: &ir::Schema) -> String {
        let mut out = String::new();
        out.push_str(IMPORTS);

        // data dialect — standalone type definitions.
        for ty in &schema.types {
            out.push('\n');
            out.push_str(&render_typedef(ty));
        }

        // protocol dialect — channel payload structs.
        if let Some(protocol) = &schema.protocol {
            for channel in &protocol.channels {
                out.push_str(&render_channel(channel));
            }
        }

        out
    }
}

/// Header import block, matching club-unison's `generate_imports`.
const IMPORTS: &str = "\
use serde::{Deserialize, Serialize};
use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::collections::HashMap;
";

/// Render one standalone [`ir::TypeDef`].
fn render_typedef(ty: &ir::TypeDef) -> String {
    match ty {
        ir::TypeDef::Struct { name, fields } => render_struct(name, fields),
        ir::TypeDef::Enum { name, variants } => render_enum(name, variants),
    }
}

/// Render a `struct` from a name and field list. A fieldless struct becomes a
/// unit struct (`pub struct Name;`), matching club-unison.
fn render_struct(name: &str, fields: &[ir::Field]) -> String {
    let derive = "#[derive(Debug, Clone, Serialize, Deserialize)]\n";
    if fields.is_empty() {
        return format!("{derive}pub struct {name};\n");
    }
    let mut out = String::new();
    out.push_str(derive);
    out.push_str(&format!("pub struct {name} {{\n"));
    for field in fields {
        out.push_str(&render_field(field));
    }
    out.push_str("}\n");
    out
}

/// Render an `enum` of string-valued variants.
fn render_enum(name: &str, variants: &[String]) -> String {
    let mut out = String::new();
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]\n");
    out.push_str("#[serde(rename_all = \"snake_case\")]\n");
    out.push_str(&format!("pub enum {name} {{\n"));
    for v in variants {
        out.push_str(&format!("    #[serde(rename = \"{v}\")]\n"));
        out.push_str(&format!("    {},\n", to_pascal_case(v)));
    }
    out.push_str("}\n");
    out
}

/// Render a single struct field with its `serde` attributes.
fn render_field(field: &ir::Field) -> String {
    let mut out = String::new();

    // `#[serde(rename = "...")]` when the source name is not snake_case.
    let snake = to_snake_case(&field.name);
    if field.name != snake {
        out.push_str(&format!("    #[serde(rename = \"{}\")]\n", field.name));
    }

    let base = ty_to_rust(&field.ty);
    let rust_ty = if field.required {
        base
    } else {
        out.push_str("    #[serde(skip_serializing_if = \"Option::is_none\")]\n");
        format!("Option<{base}>")
    };

    out.push_str(&format!(
        "    pub {}: {rust_ty},\n",
        field_ident(&field.name)
    ));
    out
}

/// Render a field name as a valid Rust identifier. A name that collides with
/// a Rust keyword is escaped as a raw identifier (`r#type`) so the generated
/// source compiles. `serde` strips the `r#` prefix, so the wire name is
/// unaffected.
///
/// `crate` / `self` / `Self` / `super` cannot be raw identifiers; they are
/// left as-is (a KDL schema field is extremely unlikely to use them).
fn field_ident(name: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "dyn", "else", "enum", "extern", "false", "fn", "for",
        "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
        "static", "struct", "trait", "true", "type", "unsafe", "use", "where", "while", "async",
        "await", "gen", "abstract", "become", "box", "do", "final", "macro", "override", "priv",
        "try", "typeof", "unsized", "virtual", "yield",
    ];
    if KEYWORDS.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_string()
    }
}

/// Map an [`ir::Ty`] to its Rust type expression.
fn ty_to_rust(ty: &ir::Ty) -> String {
    match ty {
        ir::Ty::Primitive(p) => prim_to_rust(*p).to_string(),
        ir::Ty::Array(inner) => format!("Vec<{}>", ty_to_rust(inner)),
        ir::Ty::Named(name) => name.clone(),
    }
}

/// Map an [`ir::Prim`] to its Rust type.
fn prim_to_rust(p: ir::Prim) -> &'static str {
    match p {
        ir::Prim::String => "String",
        ir::Prim::Int => "i64",
        ir::Prim::Float => "f64",
        ir::Prim::Bool => "bool",
        ir::Prim::Datetime => "DateTime<Utc>",
        ir::Prim::Json => "serde_json::Value",
    }
}

/// Render every payload struct for one channel: request payloads, `returns`
/// messages, and event payloads.
fn render_channel(channel: &ir::Channel) -> String {
    let mut out = String::new();
    for req in &channel.requests {
        out.push('\n');
        out.push_str(&render_struct(&req.name, &req.fields));
        if let Some(returns) = &req.returns {
            out.push('\n');
            out.push_str(&render_struct(&returns.name, &returns.fields));
        }
    }
    for evt in &channel.events {
        out.push('\n');
        out.push_str(&render_struct(&evt.name, &evt.fields));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str, ty: ir::Ty, required: bool) -> ir::Field {
        ir::Field {
            name: name.to_string(),
            ty,
            required,
        }
    }

    #[test]
    fn emits_import_header() {
        let out = RustEmitter::new().emit(&ir::Schema::default());
        assert!(out.contains("use serde::{Deserialize, Serialize};"));
        assert!(out.contains("use chrono::{DateTime, Utc};"));
    }

    #[test]
    fn emits_struct_with_required_field() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "User".to_string(),
                fields: vec![field("name", ir::Ty::Primitive(ir::Prim::String), true)],
            }],
            protocol: None,
        };
        let out = RustEmitter::new().emit(&schema);
        assert!(out.contains("#[derive(Debug, Clone, Serialize, Deserialize)]"));
        assert!(out.contains("pub struct User {"));
        assert!(out.contains("    pub name: String,"));
    }

    #[test]
    fn keyword_field_name_is_raw_identifier() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "Node".to_string(),
                fields: vec![field("type", ir::Ty::Primitive(ir::Prim::String), true)],
            }],
            protocol: None,
        };
        let out = RustEmitter::new().emit(&schema);
        // `type` is a Rust keyword — it must be escaped as a raw identifier
        // so the generated source compiles.
        assert!(out.contains("pub r#type: String,"));
    }

    #[test]
    fn optional_field_becomes_option_with_skip() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "User".to_string(),
                fields: vec![field("nick", ir::Ty::Primitive(ir::Prim::String), false)],
            }],
            protocol: None,
        };
        let out = RustEmitter::new().emit(&schema);
        assert!(out.contains("#[serde(skip_serializing_if = \"Option::is_none\")]"));
        assert!(out.contains("pub nick: Option<String>,"));
    }

    #[test]
    fn non_snake_field_gets_serde_rename() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "User".to_string(),
                fields: vec![field(
                    "displayName",
                    ir::Ty::Primitive(ir::Prim::String),
                    true,
                )],
            }],
            protocol: None,
        };
        let out = RustEmitter::new().emit(&schema);
        assert!(out.contains("#[serde(rename = \"displayName\")]"));
        assert!(out.contains("pub displayName: String,"));
    }

    #[test]
    fn fieldless_struct_is_unit() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "Empty".to_string(),
                fields: vec![],
            }],
            protocol: None,
        };
        let out = RustEmitter::new().emit(&schema);
        assert!(out.contains("pub struct Empty;"));
    }

    #[test]
    fn emits_enum_with_rename() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Enum {
                name: "Role".to_string(),
                variants: vec!["admin".to_string(), "guest_user".to_string()],
            }],
            protocol: None,
        };
        let out = RustEmitter::new().emit(&schema);
        assert!(out.contains("#[serde(rename_all = \"snake_case\")]"));
        assert!(out.contains("pub enum Role {"));
        assert!(out.contains("#[serde(rename = \"admin\")]"));
        assert!(out.contains("    Admin,"));
        assert!(out.contains("#[serde(rename = \"guest_user\")]"));
        assert!(out.contains("    GuestUser,"));
    }

    #[test]
    fn maps_primitive_and_compound_types() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "T".to_string(),
                fields: vec![
                    field("n", ir::Ty::Primitive(ir::Prim::Int), true),
                    field("f", ir::Ty::Primitive(ir::Prim::Float), true),
                    field("b", ir::Ty::Primitive(ir::Prim::Bool), true),
                    field("at", ir::Ty::Primitive(ir::Prim::Datetime), true),
                    field("blob", ir::Ty::Primitive(ir::Prim::Json), true),
                    field(
                        "tags",
                        ir::Ty::Array(Box::new(ir::Ty::Primitive(ir::Prim::String))),
                        true,
                    ),
                    field("owner", ir::Ty::Named("User".to_string()), true),
                ],
            }],
            protocol: None,
        };
        let out = RustEmitter::new().emit(&schema);
        assert!(out.contains("pub n: i64,"));
        assert!(out.contains("pub f: f64,"));
        assert!(out.contains("pub b: bool,"));
        assert!(out.contains("pub at: DateTime<Utc>,"));
        assert!(out.contains("pub blob: serde_json::Value,"));
        assert!(out.contains("pub tags: Vec<String>,"));
        assert!(out.contains("pub owner: User,"));
    }

    #[test]
    fn emits_channel_request_returns_and_event_structs() {
        let schema = ir::Schema {
            types: vec![],
            protocol: Some(ir::Protocol {
                name: "ping-pong".to_string(),
                version: "2.0.0".to_string(),
                namespace: None,
                description: None,
                channels: vec![ir::Channel {
                    name: "ping-pong".to_string(),
                    from: ir::ChannelFrom::Client,
                    lifetime: ir::ChannelLifetime::Persistent,
                    backend: ir::ChannelBackend::Stream,
                    channel_id: None,
                    requests: vec![ir::Request {
                        name: "Ping".to_string(),
                        fields: vec![field("seq", ir::Ty::Primitive(ir::Prim::Int), true)],
                        returns: Some(ir::Message {
                            name: "Pong".to_string(),
                            fields: vec![field("seq", ir::Ty::Primitive(ir::Prim::Int), true)],
                        }),
                    }],
                    events: vec![ir::Event {
                        name: "Tick".to_string(),
                        fields: vec![],
                    }],
                }],
            }),
        };
        let out = RustEmitter::new().emit(&schema);
        assert!(out.contains("pub struct Ping {"));
        assert!(out.contains("pub struct Pong {"));
        assert!(out.contains("pub struct Tick;"));
    }
}
