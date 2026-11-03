# 02. Short file references in compose

> **Status**: Draft — notation selected; production implementation pending
> **Related**: [design 02](../design/02-compose-short-references.md), `mem_1CgfhGM9mjkVTjoUNZfX1E` (decision), `mem_1Cgfgpd9ECRvRMSkf68so2` (prototype findings)
> **対象**: `src/compose/mod.rs`, `src/compose/error.rs`, `tests/compose.rs`

## Purpose

Use `> "a"` as the primary notation for importing a single KDL file. The `>`
marker makes the import visible while keeping the expression short. When the
reference has no extension, append `.kdl`; resolve relative references from the
importing file's directory.

This is a convention implemented by the opt-in composer, not a new KDL grammar
feature. `>` is a node name, not a type annotation or a KDL operator. Ordinary
parsing and deserialization do not read other files. See the KDL specification
for [nodes](https://kdl.dev/spec/#name-node),
[identifier strings](https://kdl.dev/spec/#name-identifier-string), and
[type annotations](https://kdl.dev/spec/#name-type-annotation).

## Intended usage

Given these files:

```text
case/
  plan.kdl
  共通仮設.kdl
  内装.kdl
```

`plan.kdl` can contain:

```kdl
estimate "案A" {
    > "共通仮設"
    > "内装"
}
```

Each reference is replaced at its position by the referenced file's composed
top-level nodes. The surrounding `estimate` remains. References inside imported
files resolve relative to those files, not to `plan.kdl` or the process CWD.

| Reference in `case/plan.kdl` | Requested file |
|---|---|
| `> "a"` | `case/a.kdl` |
| `> "./a"` | `case/a.kdl` |
| `> "parts/a"` | `case/parts/a.kdl` |
| `> "./a.kdl"` | `case/a.kdl` |
| `> "a.data"` | `case/a.data` (still parsed as KDL) |

Extension completion is deterministic. `> "a"` requests `a.kdl` even when an
extensionless file named `a` also exists. There is no filesystem-dependent
fallback, search path, environment expansion, or automatic directory import.

## Requirements

- Recognize an untyped node whose name is exactly `>` as a single-file reference.
  Its path is a string argument. A missing or non-string path is an error.
- Apply extension completion to the new shorthand only. A supplied extension
  remains unchanged; it is never replaced with `.kdl`.
- Support top-level and nested references, preserving splice position and source
  order. Referencing the same file twice expands it twice; an active recursive
  cycle is an error.
- Share the existing single-file expansion and option semantics: `only` /
  `except`, then `rename`, then `as`. These transform the first string argument
  of included top-level nodes, not arbitrary internal cross-references.
- Preserve the existing `(<)file` and successful `(<)glob` behavior, including
  literal path/pattern handling. Do not apply implicit `.kdl` completion to
  either. Glob failure handling is clarified in [design 03](../design/03-fallible-glob.md).
- Keep ordinary `path "./output.kdl"`, `a`, `"a"`, and `enabled` as data.
  Import recognition must not depend on whether a similarly named file exists.
- Propagate single-file resolution/read/parse errors and include cycles instead
  of returning the nodes expanded so far. Glob traversal failures must also
  propagate once the separate [glob correction](../design/03-fallible-glob.md)
  is in use. This is not an atomic filesystem snapshot guarantee.

For example, the following keeps the output path as data and imports `a.kdl`:

```kdl
export {
    path "./output.kdl"
    > "a"
}
```

## Compatibility and alternatives

`(<)file` and `(<)glob` remain supported. The earlier `(>)path` and `>path`
proposals are design history, not additional aliases required by this spec;
neither was implemented in the production composer during this discussion.

Bare `"a"` was tried and is not selected. The current AST treats `"a"` and `a`
identically, so interpreting an empty node as a reference also consumes ordinary
data such as `enabled`. The explicit `>` marker avoids that ambiguity.

The new convention still reserves a name within compose: a document previously
using an untyped `>` node as data will change meaning when processed by the new
composer. This must be called out in release notes. Pure parser behavior is
unchanged. Quotes are recommended for readable paths, but do not act as the
import marker; recognition follows the parsed node name and string value.

The earlier `<` / `>` discussion expressed “content intake” / “reference
resolution,” not a strict raw-versus-interpreted distinction. Existing `<`
directives already perform recursive interpretation and transformations.

## Scope and remaining decisions

This document selects the shorthand and extension-completion rule. It does not
add directory imports, new glob syntax, expression evaluation, automatic
writeback, transactions, or file watching.

Caller-controlled loading and dependency reports remain related design work.
Their public Rust API is not settled here. The existing glob error correction
is tracked in [design 03](../design/03-fallible-glob.md). GFP owns
case access restrictions, no-follow I/O and consistent storage generations.

Before implementation, specify validation for empty paths, directory-like
references, dotfiles, trailing dots and extra or annotated arguments. Define
platform-specific path behavior explicitly rather than introducing probe-order
fallbacks. These edge cases were not settled by the notation experiment.

## Evidence and status

An isolated prototype expanded `"a"` and `> "a"` into a real `a.kdl` file's
nodes. Six checks covered nested expansion/order, empty-node collision and
preservation, quote equivalence, missing files and cycles. Only the explicit
marker variant is selected. This does not establish production support for
legacy directives, options, glob, custom I/O or dependency reports.

## Status log

- 2026-11-03: Documented the selected `> "a"` notation and implicit `.kdl`
  completion after the prototype comparison. Production implementation pending.
- 2026-11-03: Deep review clarified the error guarantee. The follow-up glob fix
  is separate from shorthand implementation; successful legacy matching remains.
