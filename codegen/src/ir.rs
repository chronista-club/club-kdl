//! Schema IR — the intermediate representation between the KDL parser and the
//! language emitters.
//!
//! This is the central contract of `club-kdl-codegen`: the parser produces a
//! [`Schema`], and every [`crate::Emitter`] consumes one. Keeping the IR a
//! plain data structure (no behaviour) lets the parser and emitters evolve
//! independently.
//!
//! The IR spans three dialects, all reachable from a single [`Schema`]:
//!
//! - **data dialect** — [`TypeDef`] (`struct` / `enum`) built from [`Field`]
//!   and [`Ty`]. Models standalone named *value* types (embedded, no identity).
//! - **entity dialect** — [`Record`] (`record`) and [`Relation`]
//!   (`relation`), modelling persisted entities with identity and the graph
//!   edges between them. These map to SurrealDB `DEFINE TABLE` statements
//!   (`TYPE NORMAL` for records, `TYPE RELATION` for relations). They reuse
//!   the data dialect's [`Field`].
//! - **protocol dialect** — [`Protocol`] / [`Channel`] / [`Request`] /
//!   [`Event`], modelling KDL channel schemas. Payload definitions reuse the
//!   data dialect's [`Field`], so an emitter writes field-rendering logic once
//!   and it applies to standalone types, entities, and channel payloads.
//!
//! The distinction between a `struct` and a `record` is **identity**: a
//! `struct` is an embedded value type (no `id`, never becomes a table), while
//! a `record` has an `id` and becomes a first-class table. A field referring
//! to a `struct` / `enum` uses a bare [`Ty::Named`] (embedded); a field
//! referring to a `record` uses [`Ty::Link`] (a stored reference).
//!
//! Legacy constructs (`service` / `method` / `stream` / `send` / `recv`) are
//! intentionally **not** modelled — see CLAUDE.md "Legacy は残さない". The IR
//! describes only the modern channel dialect.
//!
//! See the design memory `mem_1Cb5mWnMTdzXfJVoNGFwup` and `ROADMAP.md`
//! (Phase 1) for the full plan.

// =============================================================================
// Schema root
// =============================================================================

/// A whole KDL schema file: standalone type definitions, entity / relation
/// definitions, plus an optional protocol definition.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Schema {
    /// Standalone value-type definitions (`struct` / `enum`) in source order.
    pub types: Vec<TypeDef>,
    /// Entity definitions (`record`) in source order — persisted tables with
    /// identity.
    pub records: Vec<Record>,
    /// Graph edge definitions (`relation`) in source order — `TYPE RELATION`
    /// tables connecting two records.
    pub relations: Vec<Relation>,
    /// The protocol definition, if the file declares one. A file may contain
    /// only data types, only a protocol, or both.
    pub protocol: Option<Protocol>,
}

// =============================================================================
// Data dialect — standalone named types
// =============================================================================

/// A single named type definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeDef {
    /// A record type with named fields.
    Struct {
        /// Type name (e.g. `"User"`).
        name: String,
        /// Fields in source order.
        fields: Vec<Field>,
    },
    /// An enumeration of string-valued variants.
    Enum {
        /// Type name (e.g. `"Role"`).
        name: String,
        /// Variant names in source order.
        variants: Vec<String>,
    },
}

/// A field of a [`TypeDef::Struct`], a [`Record`] / [`Relation`], or of a
/// protocol-dialect payload ([`Request`] / [`Event`] / [`Message`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Field name.
    pub name: String,
    /// Field type.
    pub ty: Ty,
    /// Whether the field is required. `false` maps to the target's optional
    /// form (Rust `Option<T>`, TS `?:`, Zod `.optional()`, SurrealQL `option<T>`).
    pub required: bool,
    /// Whether an `object`-typed field is schemaless (`flexible=#true`). Only
    /// meaningful for [`Ty::Primitive`]`(`[`Prim::Json`]`)`; emitted by the
    /// SurrealQL target as `FLEXIBLE TYPE object`.
    pub flexible: bool,
    /// An optional default value, as written in the KDL `default="..."`
    /// property. Carried verbatim; emitters quote / render it per target.
    pub default: Option<String>,
}

/// A field type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    /// A built-in primitive.
    Primitive(Prim),
    /// A homogeneous array of another type.
    Array(Box<Ty>),
    /// A reference to a [`TypeDef`] (`struct` / `enum`) by name — an
    /// **embedded** value type.
    Named(String),
    /// A reference to a [`Record`] by name — a **stored link** (`link<Name>`
    /// in KDL). Distinct from [`Self::Named`]: a link is a foreign key / edge
    /// target, not an embedded value.
    Link(String),
    /// A union of two or more alternative types (`A | B | ...` in KDL).
    /// Members are kept in source order.
    Union(Vec<Ty>),
    /// A string-literal type (`'value'` in KDL). Used as a union member to
    /// express closed string sets (`'public' | 'private'`).
    Literal(String),
}

