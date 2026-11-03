# 03. Fallible glob traversal

> **Status**: Draft — implemented locally; release pending
> **Related**: `mem_1CgfiKufKrUCVb7phQzr1c`, [spec 02](../spec/02-compose-short-references.md)
> **対象**: `src/compose/glob_expand.rs`, `src/compose/mod.rs`, `src/compose/error.rs`, `tests/compose_glob.rs`, `tests/compose_glob_compat.rs`

## Problem and contract

The old composer discarded `glob::GlobError` with `filter_map(Result::ok)` and
discarded metadata failures through `Path::is_file()`. The glob 0.3.3 iterator
also converts some literal-path and directory-classification failures into no
match internally. Propagating its iterator errors alone cannot fix all cases.

The correction applies to existing `(<)glob`. Empty matches and genuinely
absent literal candidates remain successful. Directory search errors, matched
entry metadata errors, dangling symlinks and file read/parse errors abort
composition. Nothing expanded so far is returned as an `Ok` document.

Keep the public function signatures and existing `ComposeError` variants.
Invalid patterns use `Glob` with the importing path. Filesystem failures use
`Io` with the failing path and original `io::Error`. Recursive directory cycles
use `Io` with `InvalidInput`; content include cycles still use `Cycle`.

## Traversal

Retain `glob::Pattern` for validation and matching; own filesystem enumeration
and metadata calls in the private `glob_expand` module. This avoids another
dependency and avoids changing the KDL parser or public API.

- Validate the full pattern, then split its path components while preserving
  platform prefixes, roots, `.` and `..`. The importing directory is a literal
  base, not text to reinterpret as a pattern.
- Literal components use direct metadata lookups, without requiring directory
  listing permission. On `NotFound` / `NotADirectory`, use `symlink_metadata` to
  distinguish an absent candidate from a dangling symlink. Other errors escape.
- Wildcard components enumerate a directory, match names, then inspect matched
  entries. A matched entry disappearing is an error. A directory-entry iteration
  error is reported against its parent directory.
- Use `MatchOptions::new()`; `Default` has different case behavior. Preserve
  case-sensitive matching, hidden names and virtual `.` / `..` matches for
  patterns beginning with a literal dot.
- Handle `**` separately. Consecutive recursive components collapse; a terminal
  `**` matches directories only, which compose excludes. A remaining component
  matches children at each recursive level, including zero directory levels.
- Follow directory symlinks. Track canonical directories only on the active
  branch of the current recursive component. Reset that history when advancing
  to another component. Revisiting the same directory there is a traversal
  cycle; encountering it through a separate alias is not.
- Keep glob's non-UTF8 matching behavior: ordinary wildcard names that are not
  UTF-8 do not match. Recursive traversal can still descend through such a
  directory to UTF-8 names below it. On Windows, retain glob's existing empty
  result for unsupported verbatim prefixes; verbatim drive paths are supported.
- Exclude directories, sort final paths and retain repeated occurrences. Do not
  deduplicate by canonical identity or complete filename extensions.

Traversal uses an explicit pending list rather than recursive Rust calls. Errors
are propagated before a successful match list is returned. Composition then
uses the existing file resolver and shared filter/rename/prefix transformations.

## Verification

The first regressions failed against the old composer for an unreadable
recursive branch, an unsearchable literal parent, a matched dangling symlink,
and a recursive directory symlink cycle. Real permission fixtures restore their
permissions on unwind and verify that access is denied. Unix permission tests
explicitly skip privileged UID 0; they are not claims about Windows ACL tests.

Success coverage includes mixed extensions, file/glob/file relative resolution,
source order, options and surrounding siblings, true empty results, directory
exclusion, parse/read failures and include cycles. Differential tests compare
successful patterns to the existing glob iterator, including recursive/dot
patterns and symlink aliases.

The stricter failure behavior is intentional and belongs in release notes.
The shorthand in spec 02 and the caller-controlled source API are not implemented
by this correction.

## Limits and do not do

- This is error propagation, not a filesystem transaction. Concurrent changes
  can still change what is observed; no snapshot or no-follow guarantee is made.
- Do not suppress metadata failures as “not a file,” or use existence checks to
  silently replace a failed search with zero matches.
- Do not use a global visited set: it would remove legitimate alias imports.
- Do not broaden glob into extension completion or new directive syntax.

## Status log

- 2026-11-03: Added strict traversal after real failure reproduction and RED
  tests. Existing successful matching is covered by regression and differential
  tests; review and publication remain separate gates.
- 2026-11-03: Team-b implementation review found no remaining issues after
  preserving absent absolute-root behavior. Local checks include workspace
  tests and Rust 1.95 MSRV; Linux/Windows-specific execution is a CI gate.
