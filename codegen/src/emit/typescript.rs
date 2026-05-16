//! TypeScript emitter — renders [`ir::Schema`] into TypeScript source text.
//!
//! Ported from club-unison's `codegen/typescript.rs`. Output format is kept
//! faithful so Phase 1 Step 6 can diff this against club-unison's generator
//! for regression detection.
//!
//! ## What it emits
//!
//! - A fixed import / type-alias header (`Timestamp`, `UUID`, `LanguageCode`).
//! - data dialect: every [`ir::TypeDef`] — `interface` for structs, string
//!   `enum` for enums.
//! - protocol dialect: for every [`ir::Channel`], per club-unison's
//!   `generate_channel`:
//!   - one `interface` per event payload, request payload and `returns` message,
//!   - a `<Channel>ChannelEventTypes` type map,
//!   - a `<Channel>ChannelRequestTypes` type map,
//!   - a `<Channel>ChannelMeta` `const` carrying channel metadata.
//!
//! ## Differences from club-unison (IR-driven port)
//!
//! - No inline `_inline_*` message skipping, no `service` legacy handling, and
//!   no field-level `@default` / `@minimum` / `@pattern` JSDoc — the IR does
//!   not carry constraints or defaults.
//! - Named type references emit the bare PascalCase identifier; the special
//!   `timestamp` / `uuid` / `language_code` aliases of club-unison are still
//!   honoured for compatibility with the fixed header.

use crate::ir;
use crate::Emitter;

use super::case::to_pascal_case;

/// The TypeScript code generation target.
#[derive(Debug, Default, Clone, Copy)]
pub struct TypeScriptEmitter;

impl TypeScriptEmitter {
    /// Create a new [`TypeScriptEmitter`].
    pub fn new() -> Self {
        Self
    }
}

impl Emitter for TypeScriptEmitter {
    fn emit(&self, schema: &ir::Schema) -> String {
        let mut code = String::new();
        code.push_str(HEADER);
        code.push('\n');

        // data dialect — standalone type definitions.
        for ty in &schema.types {
            match ty {
                ir::TypeDef::Struct { name, fields } => {
                    code.push_str(&render_interface(name, fields));
                }
                ir::TypeDef::Enum { name, variants } => {
                    code.push_str(&render_enum(name, variants));
                }
            }
            code.push_str("\n\n");
        }

        // protocol dialect — channel interfaces + metadata.
        if let Some(protocol) = &schema.protocol {
            if let Some(namespace) = &protocol.namespace {
                code.push_str(&format!("// Namespace: {namespace}\n"));
                code.push_str(&format!("// Version: {}\n\n", protocol.version));
            }
            for channel in &protocol.channels {
                code.push_str(&render_channel(channel));
                code.push_str("\n\n");
            }
        }

        code
    }
}

/// Fixed header block, matching club-unison's `generate_imports`.
const HEADER: &str = "\
// Auto-generated TypeScript definitions
// DO NOT EDIT MANUALLY

export type Timestamp = string; // ISO-8601 format
export type UUID = string;
export type LanguageCode = string; // ISO 639-1 format
";

/// Render a plain `interface` from a name and field list.
fn render_interface(name: &str, fields: &[ir::Field]) -> String {
    let body: Vec<String> = fields.iter().map(render_field).collect();
    format!("export interface {name} {{\n{}\n}}", body.join("\n"))
}

/// Render a string-valued `enum`.
fn render_enum(name: &str, variants: &[String]) -> String {
    let body: Vec<String> = variants
        .iter()
        .map(|v| format!("  {} = '{}',", to_pascal_case(v), v))
        .collect();
    format!("export enum {name} {{\n{}\n}}", body.join("\n"))
}

/// Render a single interface field line.
fn render_field(field: &ir::Field) -> String {
    let optional = if field.required { "" } else { "?" };
    format!(
        "  {}{}: {};",
        field.name,
        optional,
        ty_to_ts(&field.ty)
    )
}

