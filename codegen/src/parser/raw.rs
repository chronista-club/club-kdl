//! Raw deserialization structs — a near 1:1 mirror of the KDL schema syntax.
//!
//! These types exist only to be filled by `club-kdl`'s [`KdlDeserialize`]
//! derive. They keep KDL-shaped data (strings for enum-like properties, a flat
//! `type` string for field types); the lowering step in [`super`] converts
//! them into the validated [`crate::ir`] representation.
//!
//! Keeping the raw layer separate from the IR means KDL syntax quirks stay
//! contained here, and the IR can be a clean, target-agnostic contract.

use club_kdl::KdlDeserialize;

/// Root of a KDL schema document.
#[derive(Debug, Default, KdlDeserialize)]
#[kdl(document)]
pub struct RawSchema {
    /// The protocol definition, if the document declares one.
    #[kdl(child)]
    pub protocol: Option<RawProtocol>,

    /// Standalone `struct` definitions (data dialect).
    #[kdl(children, name = "struct")]
    pub structs: Vec<RawStruct>,

    /// Standalone `enum` definitions (data dialect).
    #[kdl(children, name = "enum")]
    pub enums: Vec<RawEnum>,
}

/// A `protocol "name" version="x" { ... }` node.
#[derive(Debug, KdlDeserialize)]
#[kdl(name = "protocol")]
pub struct RawProtocol {
    /// Protocol name.
    #[kdl(argument)]
    pub name: String,

    /// Protocol version string.
    #[kdl(property)]
    pub version: String,

    /// Optional `namespace "..."` child.
    #[kdl(child, unwrap_arg)]
    pub namespace: Option<String>,

    /// Optional `description "..."` child.
    #[kdl(child, unwrap_arg)]
    pub description: Option<String>,

    /// `channel` children.
    #[kdl(children, name = "channel")]
    pub channels: Vec<RawChannel>,
}

/// A `channel "name" from="..." lifetime="..." { ... }` node.
#[derive(Debug, KdlDeserialize)]
#[kdl(name = "channel")]
pub struct RawChannel {
    /// Channel name.
    #[kdl(argument)]
    pub name: String,

    /// Which peer opens the channel: `"client"` / `"server"` / `"either"`.
    #[kdl(property)]
    pub from: String,

    /// Channel lifetime: `"transient"` / `"persistent"`.
    #[kdl(property)]
    pub lifetime: String,

    /// Wire backend: `"stream"` (default) / `"datagram"`.
    #[kdl(property)]
    pub backend: Option<String>,

    /// Demux identifier, required for `backend="datagram"`.
    #[kdl(property)]
    pub channel_id: Option<u64>,

    /// `request` children.
    #[kdl(children, name = "request")]
    pub requests: Vec<RawRequest>,

    /// `event` children.
    #[kdl(children, name = "event")]
    pub events: Vec<RawEvent>,
}

/// A `request "Name" { ... }` node within a channel.
#[derive(Debug, KdlDeserialize)]
#[kdl(name = "request")]
pub struct RawRequest {
    /// Request name.
    #[kdl(argument)]
    pub name: String,

    /// Request payload `field` children.
    #[kdl(children, name = "field")]
    pub fields: Vec<RawField>,

    /// Optional `returns "Name" { ... }` block.
    #[kdl(child)]
    pub returns: Option<RawMessage>,
}

/// An `event "Name" { ... }` node within a channel.
#[derive(Debug, KdlDeserialize)]
#[kdl(name = "event")]
pub struct RawEvent {
    /// Event name.
    #[kdl(argument)]
    pub name: String,

    /// Event payload `field` children.
    #[kdl(children, name = "field")]
    pub fields: Vec<RawField>,
}

/// A named payload block — the `returns` block of a [`RawRequest`].
#[derive(Debug, KdlDeserialize)]
pub struct RawMessage {
    /// Message name.
    #[kdl(argument)]
    pub name: String,

    /// Payload `field` children.
    #[kdl(children, name = "field")]
    pub fields: Vec<RawField>,
}

/// A `field "name" type="..." required=#true` node.
#[derive(Debug, KdlDeserialize)]
#[kdl(name = "field")]
pub struct RawField {
    /// Field name.
    #[kdl(argument)]
    pub name: String,

    /// Field type as written in KDL (e.g. `"string"`, `"array<int>"`, `"User"`).
    #[kdl(property, rename = "type")]
    pub type_str: String,

    /// Whether the field is required. Absent property defaults to `false`.
    #[kdl(property, default)]
    pub required: bool,
}

/// A `struct "Name" { field ... }` node (data dialect).
#[derive(Debug, KdlDeserialize)]
#[kdl(name = "struct")]
pub struct RawStruct {
    /// Type name.
    #[kdl(argument)]
    pub name: String,

    /// `field` children.
    #[kdl(children, name = "field")]
    pub fields: Vec<RawField>,
}

/// An `enum "Name" { variant ... }` node (data dialect).
#[derive(Debug, KdlDeserialize)]
#[kdl(name = "enum")]
pub struct RawEnum {
    /// Type name.
    #[kdl(argument)]
    pub name: String,

    /// `variant` children.
    #[kdl(children, name = "variant")]
    pub variants: Vec<RawVariant>,
}

/// A `variant "name"` node within an [`RawEnum`].
#[derive(Debug, KdlDeserialize)]
#[kdl(name = "variant")]
pub struct RawVariant {
    /// Variant name.
    #[kdl(argument)]
    pub name: String,
}
