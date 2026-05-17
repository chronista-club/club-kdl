//! SurrealQL emitter — renders [`ir::Schema`] into SurrealDB schema DDL.
//!
//! Unlike the Rust / TypeScript / Zod emitters (which produce application-layer
//! types and validators), this emitter produces **database schema** —
//! `DEFINE TABLE` / `DEFINE FIELD` statements. The protocol dialect
//! (`channel` / `request` / `event`) has no database representation, so this
//! emitter consumes the **data dialect only**; a protocol-only schema yields
//! just the header.
//!
//! ## Type mapping decisions (派生 todo `mem_1Cb5kAE5aAqYimBRfBnzVj`)
//!
//! - **enum**: SurrealDB has no enum type. An enum-typed field becomes
//!   `string` with an `ASSERT $value IN [...]` clause listing the variants.
//! - **struct reference**: a named struct becomes a `record<table>` link
//!   (each `struct` is one `DEFINE TABLE`).
//! - **optional**: a non-required field is wrapped in `option<T>`.

use std::collections::HashMap;

use crate::Emitter;
use crate::ir;

use super::case::to_snake_case;

/// The SurrealQL code generation target.
#[derive(Debug, Default, Clone, Copy)]
pub struct SurrealQlEmitter;

impl SurrealQlEmitter {
    /// Create a new [`SurrealQlEmitter`].
    pub fn new() -> Self {
        Self
    }
}

impl Emitter for SurrealQlEmitter {
    fn emit(&self, schema: &ir::Schema) -> String {
        // Map enum name → variants, for `ASSERT $value IN [...]` rendering.
        let enums: HashMap<&str, &[String]> = schema
            .types
            .iter()
            .filter_map(|t| match t {
                ir::TypeDef::Enum { name, variants } => Some((name.as_str(), variants.as_slice())),
                ir::TypeDef::Struct { .. } => None,
            })
            .collect();

        let mut code = String::from(HEADER);
        for ty in &schema.types {
            if let ir::TypeDef::Struct { name, fields } = ty {
                code.push('\n');
                code.push_str(&render_table(name, fields, &enums));
            }
        }
        code
    }
}

/// Fixed header block.
const HEADER: &str = "\
-- Auto-generated SurrealDB schema
-- DO NOT EDIT MANUALLY
";

/// Render one `struct` as a `DEFINE TABLE` plus its `DEFINE FIELD`s.
fn render_table(name: &str, fields: &[ir::Field], enums: &HashMap<&str, &[String]>) -> String {
    let table = to_snake_case(name);
    let mut out = format!("DEFINE TABLE {table} SCHEMAFULL;\n");
    for field in fields {
        out.push_str(&render_field(&table, field, enums));
    }
    out
}

/// Render one `DEFINE FIELD` statement.
fn render_field(table: &str, field: &ir::Field, enums: &HashMap<&str, &[String]>) -> String {
    let (base, assert) = ty_to_surql(&field.ty, enums);
    let full = if field.required {
        base
    } else {
        format!("option<{base}>")
    };
    let mut line = format!("DEFINE FIELD {} ON {table} TYPE {full}", field.name);
    if let Some(clause) = assert {
        line.push(' ');
        line.push_str(&clause);
    }
    line.push_str(";\n");
    line
}

/// Map an [`ir::Ty`] to a SurrealQL type, plus an optional `ASSERT` clause
/// (used to constrain enum-typed fields to their variant set).
fn ty_to_surql(ty: &ir::Ty, enums: &HashMap<&str, &[String]>) -> (String, Option<String>) {
    match ty {
        ir::Ty::Primitive(p) => (prim_to_surql(*p).to_string(), None),
        ir::Ty::Array(inner) => {
            let (inner_ty, _) = ty_to_surql(inner, enums);
            (format!("array<{inner_ty}>"), None)
        }
        ir::Ty::Named(name) => match enums.get(name.as_str()) {
            // enum → string constrained by an ASSERT clause.
            Some(variants) => {
                let list: Vec<String> = variants.iter().map(|v| format!("'{v}'")).collect();
                (
                    "string".to_string(),
                    Some(format!("ASSERT $value IN [{}]", list.join(", "))),
                )
            }
            // struct → a record link to that struct's table.
            None => (format!("record<{}>", to_snake_case(name)), None),
        },
    }
}