/// Map an [`ir::Ty`] to its TypeScript type expression.
fn ty_to_ts(ty: &ir::Ty) -> String {
    match ty {
        ir::Ty::Primitive(p) => prim_to_ts(*p).to_string(),
        ir::Ty::Array(inner) => format!("{}[]", ty_to_ts(inner)),
        ir::Ty::Named(name) => named_to_ts(name),
    }
}

/// Map an [`ir::Prim`] to its TypeScript type.
fn prim_to_ts(p: ir::Prim) -> &'static str {
    match p {
        ir::Prim::String => "string",
        ir::Prim::Int | ir::Prim::Float => "number",
        ir::Prim::Bool => "boolean",
        ir::Prim::Datetime => "Timestamp",
        ir::Prim::Json => "any",
    }
}

/// Resolve a named type reference, honouring club-unison's special aliases.
fn named_to_ts(name: &str) -> String {
    match name {
        "timestamp" => "Timestamp".to_string(),
        "uuid" => "UUID".to_string(),
        "language_code" => "LanguageCode".to_string(),
        _ => to_pascal_case(name),
    }
}

/// Render a payload `interface` with a leading JSDoc comment of the given kind.
fn render_payload_interface(kind: &str, name: &str, fields: &[ir::Field]) -> String {
    if fields.is_empty() {
        format!("/** {kind} \"{name}\" — empty payload */\nexport interface {name} {{}}")
    } else {
        let body: Vec<String> = fields.iter().map(render_field).collect();
        format!(
            "/** {kind} \"{name}\" */\nexport interface {name} {{\n{}\n}}",
            body.join("\n")
        )
    }
}

