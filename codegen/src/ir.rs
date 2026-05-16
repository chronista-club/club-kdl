//! Schema IR — the intermediate representation between the KDL parser and the
//! language emitters.
//!
//! This is the central contract of `club-kdl-codegen`: the parser produces a
//! [`Schema`], and every [`crate::Emitter`] consumes one. Keeping the IR a
//! plain data structure (no behaviour) lets the parser and emitters evolve
//! independently.
//!
//! Currently this models the **data dialect** (`struct` / `enum` / `field`).
//! The **protocol dialect** (`protocol` / `channel` / `request`) will extend
//! this — see design memory `mem_1Cb5mWnMTdzXfJVoNGFwup`.

/// A whole KDL schema file: an ordered set of type definitions.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Schema {
    /// Type definitions in source order.
    pub types: Vec<TypeDef>,
}

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

/// A field of a [`TypeDef::Struct`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Field name.
    pub name: String,
    /// Field type.
    pub ty: Ty,
    /// Whether the field is required. `false` maps to the target's optional
    /// form (Rust `Option<T>`, TS `?:`, Zod `.optional()`, SurrealQL `option<T>`).
    pub required: bool,
}

/// A field type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    /// A built-in primitive.
    Primitive(Prim),
    /// A homogeneous array of another type.
    Array(Box<Ty>),
    /// A reference to another [`TypeDef`] by name.
    Named(String),
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
