//! Parser — KDL schema file → [`crate::ir::Schema`].
//!
//! Parsing happens in two stages:
//!
//! 1. **deserialize** — `club-kdl`'s `KdlDeserialize` derive fills the
//!    KDL-shaped `raw` structs from the document.
//! 2. **lower** — `raw` structs are converted into the validated
//!    [`crate::ir`] representation: enum-like strings become real enums, the
//!    flat `type` string becomes a [`crate::ir::Ty`], and channel semantics
//!    (datagram `channel_id` requirements) are checked.
//!
//! Only the modern dialect is accepted — `protocol` / `channel` / `request` /
//! `returns` / `event` / `field`, standalone `struct` / `enum`, and the
//! entity dialect `record` / `relation` / `id`. Legacy `service` / `method` /
//! `send` / `recv` constructs are not parsed.

mod raw;

use std::collections::HashSet;

use crate::ir;

/// An error produced while parsing a KDL schema.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// The input was not well-formed KDL, or did not match the schema shape.
    #[error("KDL parse error: {0}")]
    Kdl(String),
    /// The input parsed as KDL but violated a schema rule.
    #[error("schema validation error: {0}")]
    Validation(String),
}

/// Parse a KDL schema source string into a [`Schema`](ir::Schema).
///
/// # Errors
///
/// Returns [`ParseError::Kdl`] if the input is not well-formed KDL or does not
/// match the schema shape, and [`ParseError::Validation`] if it parses but
/// breaks a schema rule (e.g. a datagram channel without a `channel_id`).
pub fn parse(src: &str) -> Result<ir::Schema, ParseError> {
    let raw: raw::RawSchema =
        club_kdl::from_str(src).map_err(|e| ParseError::Kdl(e.to_string()))?;
    lower_schema(raw)
}

// =============================================================================
// Lowering — raw structs → IR
// =============================================================================

fn lower_schema(raw: raw::RawSchema) -> Result<ir::Schema, ParseError> {
    let mut types = Vec::with_capacity(raw.structs.len() + raw.enums.len());
    for s in raw.structs {
        types.push(lower_struct(s)?);
    }
    for e in raw.enums {
        types.push(lower_enum(e));
    }
    let records = raw
        .records
        .into_iter()
        .map(lower_record)
        .collect::<Result<Vec<_>, _>>()?;
    let relations = raw
        .relations
        .into_iter()
        .map(lower_relation)
        .collect::<Result<Vec<_>, _>>()?;
    let protocol = raw.protocol.map(lower_protocol).transpose()?;
    let schema = ir::Schema {
        types,
        records,
        relations,
        protocol,
    };
    validate_type_refs(&schema)?;
    Ok(schema)
}

/// The set of type names a schema defines, split by reference kind.
///
/// A [`ir::Ty::Named`] (embedded value) must resolve into [`Self::values`];
/// a [`ir::Ty::Link`] (stored reference) must resolve into [`Self::records`].
struct DefinedNames<'a> {
    /// `struct` / `enum` names — valid [`ir::Ty::Named`] targets.
    values: HashSet<&'a str>,
    /// `record` names — valid [`ir::Ty::Link`] targets.
    records: HashSet<&'a str>,
}

