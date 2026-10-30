# 01. Compact vocabulary codec

> **Status**: Draft
> **Related**: [spec 01](../spec/01-name-shortcuts.md), `mem_1CgYBNbHDqGGdtK66QsMCL`
> **対象**: `src/compact.rs`, `tests/compact.rs`, `benches/name_shortcuts.rs`

The opt-in `compact` module sits above the existing KDL tree, parser and writer.
It uses existing `KdlSerialize::to_kdl_doc` and `KdlDeserialize::from_kdl_doc`
without changing derive behavior or normal output. Correctness and an explicit
format come first; tree construction and dictionary lookup are measured costs.

## Name table

`NameTable` owns bidirectional ordered mappings for the complete allowed node/key
vocabulary, including unchanged names. Constructors accept names, a document, or
a serializable value. Names are deduplicated and sorted by original UTF-8 byte
order. This makes assignment independent of traversal order and repetition.

Aliases enumerate `a..z`, `A..Z`, `aa..az`, `aA..aZ`, `ba..`, etc., skipping KDL
keywords that would require quotes (e.g. `inf`, `nan`). Reserve the
entire original vocabulary, then allocate aliases only when the KDL serialized
spelling is at least three bytes and is longer than the alias. A generated alias
never equals any original name. Identity mappings preserve short names. Imported
tables must be bijective and must reject aliases that shadow another original.

Canonical export uses the existing writer with entries sorted by original name:

```kdl
kdl-names 1 {
    a object_identifier
    b record_update
    x x
}
```

Each child is a wire name followed by exactly one untyped string original name.
All names, even numeric or unusual ones, use KDL string escaping. The header has
one untyped integer version, no properties or type annotation. Table children
have no properties, annotations or grandchildren. Duplicate wire or original
names are errors. The table ID is lowercase hex SHA-256 of the bytes
`club-kdl:compact:1\0`, followed by each original/alias pair in original-name
UTF-8 byte order. Each string is prefixed by its UTF-8 byte length as an unsigned
64-bit big-endian integer. This is independent of KDL whitespace and quoting;
the dictionary hash input is internal, not a non-KDL wire format.
`sha2` is enabled only by `compact`.

## Messages and files

Shared-table messages contain a header then payload nodes:

```kdl
kdl-message 1 "<64-character table ID>"
b a=42 x=10
```

The message header has exactly two untyped arguments (integer version, string
table ID), no children/properties/annotation. The receiver checks version and
table identity before resolving names. Only the first node is protocol metadata;
payload nodes may use any original name, including protocol-like names.

Self-contained files contain a table followed by payload nodes, with no redundant
message header or table hash. `compact::to_string/from_str` handle objects;
`compact::encode_document/decode_document` handle arbitrary documents.
`NameTable::encode/decode` and `encode_document/decode_document` reuse a table.
Table-only KDL is exported/imported with `to_kdl/from_kdl`.

The implementation consumes temporary trees and replaces only node/key `Cow`
strings. Shared decode can borrow expanded names from the table and values from
the input. Self-contained document decode owns its result because its temporary
table goes out of scope. Errors are `CompactError`, wrapping ordinary KDL errors
and distinguishing invalid tables/envelopes, versions, mismatches and unknown
names. No partial result is returned. Convenience APIs allocate a fresh string.

## Do not do

- Do not shorten values or type annotations by string substitution.
- Do not interpret normal KDL as a compact envelope without an explicit API call.
- Do not pass unknown symbols through: a future original name could be an alias.
- Do not infer a peer's mapping from message order or treat the digest as a MAC.
- Do not claim the fixed-name benchmark predicts runtime dictionary performance.
- Do not count packet framing and table exchange as free in size comparisons.
- Do not measure aliases by raw length when they need KDL quotes; skip keywords.
- Do not hash writer layout: use the specified versioned, length-prefixed pairs.

## Status log

- 2026-10-30: Draft after benchmark and implementation GO. Explicit ordinary vs
  compact output, shared message tables and self-contained file envelopes chosen.
- 2026-10-30: Implemented the opt-in AST codec and observed RED → GREEN for
  storage, messaging, malformed dictionaries and collision cases. Keyword aliases
  and layout-dependent IDs were caught by failing tests and corrected. Runtime
  benchmark shows byte savings with CPU overhead; no speedup is claimed.
