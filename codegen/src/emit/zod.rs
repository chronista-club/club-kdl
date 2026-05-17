//! Zod emitter — renders [`ir::Schema`] into Zod schema source (TypeScript).
//!
//! Zod schemas are runtime validators. The generated `export const` values
//! mirror the data dialect's `struct` / `enum`, the entity dialect's
//! `record` / `relation`, and the protocol dialect's request / response /
//! event payloads.
//!
//! ## Tier 1 type mapping
//!
//! - `link<Record>` → `z.string()` (the linked record's id).
//! - `'literal'` → `z.literal("value")`.
//! - `A | B` → `z.union([...])`; a union of string literals collapses to a
//!   `z.enum([...])` (Zod's idiomatic closed-string-set validator).
//! - `record` → a `z.object({...})` with a leading `id: z.string()`.
//! - `relation` → a `z.object({...})` with `id` / `in` / `out: z.string()`.
//!
//! ## Ordering
//!
//! Unlike the TypeScript emitter (whose `interface` declarations are types and
//! thus order-independent), a Zod schema is a **value** — `z.object({ role:
//! Role })` needs `Role` defined first. The emitter therefore writes all
//! `enum` schemas before any `object` schema. Struct-to-struct references rely
//! on source order (a forward reference between two structs is uncommon and
//! out of Phase 1 scope).

use crate::Emitter;
use crate::ir;

use super::case::to_pascal_case;

/// The Zod code generation target.
#[derive(Debug, Default, Clone, Copy)]
pub struct ZodEmitter;

impl ZodEmitter {
    /// Create a new [`ZodEmitter`].
    pub fn new() -> Self {
        Self
    }
}

impl Emitter for ZodEmitter {
    fn emit(&self, schema: &ir::Schema) -> String {
        let mut code = String::new();
        code.push_str(HEADER);
        code.push('\n');

        // enums first — a Zod schema is a value and cannot be forward-referenced.
        for ty in &schema.types {
            if let ir::TypeDef::Enum { name, variants } = ty {
                code.push_str(&render_enum(name, variants));
                code.push_str("\n\n");
            }
        }
        // then structs.
        for ty in &schema.types {
            if let ir::TypeDef::Struct { name, fields } = ty {
                code.push_str(&render_object(name, fields));
                code.push_str("\n\n");
            }
        }

        // entity dialect — records and relations.
        for record in &schema.records {
            code.push_str(&render_object(&record.name, &record_members(record)));
            code.push_str("\n\n");
        }
        for relation in &schema.relations {
            code.push_str(&render_object(&relation.name, &relation_members(relation)));
            code.push_str("\n\n");
        }

        // protocol dialect — request / response / event payload schemas.
        if let Some(protocol) = &schema.protocol {
            for channel in &protocol.channels {
                for req in &channel.requests {
                    code.push_str(&render_object(&req.name, &req.fields));
                    code.push_str("\n\n");
                    if let Some(returns) = &req.returns {
                        code.push_str(&render_object(&returns.name, &returns.fields));
                        code.push_str("\n\n");
                    }
                }
                for evt in &channel.events {
                    code.push_str(&render_object(&evt.name, &evt.fields));
                    code.push_str("\n\n");
                }
            }
        }

        code
    }
}

/// Fixed header block.
const HEADER: &str = "\
// Auto-generated Zod schemas
// DO NOT EDIT MANUALLY
import { z } from \"zod\";
";

/// Render an `enum` as a `z.enum([...])`.
fn render_enum(name: &str, variants: &[String]) -> String {
    let vs: Vec<String> = variants.iter().map(|v| format!("\"{v}\"")).collect();
    format!(
        "export const {} = z.enum([{}]);",
        to_pascal_case(name),
        vs.join(", ")
    )
}

/// Render a `struct` / payload as a `z.object({...})`.
fn render_object(name: &str, fields: &[ir::Field]) -> String {
    let pascal = to_pascal_case(name);
    if fields.is_empty() {
        return format!("export const {pascal} = z.object({{}});");
    }
    let body: Vec<String> = fields.iter().map(render_field).collect();
    format!(
        "export const {pascal} = z.object({{\n{}\n}});",
        body.join("\n")
    )
}

/// Render a single object field line.
fn render_field(field: &ir::Field) -> String {
    let base = ty_to_zod(&field.ty);
    let schema = if field.required {
        base
    } else {
        format!("{base}.optional()")
    };
    format!("  {}: {},", field.name, schema)
}

/// The synthetic `id: z.string()` member shared by records and relations.
fn id_member() -> ir::Field {
    ir::Field {
        name: "id".to_string(),
        ty: ir::Ty::Primitive(ir::Prim::String),
        required: true,
        flexible: false,
        default: None,
    }
}

