//! SurrealQL emitter — renders [`ir::Schema`] into SurrealDB schema DDL.
//!
//! Unlike the Rust / TypeScript / Zod emitters (which produce application-layer
//! types and validators), this emitter produces **database schema** —
//! `DEFINE TABLE` / `DEFINE FIELD` statements. The protocol dialect
//! (`channel` / `request` / `event`) has no database representation, so this
//! emitter consumes the **data and entity dialects only**; a protocol-only
//! schema yields just the header.
//!
//! ## Type mapping decisions (派生 todo `mem_1Cb5kAE5aAqYimBRfBnzVj`)
//!
//! - **struct**: an embedded value type — a `DEFINE TABLE ... SCHEMAFULL`
//!   with no explicit `TYPE`. A struct reference is rendered as an embedded
//!   `object` is *not* used; instead a named struct becomes a `record<table>`
//!   link, matching the prior behaviour.
//! - **record**: a first-class entity — `DEFINE TABLE <t> TYPE NORMAL
//!   SCHEMAFULL`. Each `record` is one table; its `id` is the table id.
//! - **relation**: a graph edge — `DEFINE TABLE <t> TYPE RELATION IN <from>
//!   OUT <to> SCHEMAFULL`. A `unique=#true` relation also emits a
//!   `DEFINE INDEX ... UNIQUE` on `(in, out)`.
//! - **enum**: SurrealDB has no enum type. An enum-typed field becomes
//!   `string` with an `ASSERT $value IN [...]` clause listing the variants.
//! - **`link<Record>`**: a `record<table>` link.
//! - **literal / literal union**: `'a' | 'b'` becomes `string` with an
//!   `ASSERT $value IN ['a', 'b']` clause.
//! - **`object` + `flexible=#true`**: rendered as `FLEXIBLE TYPE object`
//!   (schemaless nested object).
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
        // data dialect — `struct` tables (embedded value types).
        for ty in &schema.types {
            if let ir::TypeDef::Struct { name, fields } = ty {
                code.push('\n');
                code.push_str(&render_table(name, fields, TableKind::Struct, &enums));
            }
        }
        // entity dialect — `record` tables.
        for record in &schema.records {
            code.push('\n');
            code.push_str(&render_table(
                &record.name,
                &record.fields,
                TableKind::Record,
                &enums,
            ));
        }
        // entity dialect — `relation` (edge) tables.
        for relation in &schema.relations {
            code.push('\n');
            code.push_str(&render_relation(relation, &enums));
        }
        code
    }
}

/// Which flavour of `DEFINE TABLE` to emit.
#[derive(Clone, Copy)]
enum TableKind {
    /// A `struct` — an embedded value type. No explicit `TYPE` clause, to
    /// keep the prior behaviour byte-stable.
    Struct,
    /// A `record` — a first-class entity. `TYPE NORMAL`.
    Record,
}

/// Fixed header block.
const HEADER: &str = "\
-- Auto-generated SurrealDB schema
-- DO NOT EDIT MANUALLY
";

/// Render one `struct` / `record` as a `DEFINE TABLE` plus its `DEFINE FIELD`s.
fn render_table(
    name: &str,
    fields: &[ir::Field],
    kind: TableKind,
    enums: &HashMap<&str, &[String]>,
) -> String {
    let table = to_snake_case(name);
    let type_clause = match kind {
        TableKind::Struct => "",
        TableKind::Record => "TYPE NORMAL ",
    };
    let mut out = format!("DEFINE TABLE {table} {type_clause}SCHEMAFULL;\n");
    for field in fields {
        out.push_str(&render_field(&table, field, enums));
    }
    out
}