/// Check that every type reference resolves: a [`ir::Ty::Named`] to a defined
/// `struct` / `enum`, and a [`ir::Ty::Link`] to a defined `record`. Without
/// this, an unknown type name silently flows through to an emitter and
/// produces source that fails to compile.
fn validate_type_refs(schema: &ir::Schema) -> Result<(), ParseError> {
    let values: HashSet<&str> = schema
        .types
        .iter()
        .map(|t| match t {
            ir::TypeDef::Struct { name, .. } | ir::TypeDef::Enum { name, .. } => name.as_str(),
        })
        .collect();
    let records: HashSet<&str> = schema.records.iter().map(|r| r.name.as_str()).collect();
    let defined = DefinedNames { values, records };

    for ty in &schema.types {
        if let ir::TypeDef::Struct { fields, .. } = ty {
            check_fields(fields, &defined)?;
        }
    }
    for record in &schema.records {
        check_fields(&record.fields, &defined)?;
    }
    for relation in &schema.relations {
        check_fields(&relation.fields, &defined)?;
        // A relation's endpoints must name defined records.
        for (role, endpoint) in [("from", &relation.from), ("to", &relation.to)] {
            if !defined.records.contains(endpoint.as_str()) {
                return Err(ParseError::Validation(format!(
                    "relation {:?} {role}={endpoint:?} references unknown record; \
                     define it as a `record`",
                    relation.name
                )));
            }
        }
    }
    if let Some(protocol) = &schema.protocol {
        for channel in &protocol.channels {
            for request in &channel.requests {
                check_fields(&request.fields, &defined)?;
                if let Some(returns) = &request.returns {
                    check_fields(&returns.fields, &defined)?;
                }
            }
            for event in &channel.events {
                check_fields(&event.fields, &defined)?;
            }
        }
    }
    Ok(())
}

fn check_fields(fields: &[ir::Field], defined: &DefinedNames) -> Result<(), ParseError> {
    for field in fields {
        check_ty(&field.ty, &field.name, defined)?;
    }
    Ok(())
}

fn check_ty(ty: &ir::Ty, field: &str, defined: &DefinedNames) -> Result<(), ParseError> {
    match ty {
        ir::Ty::Primitive(_) | ir::Ty::Literal(_) => Ok(()),
        ir::Ty::Array(inner) => check_ty(inner, field, defined),
        ir::Ty::Union(members) => members.iter().try_for_each(|m| check_ty(m, field, defined)),
        ir::Ty::Named(name) if defined.values.contains(name.as_str()) => Ok(()),
        ir::Ty::Named(name) => Err(ParseError::Validation(format!(
            "field {field:?} references unknown type {name:?}; \
             define it as a `struct` or `enum`, or use `link<{name}>` for a `record`"
        ))),
        ir::Ty::Link(name) if defined.records.contains(name.as_str()) => Ok(()),
        ir::Ty::Link(name) => Err(ParseError::Validation(format!(
            "field {field:?} links to unknown record {name:?}; \
             define it as a `record`"
        ))),
    }
}

fn lower_struct(raw: raw::RawStruct) -> Result<ir::TypeDef, ParseError> {
    Ok(ir::TypeDef::Struct {
        name: raw.name,
        description: raw.description,
        fields: lower_fields(raw.fields)?,
    })
}

fn lower_enum(raw: raw::RawEnum) -> ir::TypeDef {
    ir::TypeDef::Enum {
        name: raw.name,
        description: raw.description,
        variants: raw.variants.into_iter().map(|v| v.name).collect(),
    }
}

fn lower_record(raw: raw::RawRecord) -> Result<ir::Record, ParseError> {
    let id_strategy = lower_id_strategy(raw.id.and_then(|i| i.strategy).as_deref())?;
    Ok(ir::Record {
        name: raw.name,
        description: raw.description,
        id_strategy,
        fields: lower_fields(raw.fields)?,
    })
}

fn lower_relation(raw: raw::RawRelation) -> Result<ir::Relation, ParseError> {
    Ok(ir::Relation {
        name: raw.name,
        description: raw.description,
        from: raw.from,
        to: raw.to,
        unique: raw.unique,
        fields: lower_fields(raw.fields)?,
    })
}

fn lower_id_strategy(s: Option<&str>) -> Result<ir::IdStrategy, ParseError> {
    match s {
        None | Some("uuidv7") => Ok(ir::IdStrategy::Uuidv7),
        Some("ulid") => Ok(ir::IdStrategy::Ulid),
        Some("manual") => Ok(ir::IdStrategy::Manual),
        Some(other) => Err(ParseError::Validation(format!(
            "unknown id `strategy` value {other:?} (expected uuidv7/ulid/manual)"
        ))),
    }
}

