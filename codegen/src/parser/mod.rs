//! Parser — KDL schema file → [`crate::ir::Schema`].
//!
//! Parsing happens in two stages:
//!
//! 1. **deserialize** — `club-kdl`'s `KdlDeserialize` derive fills the
//!    KDL-shaped [`raw`] structs from the document.
//! 2. **lower** — [`raw`] structs are converted into the validated
//!    [`crate::ir`] representation: enum-like strings become real enums, the
//!    flat `type` string becomes a [`crate::ir::Ty`], and channel semantics
//!    (datagram `channel_id` requirements) are checked.
//!
//! Only the modern dialect is accepted — `protocol` / `channel` / `request` /
//! `returns` / `event` / `field` plus standalone `struct` / `enum`. Legacy
//! `service` / `method` / `send` / `recv` constructs are not parsed.

mod raw;

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
    let protocol = raw.protocol.map(lower_protocol).transpose()?;
    Ok(ir::Schema { types, protocol })
}

fn lower_struct(raw: raw::RawStruct) -> Result<ir::TypeDef, ParseError> {
    Ok(ir::TypeDef::Struct {
        name: raw.name,
        fields: lower_fields(raw.fields)?,
    })
}

fn lower_enum(raw: raw::RawEnum) -> ir::TypeDef {
    ir::TypeDef::Enum {
        name: raw.name,
        variants: raw.variants.into_iter().map(|v| v.name).collect(),
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
        required: raw.required,
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
/// Recognises the primitive set, the `array<T>` generic form, and treats any
/// other identifier as a [`Ty::Named`](ir::Ty::Named) reference to another
/// type. `object` is an alias for `json`, `number` for `float`, and
/// `timestamp` for `datetime`.
fn parse_ty(s: &str) -> Result<ir::Ty, ParseError> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix("array<").and_then(|r| r.strip_suffix('>')) {
        return Ok(ir::Ty::Array(Box::new(parse_ty(inner)?)));
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
                        field "message" type="string" required=#true
                        returns "Pong" {
                            field "reply" type="string" required=#true
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
                        field "seq" type="int" required=#true
                        field "note" type="string"
                    }
                }
            }
        "#;
        let schema = parse(src).expect("should parse");
        let channel = &schema.protocol.unwrap().channels[0];
        let event = &channel.events[0];
        assert_eq!(event.name, "Tick");
        assert!(event.fields[0].required);
        assert!(
            !event.fields[1].required,
            "absent required= defaults to false"
        );
    }

    #[test]
    fn datagram_channel_requires_channel_id() {
        let src = r#"
            protocol "p" version="1.0.0" {
                channel "metric" from="server" lifetime="persistent" backend="datagram" {
                    event "M" { field "v" type="int" required=#true }
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
                    request "R" { field "x" type="int" required=#true }
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
                    event "M" { field "v" type="int" required=#true }
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
                field "id" type="string" required=#true
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
            ir::TypeDef::Struct { name, fields } => {
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
            ir::TypeDef::Enum { name, variants } => {
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
                    event "E" { field "x" type="int" required=#true }
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
}
