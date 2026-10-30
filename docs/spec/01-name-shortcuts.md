# 01. Opt-in name shortcuts

> **Status**: Active (v0.15.0)
> **Related**: `mem_1CgYBNbHDqGGdtK66QsMCL`, [design 01](../design/01-name-shortcuts.md)
> **対象**: `src/compact.rs`, `tests/compact.rs`

## Purpose

Reduce repeated node/property names in machine-oriented KDL messages and stored
documents. Human-edited configuration and existing serialization APIs keep
their readable names. The representation must be ordinary KDL v2, readable by
independent parsers. This is a vocabulary convention above KDL, not new syntax.

## Requirements

- An explicit `compact` feature and API opt into shortening. Existing derives,
  normal readers/writers and their output remain unchanged.
- Only node names and property keys change. Values, type annotations, positional
  arguments, order, repeated properties and document roots retain their meaning.
- Compact tables, messages and files omit structural indentation. Newlines and
  braces retain node boundaries and hierarchy. Whitespace within string values
  is preserved; ordinary writer output remains indented. Readers accept both
  indented and unindented KDL without changing the format version or table ID.
- Allocate deterministic ASCII letter aliases, shortest first, extending to two
  and more characters as needed. Reserve all original names against collisions.
  Only rename names whose serialized spelling occupies at least three bytes and
  for which the alias is strictly shorter. Size is measured in UTF-8 bytes.
- A reusable name table supports independently received messages. Missing names,
  unknown aliases and the wrong table fail explicitly instead of falling through
  to a potentially different field. Tables are immutable; callers create a new
  table when their vocabulary changes.
- Stored compact documents carry their table and can be restored without a
  previous connection or external registry. They may grow for small inputs;
  compact output is an explicit choice, not a guarantee of smaller files.
- Both tables and messages have a format version. Unsupported versions and
  malformed or ambiguous table entries are rejected.
- Table identity detects accidental mismatches; it is not message authentication.
- Validate real object round trips, arbitrary document semantics, independent
  KDL parsing, collisions/exhaustion, malformed input, and default API stability.
- Benchmark the implemented encode/decode paths including table handling and
  framing. Distinguish byte savings from CPU results and the fixed-name baseline.

## Not included

Value/type-name compression, config migration, transport/session management,
schema negotiation, encryption/authentication, compression codecs, file IO,
automatic growth of live dictionaries, original formatting preservation, and
single-line/minimal KDL output.

## Status log

- 2026-10-30: v0.15.0 includes deterministic aliases and indentation-free compact
  output. Original idea: `mem_1CgYEeWUHuLPHVaFWe6z71`; implementation/release:
  `mem_1CgYGA5N9F8xynDW6t9N2A`.