/// A record's object members: a leading `id`, then its declared fields.
fn record_members(record: &ir::Record) -> Vec<ir::Field> {
    let mut members = Vec::with_capacity(record.fields.len() + 1);
    members.push(id_member());
    members.extend(record.fields.iter().cloned());
    members
}

/// A relation's edge-object members: `id` / `in` / `out`, then its declared
/// edge-property fields.
fn relation_members(relation: &ir::Relation) -> Vec<ir::Field> {
    let endpoint = |name: &str| ir::Field {
        name: name.to_string(),
        ty: ir::Ty::Primitive(ir::Prim::String),
        required: true,
        flexible: false,
        default: None,
    };
    let mut members = Vec::with_capacity(relation.fields.len() + 3);
    members.push(id_member());
    members.push(endpoint("in"));
    members.push(endpoint("out"));
    members.extend(relation.fields.iter().cloned());
    members
}

/// Map an [`ir::Ty`] to its Zod schema expression.
fn ty_to_zod(ty: &ir::Ty) -> String {
    match ty {
        ir::Ty::Primitive(p) => prim_to_zod(*p).to_string(),
        ir::Ty::Array(inner) => format!("z.array({})", ty_to_zod(inner)),
        // a named type references another generated schema by identifier.
        ir::Ty::Named(name) => to_pascal_case(name),
        // a link is validated as the target record's id — a string.
        ir::Ty::Link(_) => "z.string()".to_string(),
        // a string literal → `z.literal(...)`.
        ir::Ty::Literal(value) => format!("z.literal(\"{value}\")"),
        ir::Ty::Union(members) => {
            // A union of string literals → the idiomatic `z.enum([...])`.
            if let Some(values) = literal_union_values(members) {
                let vs: Vec<String> = values.iter().map(|v| format!("\"{v}\"")).collect();
                format!("z.enum([{}])", vs.join(", "))
            } else {
                let mut parts: Vec<String> = Vec::new();
                for m in members {
                    let p = ty_to_zod(m);
                    if !parts.contains(&p) {
                        parts.push(p);
                    }
                }
                // members collapsing to one schema need no `z.union` wrapper.
                if parts.len() == 1 {
                    parts.into_iter().next().unwrap()
                } else {
                    format!("z.union([{}])", parts.join(", "))
                }
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

/// Map an [`ir::Prim`] to its Zod schema expression.
fn prim_to_zod(p: ir::Prim) -> &'static str {
    match p {
        ir::Prim::String => "z.string()",
        ir::Prim::Int => "z.number().int()",
        ir::Prim::Float => "z.number()",
        ir::Prim::Bool => "z.boolean()",
        ir::Prim::Datetime => "z.string().datetime()",
        ir::Prim::Json => "z.unknown()",
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
        let out = ZodEmitter::new().emit(&ir::Schema::default());
        assert!(out.contains("import { z } from \"zod\";"));
        assert!(out.contains("// DO NOT EDIT MANUALLY"));
    }

    #[test]
    fn emits_enum_as_z_enum() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Enum {
                name: "Role".to_string(),
                variants: vec!["admin".to_string(), "member".to_string()],
            }],
            protocol: None,
            ..Default::default()
        };
        let out = ZodEmitter::new().emit(&schema);
        assert!(out.contains("export const Role = z.enum([\"admin\", \"member\"]);"));
    }

    #[test]
    fn emits_object_with_optional_field() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "User".to_string(),
                fields: vec![
                    field("name", ir::Ty::Primitive(ir::Prim::String), true),
                    field("nick", ir::Ty::Primitive(ir::Prim::String), false),
                ],
            }],
            protocol: None,
            ..Default::default()
        };
        let out = ZodEmitter::new().emit(&schema);
        assert!(out.contains("export const User = z.object({"));
        assert!(out.contains("  name: z.string(),"));
        assert!(out.contains("  nick: z.string().optional(),"));
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
                ],
            }],
            protocol: None,
            ..Default::default()
        };
        let out = ZodEmitter::new().emit(&schema);
        assert!(out.contains("  n: z.number().int(),"));
        assert!(out.contains("  f: z.number(),"));
        assert!(out.contains("  b: z.boolean(),"));
        assert!(out.contains("  at: z.string().datetime(),"));
        assert!(out.contains("  blob: z.unknown(),"));
        assert!(out.contains("  tags: z.array(z.string()),"));
    }

    #[test]
    fn enum_is_emitted_before_referencing_struct() {
        // `User` references `Role`; the schema lists the struct *first*.
        // The emitter must still place the enum before the object so the
        // generated Zod value resolves.
        let schema = ir::Schema {
            types: vec![
                ir::TypeDef::Struct {
                    name: "User".to_string(),
                    fields: vec![field("role", ir::Ty::Named("Role".to_string()), true)],
                },
                ir::TypeDef::Enum {
                    name: "Role".to_string(),
                    variants: vec!["admin".to_string()],
                },
            ],
            protocol: None,
            ..Default::default()
        };
        let out = ZodEmitter::new().emit(&schema);
        let enum_pos = out.find("export const Role").expect("enum emitted");
        let struct_pos = out.find("export const User").expect("struct emitted");
        assert!(
            enum_pos < struct_pos,
            "enum must precede the struct using it"
        );
        assert!(out.contains("  role: Role,"));
    }

    #[test]
    fn emits_protocol_payload_schemas() {
        let schema = ir::Schema {
            types: vec![],
            records: vec![],
            relations: vec![],
            protocol: Some(ir::Protocol {
                name: "chat".to_string(),
                version: "1.0.0".to_string(),
                namespace: None,
                description: None,
                channels: vec![ir::Channel {
                    name: "messaging".to_string(),
                    from: ir::ChannelFrom::Client,
                    lifetime: ir::ChannelLifetime::Persistent,
                    backend: ir::ChannelBackend::Stream,
                    channel_id: None,
                    requests: vec![ir::Request {
                        name: "Send".to_string(),
                        fields: vec![field("body", ir::Ty::Primitive(ir::Prim::String), true)],
                        returns: Some(ir::Message {
                            name: "Ack".to_string(),
                            fields: vec![field("id", ir::Ty::Primitive(ir::Prim::String), true)],
                        }),
                    }],
                    events: vec![],
                }],
            }),
        };
        let out = ZodEmitter::new().emit(&schema);
        assert!(out.contains("export const Send = z.object({"));
        assert!(out.contains("export const Ack = z.object({"));
    }

    // -------------------------------------------------------------------------
    // Tier 1 — record / relation / link / literal / union
    // -------------------------------------------------------------------------

    #[test]
    fn record_becomes_object_with_id() {
        let schema = ir::Schema {
            records: vec![ir::Record {
                name: "Atlas".to_string(),
                id_strategy: ir::IdStrategy::Uuidv7,
                fields: vec![field("name", ir::Ty::Primitive(ir::Prim::String), true)],
            }],
            ..Default::default()
        };
        let out = ZodEmitter::new().emit(&schema);
        assert!(out.contains("export const Atlas = z.object({"));
        assert!(out.contains("  id: z.string(),"));
        assert!(out.contains("  name: z.string(),"));
    }

    #[test]
    fn relation_object_is_pascal_cased_with_in_out() {
        let schema = ir::Schema {
            relations: vec![ir::Relation {
                name: "derivedFrom".to_string(),
                from: "Memory".to_string(),
                to: "Memory".to_string(),
                unique: false,
                fields: vec![field("reason", ir::Ty::Primitive(ir::Prim::String), false)],
            }],
            ..Default::default()
        };
        let out = ZodEmitter::new().emit(&schema);
        assert!(out.contains("export const DerivedFrom = z.object({"));
        assert!(out.contains("  id: z.string(),"));
        assert!(out.contains("  in: z.string(),"));
        assert!(out.contains("  out: z.string(),"));
        assert!(out.contains("  reason: z.string().optional(),"));
    }

    #[test]
    fn link_becomes_z_string() {
        let schema = ir::Schema {
            records: vec![ir::Record {
                name: "Atlas".to_string(),
                id_strategy: ir::IdStrategy::Uuidv7,
                fields: vec![field("parent", ir::Ty::Link("Atlas".to_string()), true)],
            }],
            ..Default::default()
        };
        let out = ZodEmitter::new().emit(&schema);
        assert!(out.contains("  parent: z.string(),"));
    }

    #[test]
    fn literal_union_collapses_to_z_enum() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "T".to_string(),
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
        let out = ZodEmitter::new().emit(&schema);
        assert!(out.contains("  visibility: z.enum([\"public\", \"private\"]),"));
    }

    #[test]
    fn mixed_union_becomes_z_union() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "T".to_string(),
                fields: vec![field(
                    "v",
                    ir::Ty::Union(vec![
                        ir::Ty::Primitive(ir::Prim::String),
                        ir::Ty::Primitive(ir::Prim::Int),
                    ]),
                    true,
                )],
            }],
            ..Default::default()
        };
        let out = ZodEmitter::new().emit(&schema);
        assert!(out.contains("  v: z.union([z.string(), z.number().int()]),"));
    }
}
