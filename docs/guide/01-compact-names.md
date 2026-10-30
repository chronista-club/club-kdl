# 01. Rule-based automatic aliases

The `compact` feature shortens node names and property keys for machine-oriented
storage and messages. It uses standard KDL v2 strings and syntax. Keep ordinary
`club_kdl::to_string/from_str` for readable configuration; they are unchanged.

This feature is currently unreleased. To try this checkout:

```toml
[dependencies]
club-kdl = { path = "../club-kdl", features = ["compact"] }
```

## Self-contained files

The same derives work with both representations:

```rust
use club_kdl::{KdlDeserialize, KdlSerialize, compact};

#[derive(Debug, PartialEq, KdlSerialize, KdlDeserialize)]
#[kdl(name = "position_update")]
struct Update {
    #[kdl(property)]
    object_identifier: u64,
}

let update = Update { object_identifier: 42 };
let readable = club_kdl::to_string(&update)?;
let stored = compact::to_string(&update)?;
let restored: Update = compact::from_str(&stored)?;
assert_eq!(restored, update);
```

`stored` starts with `kdl-names 1 { ... }`, followed by the aliased document.
The table is embedded, so a later reader needs no earlier session. The API returns
text; use your application's normal file IO to store it. Small documents may
grow because of the table. This is not a general compression codec.

For arbitrary documents, use `compact::encode_document(doc)` and
`compact::decode_document(&text)`. Decode returns an owned tree with original
names, suitable for `doc.to_string()` to recover readable KDL. Like the normal
tree APIs, comments/layout are not preserved.

## Reusing a table across messages

```rust
let sender = compact::NameTable::from_value(&update)?;
let table_text = sender.to_kdl(); // send/store once, outside the message stream
let receiver = compact::NameTable::from_kdl(&table_text)?;

let message = sender.encode(&update)?;
let restored: Update = receiver.decode(&message)?;
```

Each message starts with `kdl-message 1 <table-id>`. The receiving table checks
the version and ID before expanding any payload names. Its SHA-256 ID describes
the complete mapping; it does not authenticate messages or identify field types.
Transport negotiation, framing and mapping lifetime belong to the application.

Build a table with the **complete vocabulary**. `from_value` only observes names
actually present in that value, so absent optional children and future map keys
will not be included. Supply their union explicitly when needed:

```rust
let table = compact::NameTable::from_names([
    "position_update", "object_identifier", "optional_child", "dynamic_key",
]);
```

Tables are immutable. If vocabulary changes, create and exchange a new table.
Encoding an undeclared original name or decoding an unknown alias fails rather
than guessing. `encode_document(doc)` / `decode_document(&message)` on a table
provide the equivalent tree APIs; shared decode may borrow from the table and
input. Call `into_owned()` if the document must outlive them.

## Rules and errors

Names are sorted and deduplicated, then assigned ASCII aliases `a..z`, `A..Z`,
`aa`, and so on, skipping original names and KDL keywords requiring quotes.
Only names at least three serialized UTF-8 bytes long are candidates, and only
strictly shorter aliases are used. Short names remain in the table as identity
mappings. Only names change; values and type annotations are untouched.

The APIs return `compact::CompactError`:

- `UnknownName`: add the missing name to a new table and exchange that table.
- `UnknownAlias`: the payload contains a symbol the table does not define.
- `TableMismatch`: select the matching table; never guess or decode with another.
- `UnsupportedVersion`: the reader does not support that protocol version.
- `InvalidTable` / `InvalidEnvelope`: malformed metadata; reject it.
- `Kdl`: underlying KDL syntax or object conversion failed.

Table metadata is strict about duplicate mappings, extra arguments/properties,
annotations and nested entries. Imported aliases can be quoted strings such as
`"0"`; the automatic allocator uses letters to avoid quote overhead.

See [design 01](../design/01-name-shortcuts.md) for the protocol and
[the benchmark report](../benchmarks/name-shortcuts.md) for sizes and costs.
Runtime tree construction and lookup can cost more CPU than the normal direct
writer, even when the resulting payload is smaller.
