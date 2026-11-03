# 02. Source-preserving value edits

English | **[日本語](./02-preserve-value-edits.ja.md)**

The `edit` feature updates existing values without rewriting comments, whitespace,
line endings, or other values. It is **unreleased** and is not included in 0.16.0.
To use this checkout from a neighboring project:

```toml
club-kdl = { path = "../club-kdl", features = ["edit"] }
```

## Change a value

```rust
use club_kdl::edit::EditableDocument;

let mut doc = EditableDocument::parse(
    "// connection\nserver \"localhost\" port=8080 // development\n",
)?;
doc.set_property(&[0], "port", 9090)?;
assert_eq!(
    doc.as_str(),
    "// connection\nserver \"localhost\" port=9090 // development\n",
);
```

`parse` owns and validates the source as KDL v2. `as_str()` borrows the current
text; `to_string()` copies it; `into_string()` consumes the editor without a copy.
An unedited document is returned byte for byte, including a BOM or CRLF endings.

`set_argument(&[0], 0, "new host")` changes the first positional argument. Values
implement `ToKdlValue`: strings, numbers, booleans, `Value`, and `Option<T>` are
accepted. Quoting and escaping are automatic. The selected token may be normalized
even if its semantic value is unchanged; other tokens retain their spelling.

## Select the target

| Selection | Meaning |
|---|---|
| `&[0]` | First top-level node |
| `&[1, 0]` | First child of the second top-level node |
| Argument index `1` | Second positional argument; properties do not count |
| Property name `"port"` | Rightmost active `port` property in the selected node |

Indices count only active nodes and entries. `/-` nodes, entries, and child blocks
are preserved but are not editable targets. An empty path or a missing target
returns `EditError` and leaves the document unchanged. To read the current values,
use `Document::parse(doc.as_str())` or `club_kdl::from_str` as usual.

The editor reparses the current text for every update, so consecutive edits can
change token lengths safely. It has no file I/O and does not save automatically.

## Limits

This first version does not insert, remove, or reorder nodes/entries, merge a Rust
struct back into its original source, or update files referenced by compose.
Existing type annotations remain verbatim; schema constraints are not validated.
The ordinary `Document` / struct serializers continue to produce normalized KDL.

See [spec 03](../spec/03-preserve-value-edits.md) and
[design 04](../design/04-preserve-value-edits.md).
