# 03. Source-preserving value edits

English | **[日本語](./03-preserve-value-edits.ja.md)**

> **Status**: Draft
> **Related**: design 04, `mem_1CggW2QewS8jT8ejr6jRC6`
> **Target**: `src/edit.rs`, `tests/preserve_edits.rs`

## Purpose

Allow an application to change an existing KDL value without losing the user's
comments or rewriting the surrounding document. This is an optional editing API;
the existing semantic tree and normalized serializers retain their behavior.

## Scope and requirements

- An `edit` Cargo feature exposes `edit::EditableDocument`, which owns the source
  text and validates it as KDL v2. No new dependencies are required.
- Reading and writing an unedited document preserves every input byte, including
  comments, BOM, line endings, whitespace, and value spellings.
- A nonempty slice of zero-based node indices selects a node, descending into
  children at each step. Only active nodes count; slashdashed nodes do not.
- `set_argument(path, index, value)` replaces an existing positional argument;
  its index counts arguments, not properties. `set_property(path, name, value)`
  replaces the rightmost active property with that name, matching normal lookup.
- Only the selected value token changes. Property keys, type annotations, and
  all surrounding bytes remain intact. The replacement uses the existing KDL v2
  value writer, including quoting and escaping. Setting the same semantic value
  may normalize that token's spelling. Type annotations are not schema-validated.
- A missing node, argument, or property is an error. Empty node paths are invalid.
  Failed edits leave the text unchanged. Successful edits can be followed by more
  edits, regardless of changes in byte length.
- Slashdashed entries and child blocks remain verbatim and are never targets.

Node/entry insertion, deletion, movement, struct-to-source merging, multi-file
composition writeback, and file I/O are outside this first version. Editing scans
the current document on each call; this is not an incremental editor.

## Verification

Exact text comparisons cover comments, typed values, UTF-8, raw/multiline strings,
CRLF, slashdash, duplicate names/properties, sequential edits, and failed edits.
The KDL specification corpus checks unchanged text, targeted replacement, and
semantic reparsing. Existing parser/writer tests and before/after benchmarks
check the normal path.
