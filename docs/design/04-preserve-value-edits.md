# 04. Source-preserving value edits

English | **[日本語](./04-preserve-value-edits.ja.md)**

> **Status**: Draft
> **Related**: spec 03, `mem_1CggW2QewS8jT8ejr6jRC6`
> **Target**: `src/edit.rs`, `src/parse.rs`

## Architecture

`EditableDocument` owns a `String`. Parsing validates the original input with the
existing KDL v2 engine. Rendering returns the retained text rather than formatting
a semantic tree. The public lightweight AST gains no formatting fields.

For each edit, a parser specialization records byte ranges of active entry values
in document order. Node and entry selection uses the parsed semantic tree; its
depth-first entry order matches the range list. The editor serializes a new value
with the existing writer and replaces exactly its range in the owned source.
The next edit reparses the new text, avoiding stale offsets and self-references.

The normal parser specializes range recording off at compile time. The editing
module and recording entry point are behind `edit`; no dependency is added.

## Source ranges

- Start a value range after the property key, `=`, and any type annotation and
  intervening whitespace/comments. End at the value token's end.
- For an untyped string argument, include its original quoting/raw delimiters.
- Discard captured ranges when a slashdashed node or child block is discarded.
  Slashdashed entries do not enter the range list.
- Resolve duplicate properties from the right, like `Node::prop`.

## Constraints

Do not regex-match values, serialize the entire tree, attach comment fields to the
default AST, or use offsets from an earlier edit. Do not infer edits in referenced
files or automatically create a missing target. Indices refer to active nodes and
arguments only. Replacement tokens may be normalized; untouched tokens must not be.

## Status log

- 2026-11-04: Minimal scope approved. Initial regression demonstrates that the
  current semantic-tree round trip loses comments and source formatting.
- 2026-11-04: Value-only replacement implemented. Eight editing tests failed on
  the unimplemented setters before passing; the three validation/retention tests
  also pass. The corpus checks 233 valid documents, 87 invalid documents, and
  198 addressable values. Workspace tests, Clippy, rustdoc, doctests, and MSRV
  1.95 checks pass.

## Normal-path performance check

On Apple Silicon with Rust 1.96.0, compare the existing `parse/club-kdl` benchmarks
before the parser change and after it, with a 1-second warm-up, 3-second measurement,
and 50 samples per case:

```sh
cargo bench --bench kdl_vs_json -- 'parse/club-kdl' --warm-up-time 1 --measurement-time 3 --sample-size 50 --save-baseline before-edit
cargo bench --bench kdl_vs_json -- 'parse/club-kdl' --warm-up-time 1 --measurement-time 3 --sample-size 50 --baseline before-edit
```

Criterion's mean-time change estimates were +0.17% for the small input
(95% interval -0.28% to +0.57%, p=0.45) and -0.39% for the large input
(-0.95% to +0.17%, p=0.18). Neither detected a significant change. This is a local
check of `Document::parse`, not a zero-cost guarantee or an editor benchmark.