fn lower_protocol(raw: raw::RawProtocol) -> Result<ir::Protocol, ParseError> {
    let channels = raw
        .channels
        .into_iter()
        .map(lower_channel)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ir::Protocol {
        name: raw.name,
        version: raw.version,
        namespace: raw.namespace,
        description: raw.description,
        channels,
    })
}

fn lower_channel(raw: raw::RawChannel) -> Result<ir::Channel, ParseError> {
    let from = lower_channel_from(&raw.from)?;
    let lifetime = lower_channel_lifetime(&raw.lifetime)?;
    let backend = lower_channel_backend(raw.backend.as_deref())?;
    let requests = raw
        .requests
        .into_iter()
        .map(lower_request)
        .collect::<Result<Vec<_>, _>>()?;
    let events = raw
        .events
        .into_iter()
        .map(lower_event)
        .collect::<Result<Vec<_>, _>>()?;

    // Semantic validation — mirrors club-unison's `Channel::validate`.
    if backend == ir::ChannelBackend::Datagram {
        match raw.channel_id {
            None => {
                return Err(ParseError::Validation(format!(
                    "channel {:?} has backend=\"datagram\" but no channel_id; \
                     channel_id=N (1..) is required",
                    raw.name
                )));
            }
            Some(0) => {
                return Err(ParseError::Validation(format!(
                    "channel {:?} has channel_id=0 which is reserved; use 1..",
                    raw.name
                )));
            }
            Some(_) => {}
        }
        if !requests.is_empty() {
            return Err(ParseError::Validation(format!(
                "channel {:?} has backend=\"datagram\" with request blocks; \
                 datagram channels support event only (no Request/Response)",
                raw.name
            )));
        }
    }

    Ok(ir::Channel {
        name: raw.name,
        from,
        lifetime,
        backend,
        channel_id: raw.channel_id,
        requests,
        events,
    })
}

fn lower_request(raw: raw::RawRequest) -> Result<ir::Request, ParseError> {
    Ok(ir::Request {
        name: raw.name,
        fields: lower_fields(raw.fields)?,
        returns: raw.returns.map(lower_message).transpose()?,
    })
}

fn lower_event(raw: raw::RawEvent) -> Result<ir::Event, ParseError> {
    Ok(ir::Event {
        name: raw.name,
        fields: lower_fields(raw.fields)?,
    })
}

fn lower_message(raw: raw::RawMessage) -> Result<ir::Message, ParseError> {
    Ok(ir::Message {
        name: raw.name,
        fields: lower_fields(raw.fields)?,
    })
}

fn lower_fields(raw: Vec<raw::RawField>) -> Result<Vec<ir::Field>, ParseError> {
    raw.into_iter().map(lower_field).collect()
}

fn lower_field(raw: raw::RawField) -> Result<ir::Field, ParseError> {
    Ok(ir::Field {
        ty: parse_ty(&raw.type_str)?,
        name: raw.name,
        required: !raw.optional,
        flexible: raw.flexible,
        default: raw.default,
        description: raw.description,
        constraints: ir::Constraints {
            min: raw.min,
            max: raw.max,
            min_length: raw.min_length,
            max_length: raw.max_length,
            pattern: raw.pattern,
        },
    })
}

// =============================================================================
// Scalar lowering helpers
// =============================================================================

fn lower_channel_from(s: &str) -> Result<ir::ChannelFrom, ParseError> {
    match s {
        "client" => Ok(ir::ChannelFrom::Client),
        "server" => Ok(ir::ChannelFrom::Server),
        "either" => Ok(ir::ChannelFrom::Either),
        other => Err(ParseError::Validation(format!(
            "unknown channel `from` value {other:?} (expected client/server/either)"
        ))),
    }
}