/// Render the full block for one channel: payload interfaces, the event /
/// request type maps, and the `ChannelMeta` const. Ported from
/// club-unison's `generate_channel`.
fn render_channel(channel: &ir::Channel) -> String {
    let mut code = String::new();

    let backend_str = match channel.backend {
        ir::ChannelBackend::Stream => "stream",
        ir::ChannelBackend::Datagram => "datagram",
    };

    // Section header.
    let channel_id_note = match channel.channel_id {
        Some(id) => format!(", channel_id={id}"),
        None => String::new(),
    };
    code.push_str(&format!(
        "// ════════════════════════════════════════════════\n\
         // Channel: {name} (backend={backend_str}{channel_id_note})\n\
         // ════════════════════════════════════════════════\n\n",
        name = channel.name,
    ));

    // Event interfaces.
    let mut event_names: Vec<String> = Vec::new();
    for evt in &channel.events {
        code.push_str(&render_payload_interface("Event", &evt.name, &evt.fields));
        code.push_str("\n\n");
        event_names.push(evt.name.clone());
    }

    // Request / response interfaces. Each entry is (request name, response name).
    let mut request_mappings: Vec<(String, String)> = Vec::new();
    for req in &channel.requests {
        code.push_str(&render_payload_interface("Request", &req.name, &req.fields));
        code.push_str("\n\n");

        let response_name = match &req.returns {
            Some(returns) => {
                code.push_str(&render_payload_interface(
                    "Response",
                    &returns.name,
                    &returns.fields,
                ));
                code.push_str("\n\n");
                returns.name.clone()
            }
            None => "void".to_string(),
        };
        request_mappings.push((req.name.clone(), response_name));
    }

    let pascal = to_pascal_case(&channel.name);

    // Event type map.
    let event_types_name = format!("{pascal}ChannelEventTypes");
    code.push_str(&format!(
        "/** Event name → 生成 interface の map for \"{}\" (= type-narrowing 用) */\n",
        channel.name
    ));
    if event_names.is_empty() {
        code.push_str(&format!(
            "export type {event_types_name} = Record<string, never>;\n\n"
        ));
    } else {
        code.push_str(&format!("export interface {event_types_name} {{\n"));
        for n in &event_names {
            code.push_str(&format!("  {n}: {n};\n"));
        }
        code.push_str("}\n\n");
    }

    // Request type map.
    let request_types_name = format!("{pascal}ChannelRequestTypes");
    code.push_str(&format!(
        "/** Request name → {{ request, response }} 生成 interface の map for \"{}\" */\n",
        channel.name
    ));
    if request_mappings.is_empty() {
        code.push_str(&format!(
            "export type {request_types_name} = Record<string, never>;\n\n"
        ));
    } else {
        code.push_str(&format!("export interface {request_types_name} {{\n"));
        for (req_name, resp_type) in &request_mappings {
            code.push_str(&format!(
                "  {req_name}: {{ request: {req_name}; response: {resp_type} }};\n"
            ));
        }
        code.push_str("}\n\n");
    }

    // Channel metadata const.
    let meta_name = format!("{pascal}ChannelMeta");
    code.push_str(&format!(
        "/** Channel metadata for \"{}\" (= Phase 2 runtime SDK 用 type-narrowing 入力) */\n",
        channel.name
    ));
    code.push_str(&format!("export const {meta_name} = {{\n"));
    code.push_str(&format!("  name: {:?} as const,\n", channel.name));
    code.push_str(&format!("  backend: {backend_str:?} as const,\n"));
    if let Some(cid) = channel.channel_id {
        code.push_str(&format!("  channelId: {cid} as const,\n"));
    }
    let from_str = match channel.from {
        ir::ChannelFrom::Client => "client",
        ir::ChannelFrom::Server => "server",
        ir::ChannelFrom::Either => "either",
    };
    code.push_str(&format!("  from: {from_str:?} as const,\n"));
    let lifetime_str = match channel.lifetime {
        ir::ChannelLifetime::Transient => "transient",
        ir::ChannelLifetime::Persistent => "persistent",
    };
    code.push_str(&format!("  lifetime: {lifetime_str:?} as const,\n"));

    // events list.
    if event_names.is_empty() {
        code.push_str("  events: [] as const,\n");
    } else {
        code.push_str("  events: [");
        for (i, n) in event_names.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(&format!("{n:?}"));
        }
        code.push_str("] as const,\n");
    }

    // requests mapping.
    if request_mappings.is_empty() {
        code.push_str("  requests: {} as const,\n");
    } else {
        code.push_str("  requests: {\n");
        for (req_name, resp_type) in &request_mappings {
            code.push_str(&format!(
                "    {req_name}: {{ request: {req_name:?} as const, response: {resp_type:?} as const }},\n"
            ));
        }
        code.push_str("  } as const,\n");
    }

    // Phantom type carrier.
    code.push_str(&format!(
        "  __types: undefined as unknown as {{ events: {event_types_name}; requests: {request_types_name} }},\n"
    ));
    code.push_str("} as const;\n");

    code
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
        let out = TypeScriptEmitter::new().emit(&ir::Schema::default());
        assert!(out.contains("// DO NOT EDIT MANUALLY"));
        assert!(out.contains("export type Timestamp = string;"));
        assert!(out.contains("export type UUID = string;"));
    }

    #[test]
    fn emits_interface_with_optional_field() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "User".to_string(),
                fields: vec![
                    field("name", ir::Ty::Primitive(ir::Prim::String), true),
                    field("nick", ir::Ty::Primitive(ir::Prim::String), false),
                ],
            }],
            protocol: None,
        };
        let out = TypeScriptEmitter::new().emit(&schema);
        assert!(out.contains("export interface User {"));
        assert!(out.contains("  name: string;"));
        assert!(out.contains("  nick?: string;"));
    }

    #[test]
    fn emits_enum() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Enum {
                name: "Role".to_string(),
                variants: vec!["admin".to_string(), "guest_user".to_string()],
            }],
            protocol: None,
        };
        let out = TypeScriptEmitter::new().emit(&schema);
        assert!(out.contains("export enum Role {"));
        assert!(out.contains("  Admin = 'admin',"));
        assert!(out.contains("  GuestUser = 'guest_user',"));
    }

    #[test]
    fn maps_types() {
        let schema = ir::Schema {
            types: vec![ir::TypeDef::Struct {
                name: "T".to_string(),
                fields: vec![
                    field("n", ir::Ty::Primitive(ir::Prim::Int), true),
                    field("b", ir::Ty::Primitive(ir::Prim::Bool), true),
                    field("at", ir::Ty::Primitive(ir::Prim::Datetime), true),
                    field("blob", ir::Ty::Primitive(ir::Prim::Json), true),
                    field(
                        "tags",
                        ir::Ty::Array(Box::new(ir::Ty::Primitive(ir::Prim::String))),
                        true,
                    ),
                    field("owner", ir::Ty::Named("user_account".to_string()), true),
                ],
            }],
            protocol: None,
        };
        let out = TypeScriptEmitter::new().emit(&schema);
        assert!(out.contains("  n: number;"));
        assert!(out.contains("  b: boolean;"));
        assert!(out.contains("  at: Timestamp;"));
        assert!(out.contains("  blob: any;"));
        assert!(out.contains("  tags: string[];"));
        assert!(out.contains("  owner: UserAccount;"));
    }

    #[test]
    fn emits_channel_interfaces_and_meta() {
        let schema = ir::Schema {
            types: vec![],
            protocol: Some(ir::Protocol {
                name: "ping-pong".to_string(),
                version: "2.0.0".to_string(),
                namespace: Some("demo".to_string()),
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
        let out = TypeScriptEmitter::new().emit(&schema);
        assert!(out.contains("// Namespace: demo"));
        assert!(out.contains("// Channel: ping-pong (backend=stream)"));
        assert!(out.contains("/** Request \"Ping\" */"));
        assert!(out.contains("export interface Ping {"));
        assert!(out.contains("/** Response \"Pong\" */"));
        assert!(out.contains("/** Event \"Tick\" — empty payload */"));
        assert!(out.contains("export interface Tick {}"));
        assert!(out.contains("export interface PingPongChannelEventTypes {"));
        assert!(out.contains("export interface PingPongChannelRequestTypes {"));
        assert!(out.contains("  Ping: { request: Ping; response: Pong };"));
        assert!(out.contains("export const PingPongChannelMeta = {"));
        assert!(out.contains("  name: \"ping-pong\" as const,"));
        assert!(out.contains("  backend: \"stream\" as const,"));
        assert!(out.contains("  from: \"client\" as const,"));
        assert!(out.contains("  lifetime: \"persistent\" as const,"));
        assert!(out.contains("  events: [\"Tick\"] as const,"));
    }

    #[test]
    fn datagram_channel_meta_carries_channel_id() {
        let schema = ir::Schema {
            types: vec![],
            protocol: Some(ir::Protocol {
                name: "telemetry".to_string(),
                version: "1.0.0".to_string(),
                namespace: None,
                description: None,
                channels: vec![ir::Channel {
                    name: "metrics".to_string(),
                    from: ir::ChannelFrom::Server,
                    lifetime: ir::ChannelLifetime::Persistent,
                    backend: ir::ChannelBackend::Datagram,
                    channel_id: Some(7),
                    requests: vec![],
                    events: vec![ir::Event {
                        name: "Sample".to_string(),
                        fields: vec![field("v", ir::Ty::Primitive(ir::Prim::Float), true)],
                    }],
                }],
            }),
        };
        let out = TypeScriptEmitter::new().emit(&schema);
        assert!(out.contains("// Channel: metrics (backend=datagram, channel_id=7)"));
        assert!(out.contains("  channelId: 7 as const,"));
        assert!(out.contains("  requests: {} as const,"));
        assert!(out.contains("export type MetricsChannelRequestTypes = Record<string, never>;"));
    }
}