/// A built-in primitive type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prim {
    /// UTF-8 string.
    String,
    /// Signed integer.
    Int,
    /// Floating-point number.
    Float,
    /// Boolean.
    Bool,
    /// Date-time.
    Datetime,
    /// Arbitrary JSON value.
    Json,
}

// =============================================================================
// Entity dialect — records and relations
// =============================================================================

/// An entity definition (`record`) — a persisted type with identity.
///
/// Unlike a [`TypeDef::Struct`] (an embedded value), a `Record` carries an
/// `id` and maps to a SurrealDB `DEFINE TABLE ... TYPE NORMAL` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// Record name (e.g. `"Atlas"`).
    pub name: String,
    /// How the record's `id` is generated.
    pub id_strategy: IdStrategy,
    /// Fields in source order. The `id` field is *not* listed here — it is
    /// described by [`Self::id_strategy`].
    pub fields: Vec<Field>,
}

/// How a [`Record`]'s identifier is generated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IdStrategy {
    /// Time-ordered UUID v7. The default when `id` carries no `strategy`.
    #[default]
    Uuidv7,
    /// Lexicographically sortable ULID.
    Ulid,
    /// The id is supplied by the caller (no automatic generation).
    Manual,
}

/// A graph edge definition (`relation`) — a typed, directed connection
/// between two [`Record`]s.
///
/// Maps to a SurrealDB `DEFINE TABLE ... TYPE RELATION IN <from> OUT <to>`
/// statement. A relation may carry its own [`Field`]s (a "property edge").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    /// Relation name (e.g. `"derivedFrom"`).
    pub name: String,
    /// The record name at the `in` end of the edge.
    pub from: String,
    /// The record name at the `out` end of the edge.
    pub to: String,
    /// Whether each `(from, to)` pair must be unique.
    pub unique: bool,
    /// Edge-property fields in source order.
    pub fields: Vec<Field>,
}

// =============================================================================
// Protocol dialect — channel schemas
// =============================================================================

/// A protocol definition: the top-level grouping of [`Channel`]s.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Protocol {
    /// Protocol name (e.g. `"ping-pong"`).
    pub name: String,
    /// Protocol version string (e.g. `"2.0.0"`).
    pub version: String,
    /// Optional namespace, used by emitters for module / package placement.
    pub namespace: Option<String>,
    /// Optional human-readable description.
    pub description: Option<String>,
    /// Channels in source order.
    pub channels: Vec<Channel>,
}

/// A communication channel — the unit of request/response and event traffic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    /// Channel name (e.g. `"ping-pong"`).
    pub name: String,
    /// Which peer opens the channel.
    pub from: ChannelFrom,
    /// How long the channel lives.
    pub lifetime: ChannelLifetime,
    /// Wire backend. Defaults to [`ChannelBackend::Stream`].
    pub backend: ChannelBackend,
    /// Demux identifier, required when [`Self::backend`] is
    /// [`ChannelBackend::Datagram`]. A positive integer (`1..`).
    pub channel_id: Option<u64>,
    /// Request/response definitions in source order. Always empty for a
    /// datagram channel (datagram channels carry events only).
    pub requests: Vec<Request>,
    /// Event definitions in source order.
    pub events: Vec<Event>,
}

/// Which peer initiates (opens) a [`Channel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelFrom {
    /// The client opens the channel.
    Client,
    /// The server opens the channel.
    Server,
    /// Either peer may open the channel.
    Either,
}

/// How long a [`Channel`] lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelLifetime {
    /// Opened and closed per request.
    Transient,
    /// Held open for the duration of the connection.
    Persistent,
}

/// The wire backend a [`Channel`] runs over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChannelBackend {
    /// QUIC bidirectional stream — ordered and reliable. The default.
    #[default]
    Stream,
    /// QUIC datagram — unordered, unreliable, bounded by the MTU. Requires a
    /// [`Channel::channel_id`].
    Datagram,
}

/// A request/response pair within a [`Channel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Request name (e.g. `"Ping"`).
    pub name: String,
    /// Request payload fields in source order.
    pub fields: Vec<Field>,
    /// The response payload, if the request returns one.
    pub returns: Option<Message>,
}

/// A push event within a [`Channel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Event name (e.g. `"MetricUpdate"`).
    pub name: String,
    /// Event payload fields in source order.
    pub fields: Vec<Field>,
}

/// A named payload message — the `returns` block of a [`Request`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Message name (e.g. `"Pong"`).
    pub name: String,
    /// Payload fields in source order.
    pub fields: Vec<Field>,
}