fn lower_channel_lifetime(s: &str) -> Result<ir::ChannelLifetime, ParseError> {
    match s {
        "transient" => Ok(ir::ChannelLifetime::Transient),
        "persistent" => Ok(ir::ChannelLifetime::Persistent),
        other => Err(ParseError::Validation(format!(
            "unknown channel `lifetime` value {other:?} (expected transient/persistent)"
        ))),
    }
}

fn lower_channel_backend(s: Option<&str>) -> Result<ir::ChannelBackend, ParseError> {
    match s {
        None | Some("stream") => Ok(ir::ChannelBackend::Stream),
        Some("datagram") => Ok(ir::ChannelBackend::Datagram),
        Some(other) => Err(ParseError::Validation(format!(
            "unknown channel `backend` value {other:?} (expected stream/datagram)"
        ))),
    }
}

/// Parse a field-type string into a [`Ty`](ir::Ty).
///
/// Grammar (lowest precedence first):
///
/// - `A | B | ...` — a [`Ty::Union`](ir::Ty::Union). `|` is split only at
///   the top level (not inside `array<...>` / `link<...>`).
/// - `array<T>` — a [`Ty::Array`](ir::Ty::Array).
/// - `link<Name>` — a [`Ty::Link`](ir::Ty::Link), a stored record reference.
/// - `'literal'` — a [`Ty::Literal`](ir::Ty::Literal) (single-quoted).
/// - the primitive set — `string` / `int` / `float` / `bool` / `datetime` /
///   `json`. `object` aliases `json`, `number` aliases `float`, `timestamp`
///   aliases `datetime`.
/// - any other identifier — a [`Ty::Named`](ir::Ty::Named) reference to a
///   `struct` / `enum`.
fn parse_ty(s: &str) -> Result<ir::Ty, ParseError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseError::Validation("empty field type".to_string()));
    }

    // Union has the lowest precedence — split on top-level `|`.
    let members = split_top_level_union(s);
    if members.len() > 1 {
        let parsed = members
            .iter()
            .map(|m| parse_ty(m))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(ir::Ty::Union(parsed));
    }

    parse_atom(s)
}

/// Parse a single (non-union) type atom.
fn parse_atom(s: &str) -> Result<ir::Ty, ParseError> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix("array<").and_then(|r| r.strip_suffix('>')) {
        return Ok(ir::Ty::Array(Box::new(parse_ty(inner)?)));
    }
    if let Some(inner) = s.strip_prefix("link<").and_then(|r| r.strip_suffix('>')) {
        let name = inner.trim();
        if name.is_empty() {
            return Err(ParseError::Validation(
                "empty record name in `link<>`".to_string(),
            ));
        }
        return Ok(ir::Ty::Link(name.to_string()));
    }
    // String literal: `'value'`.
    if let Some(inner) = s.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
        return Ok(ir::Ty::Literal(inner.to_string()));
    }
    let prim = match s {
        "string" => Some(ir::Prim::String),
        "int" => Some(ir::Prim::Int),
        "float" | "number" => Some(ir::Prim::Float),
        "bool" => Some(ir::Prim::Bool),
        "datetime" | "timestamp" => Some(ir::Prim::Datetime),
        "json" | "object" => Some(ir::Prim::Json),
        _ => None,
    };
    match prim {
        Some(p) => Ok(ir::Ty::Primitive(p)),
        None if s.is_empty() => Err(ParseError::Validation("empty field type".to_string())),
        None => Ok(ir::Ty::Named(s.to_string())),
    }
}