/// Map an [`ir::Prim`] to its SurrealQL type.
fn prim_to_surql(p: ir::Prim) -> &'static str {
    match p {
        ir::Prim::String => "string",
        ir::Prim::Int => "int",
        ir::Prim::Float => "float",
        ir::Prim::Bool => "bool",
        ir::Prim::Datetime => "datetime",
        ir::Prim::Json => "object",
    }
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
    fn emits_header() {
        let out = SurrealQlEmitter::new().emit(&ir::Schema::default());
        assert!(out.contains("-- Auto-generated SurrealDB schema"));
    }

    #[test]
    fn struct_becomes_define_table_and_fields() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "User".to_string(),
                fields: vec![
                    field("id", ir::Ty::Primitive(ir::Prim::String), true),
                    field("age", ir::Ty::Primitive(ir::Prim::Int), true),
                ],
            }],
            protocol: None,
        };
        let out = SurrealQlEmitter::new().emit(&schema);
        assert!(out.contains("DEFINE TABLE user SCHEMAFULL;"));
        assert!(out.contains("DEFINE FIELD id ON user TYPE string;"));
        assert!(out.contains("DEFINE FIELD age ON user TYPE int;"));
    }

    #[test]
    fn optional_field_becomes_option_type() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "User".to_string(),
                fields: vec![field("nick", ir::Ty::Primitive(ir::Prim::String), false)],
            }],
            protocol: None,
        };
        let out = SurrealQlEmitter::new().emit(&schema);
        assert!(out.contains("DEFINE FIELD nick ON user TYPE option<string>;"));
    }

    #[test]
    fn enum_reference_becomes_string_with_assert() {
        let schema = ir::Schema {
            types: vec![
                ir::TypeDef::Struct {
                    name: "User".to_string(),
                    fields: vec![field("role", ir::Ty::Named("Role".to_string()), true)],
                },
                ir::TypeDef::Enum {
                    name: "Role".to_string(),
                    variants: vec!["admin".to_string(), "member".to_string()],
                },
            ],
            protocol: None,
        };
        let out = SurrealQlEmitter::new().emit(&schema);
        assert!(out.contains(
            "DEFINE FIELD role ON user TYPE string ASSERT $value IN ['admin', 'member'];"
        ));
    }

    #[test]
    fn struct_reference_becomes_record_link() {
        let schema = ir::Schema {
            types: vec![
                ir::TypeDef::Struct {
                    name: "Post".to_string(),
                    fields: vec![field("author", ir::Ty::Named("User".to_string()), true)],
                },
                ir::TypeDef::Struct {
                    name: "User".to_string(),
                    fields: vec![field("id", ir::Ty::Primitive(ir::Prim::String), true)],
                },
            ],
            protocol: None,
        };
        let out = SurrealQlEmitter::new().emit(&schema);
        assert!(out.contains("DEFINE FIELD author ON post TYPE record<user>;"));
    }

    #[test]
    fn array_and_primitive_mapping() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "T".to_string(),
                fields: vec![
                    field("f", ir::Ty::Primitive(ir::Prim::Float), true),
                    field("b", ir::Ty::Primitive(ir::Prim::Bool), true),
                    field("at", ir::Ty::Primitive(ir::Prim::Datetime), true),
                    field("blob", ir::Ty::Primitive(ir::Prim::Json), true),
                    field(
                        "tags",
                        ir::Ty::Array(Box::new(ir::Ty::Primitive(ir::Prim::String))),
                        true,
                    ),
                ],
            }],
            protocol: None,
        };
        let out = SurrealQlEmitter::new().emit(&schema);
        assert!(out.contains("TYPE float;"));
        assert!(out.contains("TYPE bool;"));
        assert!(out.contains("TYPE datetime;"));
        assert!(out.contains("TYPE object;"));
        assert!(out.contains("TYPE array<string>;"));
    }

    #[test]
    fn protocol_only_schema_yields_header_only() {
        // The protocol dialect has no DB representation — a protocol-only
        // schema produces just the header, no DEFINE statements.
        let schema = ir::Schema {
            types: vec![],
            protocol: Some(ir::Protocol {
                name: "p".to_string(),
                version: "1.0.0".to_string(),
                namespace: None,
                description: None,
                channels: vec![],
            }),
        };
        let out = SurrealQlEmitter::new().emit(&schema);
        assert!(out.contains("-- Auto-generated SurrealDB schema"));
        assert!(!out.contains("DEFINE TABLE"));
    }
}