/// Render one `relation` as a `DEFINE TABLE ... TYPE RELATION` plus its edge
/// `DEFINE FIELD`s and, when `unique`, a `DEFINE INDEX ... UNIQUE` on
/// `(in, out)`.
fn render_relation(relation: &ir::Relation, enums: &HashMap<&str, &[String]>) -> String {
    let table = to_snake_case(&relation.name);
    let in_t = to_snake_case(&relation.from);
    let out_t = to_snake_case(&relation.to);
    let mut out = format!("DEFINE TABLE {table} TYPE RELATION IN {in_t} OUT {out_t} SCHEMAFULL;\n");
    for field in &relation.fields {
        out.push_str(&render_field(&table, field, enums));
    }
    if relation.unique {
        out.push_str(&format!(
            "DEFINE INDEX {table}_unique_edge ON {table} FIELDS in, out UNIQUE;\n"
        ));
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
    // `flexible=#true` on an `object` field → schemaless nested object.
    let flexible = if field.flexible && is_object_ty(&field.ty) {
        "FLEXIBLE "
    } else {
        ""
    };
    let mut line = format!(
        "DEFINE FIELD {} ON {table} {flexible}TYPE {full}",
        field.name
    );
    if let Some(clause) = assert {
        line.push(' ');
        line.push_str(&clause);
    }
    if let Some(default) = &field.default {
        line.push_str(&format!(" DEFAULT {}", surql_default(&field.ty, default)));
    }
    line.push_str(";\n");
    line
}

/// Whether a type is the `object` primitive (so `flexible=` applies).
fn is_object_ty(ty: &ir::Ty) -> bool {
    matches!(ty, ir::Ty::Primitive(ir::Prim::Json))
}

/// Render a field default for SurrealQL. String-ish types are single-quoted;
/// numeric / boolean defaults are passed through verbatim.
fn surql_default(ty: &ir::Ty, raw: &str) -> String {
    let quote = matches!(
        ty,
        ir::Ty::Primitive(ir::Prim::String)
            | ir::Ty::Primitive(ir::Prim::Datetime)
            | ir::Ty::Literal(_)
            | ir::Ty::Named(_)
    ) || matches!(ty, ir::Ty::Union(members)
        if members.iter().all(|m| matches!(m, ir::Ty::Literal(_))));
    if quote {
        format!("'{raw}'")
    } else {
        raw.to_string()
    }
}

/// Map an [`ir::Ty`] to a SurrealQL type, plus an optional `ASSERT` clause
/// (used to constrain enum-typed and literal-union fields to their value set).
fn ty_to_surql(ty: &ir::Ty, enums: &HashMap<&str, &[String]>) -> (String, Option<String>) {
    match ty {
        ir::Ty::Primitive(p) => (prim_to_surql(*p).to_string(), None),
        ir::Ty::Array(inner) => {
            let (inner_ty, _) = ty_to_surql(inner, enums);
            (format!("array<{inner_ty}>"), None)
        }
        ir::Ty::Named(name) => match enums.get(name.as_str()) {
            // enum → string constrained by an ASSERT clause.
            Some(variants) => (
                "string".to_string(),
                Some(assert_in(variants.iter().map(String::as_str))),
            ),
            // struct → a record link to that struct's table.
            None => (format!("record<{}>", to_snake_case(name)), None),
        },
        // a `link<Record>` → a SurrealDB record link.
        ir::Ty::Link(name) => (format!("record<{}>", to_snake_case(name)), None),
        // a bare literal → string constrained to that single value.
        ir::Ty::Literal(value) => (
            "string".to_string(),
            Some(assert_in(std::iter::once(value.as_str()))),
        ),
        ir::Ty::Union(members) => {
            // A union of string literals → `string` with an `ASSERT IN [...]`.
            if let Some(values) = literal_union_values(members) {
                (
                    "string".to_string(),
                    Some(assert_in(values.iter().map(String::as_str))),
                )
            } else {
                // A non-literal union → a SurrealDB union type
                // (`record<x> | string` etc.). Members mapping to the same
                // type are de-duplicated; per-member `ASSERT`s are dropped
                // (a mixed union has no single value set).
                let mut parts: Vec<String> = Vec::new();
                for m in members {
                    let (t, _) = ty_to_surql(m, enums);
                    if !parts.contains(&t) {
                        parts.push(t);
                    }
                }
                (parts.join(" | "), None)
            }
        }
    }
}

/// If every union member is a [`ir::Ty::Literal`], return their values.
fn literal_union_values(members: &[ir::Ty]) -> Option<Vec<String>> {
    members
        .iter()
        .map(|m| match m {
            ir::Ty::Literal(v) => Some(v.clone()),
            _ => None,
        })
        .collect()
}

/// Build an `ASSERT $value IN ['a', 'b', ...]` clause.
fn assert_in<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let list: Vec<String> = values.map(|v| format!("'{v}'")).collect();
    format!("ASSERT $value IN [{}]", list.join(", "))
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
            flexible: false,
            default: None,
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            records: vec![],
            relations: vec![],
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

    // -------------------------------------------------------------------------
    // Tier 1 — record / relation / link / union / flexible / default
    // -------------------------------------------------------------------------

    #[test]
    fn record_becomes_define_table_type_normal() {
        let schema = ir::Schema {
            records: vec![ir::Record {
                name: "Atlas".to_string(),
                id_strategy: ir::IdStrategy::Uuidv7,
                fields: vec![field("name", ir::Ty::Primitive(ir::Prim::String), true)],
            }],
            ..Default::default()
        };
        let out = SurrealQlEmitter::new().emit(&schema);
        assert!(out.contains("DEFINE TABLE atlas TYPE NORMAL SCHEMAFULL;"));
        assert!(out.contains("DEFINE FIELD name ON atlas TYPE string;"));
    }

    #[test]
    fn struct_table_keeps_no_type_clause() {
        // A `struct` must stay byte-stable with the pre-Tier-1 output:
        // `DEFINE TABLE <t> SCHEMAFULL;` with no `TYPE`.
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "GeoPoint".to_string(),
                fields: vec![field("lat", ir::Ty::Primitive(ir::Prim::Float), true)],
            }],
            ..Default::default()
        };
        let out = SurrealQlEmitter::new().emit(&schema);
        assert!(out.contains("DEFINE TABLE geo_point SCHEMAFULL;"));
        assert!(!out.contains("TYPE NORMAL"));
    }

    #[test]
    fn relation_becomes_define_table_type_relation_with_index() {
        let schema = ir::Schema {
            relations: vec![ir::Relation {
                name: "derivedFrom".to_string(),
                from: "Memory".to_string(),
                to: "Memory".to_string(),
                unique: true,
                fields: vec![field(
                    "confidence",
                    ir::Ty::Primitive(ir::Prim::Float),
                    false,
                )],
            }],
            ..Default::default()
        };
        let out = SurrealQlEmitter::new().emit(&schema);
        assert!(
            out.contains(
                "DEFINE TABLE derived_from TYPE RELATION IN memory OUT memory SCHEMAFULL;"
            )
        );
        assert!(out.contains("DEFINE FIELD confidence ON derived_from TYPE option<float>;"));
        assert!(out.contains(
            "DEFINE INDEX derived_from_unique_edge ON derived_from FIELDS in, out UNIQUE;"
        ));
    }

    #[test]
    fn non_unique_relation_omits_index() {
        let schema = ir::Schema {
            relations: vec![ir::Relation {
                name: "tagged".to_string(),
                from: "Note".to_string(),
                to: "Tag".to_string(),
                unique: false,
                fields: vec![],
            }],
            ..Default::default()
        };
        let out = SurrealQlEmitter::new().emit(&schema);
        assert!(out.contains("TYPE RELATION IN note OUT tag"));
        assert!(!out.contains("DEFINE INDEX"));
    }

    #[test]
    fn link_field_becomes_record_link() {
        let schema = ir::Schema {
            records: vec![ir::Record {
                name: "Atlas".to_string(),
                id_strategy: ir::IdStrategy::Uuidv7,
                fields: vec![field("parent", ir::Ty::Link("Atlas".to_string()), false)],
            }],
            ..Default::default()
        };
        let out = SurrealQlEmitter::new().emit(&schema);
        assert!(out.contains("DEFINE FIELD parent ON atlas TYPE option<record<atlas>>;"));
    }

    #[test]
    fn literal_union_becomes_string_with_assert() {
        let schema = ir::Schema {
            records: vec![ir::Record {
                name: "Doc".to_string(),
                id_strategy: ir::IdStrategy::Uuidv7,
                fields: vec![field(
                    "visibility",
                    ir::Ty::Union(vec![
                        ir::Ty::Literal("public".to_string()),
                        ir::Ty::Literal("private".to_string()),
                    ]),
                    true,
                )],
            }],
            ..Default::default()
        };
        let out = SurrealQlEmitter::new().emit(&schema);
        assert!(out.contains(
            "DEFINE FIELD visibility ON doc TYPE string ASSERT $value IN ['public', 'private'];"
        ));
    }

    #[test]
    fn flexible_object_field_emits_flexible_keyword() {
        let mut f = field("metadata", ir::Ty::Primitive(ir::Prim::Json), true);
        f.flexible = true;
        let schema = ir::Schema {
            records: vec![ir::Record {
                name: "Atlas".to_string(),
                id_strategy: ir::IdStrategy::Uuidv7,
                fields: vec![f],
            }],
            ..Default::default()
        };
        let out = SurrealQlEmitter::new().emit(&schema);
        assert!(out.contains("DEFINE FIELD metadata ON atlas FLEXIBLE TYPE object;"));
    }

    #[test]
    fn default_value_is_quoted_for_string_types() {
        let mut f = field("visibility", ir::Ty::Primitive(ir::Prim::String), true);
        f.default = Some("private".to_string());
        let mut g = field("count", ir::Ty::Primitive(ir::Prim::Int), true);
        g.default = Some("0".to_string());
        let schema = ir::Schema {
            records: vec![ir::Record {
                name: "Doc".to_string(),
                id_strategy: ir::IdStrategy::Uuidv7,
                fields: vec![f, g],
            }],
            ..Default::default()
        };
        let out = SurrealQlEmitter::new().emit(&schema);
        assert!(out.contains("DEFAULT 'private'"), "string default quoted");
        assert!(out.contains("DEFAULT 0"), "numeric default unquoted");
    }
}