/// Split a type string on top-level `|`, ignoring `|` nested inside `<...>`
/// brackets or `'...'` string literals. Returns the trimmed segments.
fn split_top_level_union(s: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut depth: usize = 0;
    let mut in_literal = false;
    for c in s.chars() {
        match c {
            '\'' => {
                in_literal = !in_literal;
                cur.push(c);
            }
            '<' if !in_literal => {
                depth += 1;
                cur.push(c);
            }
            '>' if !in_literal => {
                depth = depth.saturating_sub(1);
                cur.push(c);
            }
            '|' if depth == 0 && !in_literal => {
                parts.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    parts.push(cur.trim().to_string());
    parts
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_protocol_with_request_and_returns() {
        let src = r#"
            protocol "ping-pong" version="2.0.0" {
                namespace "test.pp"
                channel "pp" from="client" lifetime="persistent" {
                    request "Ping" {
                        field "message" type="string"
                        returns "Pong" {
                            field "reply" type="string"
                        }
                    }
                }
            }
        "#;
        let schema = parse(src).expect("should parse");
        let protocol = schema.protocol.expect("has protocol");
        assert_eq!(protocol.name, "ping-pong");
        assert_eq!(protocol.version, "2.0.0");
        assert_eq!(protocol.namespace.as_deref(), Some("test.pp"));
        assert_eq!(protocol.channels.len(), 1);

        let channel = &protocol.channels[0];
        assert_eq!(channel.name, "pp");
        assert_eq!(channel.from, ir::ChannelFrom::Client);
        assert_eq!(channel.lifetime, ir::ChannelLifetime::Persistent);
        assert_eq!(channel.backend, ir::ChannelBackend::Stream);
        assert_eq!(channel.requests.len(), 1);

        let request = &channel.requests[0];
        assert_eq!(request.name, "Ping");
        assert_eq!(
            request.fields,
            vec![ir::Field {
                name: "message".to_string(),
                ty: ir::Ty::Primitive(ir::Prim::String),
                required: true,
                flexible: false,
                default: None,
                description: None,
                constraints: ir::Constraints::default(),
            }]
        );
        let returns = request.returns.as_ref().expect("has returns");
        assert_eq!(returns.name, "Pong");
        assert_eq!(returns.fields[0].name, "reply");
    }

    #[test]
    fn parses_events_and_optional_fields() {
        let src = r#"
            protocol "p" version="1.0.0" {
                channel "c" from="server" lifetime="persistent" {
                    event "Tick" {
                        field "seq" type="int"
                        field "note" type="string" optional=#true
                    }
                }
            }
        "#;
        let schema = parse(src).expect("should parse");
        let channel = &schema.protocol.unwrap().channels[0];
        let event = &channel.events[0];
        assert_eq!(event.name, "Tick");
        assert!(
            event.fields[0].required,
            "unmarked field defaults to required"
        );
        assert!(
            !event.fields[1].required,
            "explicit optional=#true → not required"
        );
    }

    #[test]
    fn datagram_channel_requires_channel_id() {
        let src = r#"
            protocol "p" version="1.0.0" {
                channel "metric" from="server" lifetime="persistent" backend="datagram" {
                    event "M" { field "v" type="int" }
                }
            }
        "#;
        let err = parse(src).expect_err("datagram without channel_id is invalid");
        assert!(matches!(err, ParseError::Validation(_)));
    }

    #[test]
    fn datagram_channel_rejects_requests() {
        let src = r#"
            protocol "p" version="1.0.0" {
                channel "c" from="client" lifetime="persistent" backend="datagram" channel_id=1 {
                    request "R" { field "x" type="int" }
                }
            }
        "#;
        let err = parse(src).expect_err("datagram with request is invalid");
        assert!(matches!(err, ParseError::Validation(_)));
    }

    #[test]
    fn parses_datagram_channel_with_id() {
        let src = r#"
            protocol "p" version="1.0.0" {
                channel "metric" from="server" lifetime="persistent" backend="datagram" channel_id=7 {
                    event "M" { field "v" type="int" }
                }
            }
        "#;
        let channel = &parse(src).unwrap().protocol.unwrap().channels[0];
        assert_eq!(channel.backend, ir::ChannelBackend::Datagram);
        assert_eq!(channel.channel_id, Some(7));
    }

    #[test]
    fn parses_data_dialect_struct_and_enum() {
        let src = r#"
            struct "User" {
                field "id" type="string"
                field "tags" type="array<string>"
                field "role" type="Role"
            }
            enum "Role" {
                variant "admin"
                variant "member"
            }
        "#;
        let schema = parse(src).expect("should parse");
        assert!(schema.protocol.is_none());
        assert_eq!(schema.types.len(), 2);

        match &schema.types[0] {
            ir::TypeDef::Struct { name, fields, .. } => {
                assert_eq!(name, "User");
                assert_eq!(
                    fields[1].ty,
                    ir::Ty::Array(Box::new(ir::Ty::Primitive(ir::Prim::String)))
                );
                assert_eq!(fields[2].ty, ir::Ty::Named("Role".to_string()));
            }
            other => panic!("expected struct, got {other:?}"),
        }
        match &schema.types[1] {
            ir::TypeDef::Enum { name, variants, .. } => {
                assert_eq!(name, "Role");
                assert_eq!(variants, &["admin", "member"]);
            }
            other => panic!("expected enum, got {other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_channel_from() {
        let src = r#"
            protocol "p" version="1.0.0" {
                channel "c" from="nobody" lifetime="persistent" {
                    event "E" { field "x" type="int" }
                }
            }
        "#;
        let err = parse(src).expect_err("unknown from value is invalid");
        assert!(matches!(err, ParseError::Validation(_)));
    }

    #[test]
    fn primitive_type_aliases() {
        assert_eq!(
            parse_ty("object").unwrap(),
            ir::Ty::Primitive(ir::Prim::Json)
        );
        assert_eq!(
            parse_ty("number").unwrap(),
            ir::Ty::Primitive(ir::Prim::Float)
        );
        assert_eq!(
            parse_ty("timestamp").unwrap(),
            ir::Ty::Primitive(ir::Prim::Datetime)
        );
    }

    #[test]
    fn rejects_unknown_type_reference() {
        let src = r#"
            struct "User" {
                field "role" type="Role"
            }
        "#;
        // `Role` is never defined as a struct/enum.
        let err = parse(src).expect_err("unknown type reference is invalid");
        assert!(matches!(err, ParseError::Validation(_)));
    }

    #[test]
    fn accepts_array_of_defined_type() {
        let src = r#"
            struct "Team" {
                field "members" type="array<User>"
            }
            struct "User" {
                field "id" type="string"
            }
        "#;
        // `array<User>` resolves because `User` is defined; order-independent.
        parse(src).expect("array<User> with User defined should parse");
    }

    // -------------------------------------------------------------------------
    // Tier 1 — record / relation / link / union
    // -------------------------------------------------------------------------

    #[test]
    fn parses_record_with_id_strategy_and_fields() {
        let src = r#"
            record "Atlas" {
                id strategy="uuidv7"
                field "name"   type="string"
                field "parent" type="link<Atlas>"
            }
        "#;
        let schema = parse(src).expect("record should parse");
        assert_eq!(schema.records.len(), 1);
        let atlas = &schema.records[0];
        assert_eq!(atlas.name, "Atlas");
        assert_eq!(atlas.id_strategy, ir::IdStrategy::Uuidv7);
        assert_eq!(atlas.fields[0].name, "name");
        // self-link resolves because `Atlas` is itself a defined record.
        assert_eq!(atlas.fields[1].ty, ir::Ty::Link("Atlas".to_string()));
    }

    #[test]
    fn record_id_strategy_defaults_to_uuidv7_when_absent() {
        let src = r#"
            record "Note" {
                field "body" type="string"
            }
        "#;
        let schema = parse(src).expect("record without `id` node should parse");
        assert_eq!(schema.records[0].id_strategy, ir::IdStrategy::Uuidv7);
    }

    #[test]
    fn parses_all_id_strategies() {
        for (kw, expected) in [
            ("ulid", ir::IdStrategy::Ulid),
            ("manual", ir::IdStrategy::Manual),
            ("uuidv7", ir::IdStrategy::Uuidv7),
        ] {
            let src = format!(
                r#"record "R" {{ id strategy="{kw}"
                       field "x" type="string" }}"#
            );
            let schema = parse(&src).expect("record parses");
            assert_eq!(schema.records[0].id_strategy, expected);
        }
    }

    #[test]
    fn rejects_unknown_id_strategy() {
        let src = r#"record "R" { id strategy="snowflake"
                       field "x" type="string" }"#;
        let err = parse(src).expect_err("unknown id strategy is invalid");
        assert!(matches!(err, ParseError::Validation(_)));
    }

    #[test]
    fn parses_relation_with_endpoints_and_edge_fields() {
        let src = r#"
            record "Memory" {
                field "body" type="string"
            }
            relation "derivedFrom" from="Memory" to="Memory" unique=#true {
                field "confidence" type="float"
                field "reason"     type="string"
            }
        "#;
        let schema = parse(src).expect("relation should parse");
        assert_eq!(schema.relations.len(), 1);
        let rel = &schema.relations[0];
        assert_eq!(rel.name, "derivedFrom");
        assert_eq!(rel.from, "Memory");
        assert_eq!(rel.to, "Memory");
        assert!(rel.unique);
        assert_eq!(rel.fields.len(), 2);
        assert_eq!(rel.fields[0].name, "confidence");
    }

    #[test]
    fn relation_unique_defaults_to_false() {
        let src = r#"
            record "A" { field "x" type="string" }
            relation "rel" from="A" to="A"
        "#;
        let schema = parse(src).expect("relation parses");
        assert!(!schema.relations[0].unique);
    }

    #[test]
    fn rejects_relation_with_unknown_endpoint() {
        let src = r#"
            record "A" { field "x" type="string" }
            relation "rel" from="A" to="Ghost"
        "#;
        let err = parse(src).expect_err("unknown relation endpoint is invalid");
        assert!(matches!(err, ParseError::Validation(_)));
    }

    #[test]
    fn parse_ty_link_and_literal_and_union() {
        assert_eq!(
            parse_ty("link<Atlas>").unwrap(),
            ir::Ty::Link("Atlas".to_string())
        );
        assert_eq!(
            parse_ty("'public'").unwrap(),
            ir::Ty::Literal("public".to_string())
        );
        assert_eq!(
            parse_ty("'public' | 'private'").unwrap(),
            ir::Ty::Union(vec![
                ir::Ty::Literal("public".to_string()),
                ir::Ty::Literal("private".to_string()),
            ])
        );
        assert_eq!(
            parse_ty("string | int | bool").unwrap(),
            ir::Ty::Union(vec![
                ir::Ty::Primitive(ir::Prim::String),
                ir::Ty::Primitive(ir::Prim::Int),
                ir::Ty::Primitive(ir::Prim::Bool),
            ])
        );
    }

    #[test]
    fn union_does_not_split_inside_brackets() {
        // `|` inside `array<...>` must not be treated as a union separator.
        // (there is no nested union here, but the splitter must stay depth-aware)
        assert_eq!(
            parse_ty("array<string>").unwrap(),
            ir::Ty::Array(Box::new(ir::Ty::Primitive(ir::Prim::String)))
        );
    }

    #[test]
    fn link_to_unknown_record_is_rejected() {
        let src = r#"
            record "Atlas" {
                field "parent" type="link<Ghost>"
            }
        "#;
        let err = parse(src).expect_err("link to undefined record is invalid");
        assert!(matches!(err, ParseError::Validation(_)));
    }

    #[test]
    fn flexible_and_default_properties_are_lowered() {
        let src = r#"
            record "Atlas" {
                field "metadata"   type="object" flexible=#true
                field "visibility" type="'public' | 'private'" default="private"
            }
        "#;
        let schema = parse(src).expect("record should parse");
        let fields = &schema.records[0].fields;
        assert!(fields[0].flexible, "flexible=#true is lowered");
        assert!(!fields[1].flexible, "absent flexible defaults false");
        assert_eq!(fields[1].default.as_deref(), Some("private"));
    }

    #[test]
    fn bare_name_is_embedded_link_is_stored() {
        // A bare `Name` resolves to a struct/enum; `link<Name>` to a record.
        // The same identifier in both forms must keep its distinct meaning.
        let src = r#"
            struct "GeoPoint" {
                field "lat" type="float"
            }
            record "Place" {
                field "at"     type="GeoPoint"
                field "parent" type="link<Place>"
            }
        "#;
        let schema = parse(src).expect("schema parses");
        let fields = &schema.records[0].fields;
        assert_eq!(fields[0].ty, ir::Ty::Named("GeoPoint".to_string()));
        assert_eq!(fields[1].ty, ir::Ty::Link("Place".to_string()));
    }

    // -------------------------------------------------------------------------
    // Tier 2 — description / constraints
    // -------------------------------------------------------------------------

    #[test]
    fn record_and_field_descriptions_are_lowered() {
        let src = r#"
            record "Memory" description="User memory with content" {
                field "content" type="string" description="Memory content text"
            }
        "#;
        let schema = parse(src).expect("record with descriptions parses");
        let memory = &schema.records[0];
        assert_eq!(
            memory.description.as_deref(),
            Some("User memory with content")
        );
        assert_eq!(
            memory.fields[0].description.as_deref(),
            Some("Memory content text")
        );
    }

    #[test]
    fn struct_enum_relation_descriptions_are_lowered() {
        let src = r#"
            struct "Point" description="A 2D point" {
                field "x" type="float"
            }
            enum "Color" description="An RGB primary" {
                variant "red"
            }
            record "Node" { field "v" type="int" }
            relation "edge" from="Node" to="Node" description="A directed edge"
        "#;
        let schema = parse(src).expect("schema parses");
        match &schema.types[0] {
            ir::TypeDef::Struct { description, .. } => {
                assert_eq!(description.as_deref(), Some("A 2D point"));
            }
            other => panic!("expected struct, got {other:?}"),
        }
        match &schema.types[1] {
            ir::TypeDef::Enum { description, .. } => {
                assert_eq!(description.as_deref(), Some("An RGB primary"));
            }
            other => panic!("expected enum, got {other:?}"),
        }
        assert_eq!(
            schema.relations[0].description.as_deref(),
            Some("A directed edge")
        );
    }

    #[test]
    fn field_constraints_are_lowered() {
        let src = r#"
            struct "Profile" {
                field "confidence" type="float" min=0 max=1
                field "name"       type="string" min_length=1 max_length=32 pattern="^[a-z]+$"
            }
        "#;
        let schema = parse(src).expect("schema parses");
        let fields = match &schema.types[0] {
            ir::TypeDef::Struct { fields, .. } => fields,
            other => panic!("expected struct, got {other:?}"),
        };
        assert_eq!(fields[0].constraints.min, Some(0));
        assert_eq!(fields[0].constraints.max, Some(1));
        assert_eq!(fields[1].constraints.min_length, Some(1));
        assert_eq!(fields[1].constraints.max_length, Some(32));
        assert_eq!(fields[1].constraints.pattern.as_deref(), Some("^[a-z]+$"));
    }

    #[test]
    fn absent_constraints_and_description_default_to_none() {
        let src = r#"
            struct "Bare" {
                field "x" type="int"
            }
        "#;
        let schema = parse(src).expect("schema parses");
        let fields = match &schema.types[0] {
            ir::TypeDef::Struct { fields, .. } => fields,
            other => panic!("expected struct, got {other:?}"),
        };
        assert!(fields[0].description.is_none());
        assert!(
            fields[0].constraints.is_empty(),
            "a field with no constraint properties has empty Constraints"
        );
    }
}
