# 02. Implementing short compose references

> **Status**: Draft — implementation plan, not shipped behavior
> **Related**: [spec 02](../spec/02-compose-short-references.md), `mem_1CgfhGM9mjkVTjoUNZfX1E` (decision), `mem_1Cgfgpd9ECRvRMSkf68so2` (prototype findings)
> **対象**: `src/compose/mod.rs`, `src/compose/error.rs`, `tests/compose.rs`

## Approach

Implement spec 02 in the composer above the existing parser. For `> "a"`, the
AST already supplies node name `>` and string argument `a`. No lexer, parser,
writer or KDL grammar extension is required. The composer must explicitly
recognize the new convention; accepting the syntax alone does not import a file.

Normalize the shorthand into the same single-file expansion path as `(<)file`.
Keep the shorthand's extension policy separate from legacy path behavior.

## Recognition and path handling

1. In `parse_directive`, recognize the exact untyped node name `>` as the new
   single-file directive. Keep existing `<` type-annotation dispatch unchanged.
2. Validate the path and options, retaining the original spelling for errors.
3. For the shorthand, append `.kdl` when the path has no extension; otherwise
   retain the supplied path. Complete only the final component, not dots in
   parent directory names. Final edge-case validation is listed in spec 02.
4. Resolve the resulting request from the importing file's directory using the
   existing single-file path flow, or the caller's source when that API exists.
5. Expand recursively, apply the shared filters, renames and namespace prefix
   to the included top-level nodes, then splice the transformed result at the
   directive position. Keep cycle detection based on resolved source identity.

The internal directive representation should distinguish original request,
effective request and origin notation as needed for diagnostics and extension
policy. These are implementation concepts, not a committed public API.

Do not perform an existence check to choose between `a` and `a.kdl`. For
`> "a"`, the effective request is always `a.kdl`. Do not cache away repeated
references: each occurrence still contributes its nodes to the output.

## Existing behavior and API work

`compose(path)` and `from_path<T>(path)` retain their current signatures and
opt-in feature boundary. Normal `Document::parse`, `from_str` and serialization
continue treating `>` as a data node name without file I/O.

The related GFP proposal requires a caller-controlled source to own reference
resolution and actual reads, including root loading and any glob enumeration.
Extension completion can be a deterministic request transformation; it must not
bypass that source by adding hidden filesystem operations. The source owns
access policy and generation consistency. A pre-read canonicalization check
alone is not a no-follow or consistency guarantee.

Dependency reports should distinguish the original request from the resolved
source. Error context should identify the importer and requested target. Exact
public types remain separate design decisions. Strict traversal of existing
filesystem globs is covered by [design 03](03-fallible-glob.md).

## Acceptance work

Before production changes, add failing behavior tests for:

- `> "a"` and `> "./a.kdl"` producing the same document as `(<)file "./a.kdl"`.
- New shorthand options matching the legacy single-file options and order.
- Nested and transitive references resolving from each importer, with source
  order, repeated imports and ordinary sibling nodes preserved.
- `.kdl` completion even when `a` and `a.kdl` both exist; missing `a.kdl` must
  fail rather than falling back to an existing `a`.
- Explicit extensions remaining unchanged, and `(<)file "a"` continuing to
  read the literal extensionless file. Existing glob behavior remains separate.
- `path`, `a`, `"a"` and empty nodes remaining data; only the exact untyped `>`
  name selects the new directive. Parser/writer round trips retain this meaning.
- Missing/unreadable files, parse errors and mixed legacy/shorthand cycles.
- The path and argument edge cases listed in spec 02 once their rules are set.

Run the existing compose tests alongside these regressions. Use independent KDL
parsing to verify the documented notation, and keep published API examples on
implemented behavior until this feature ships.

## Prototype boundary

The temporary experiment used the production `Document::parse` with its own
small resolver. Its bare mode demonstrated why quote-only references fail to
distinguish data. Its marker mode demonstrated `> "a"` expansion and preservation
of empty data nodes. The six checks passed after five expansion/error checks
first failed against the unchanged production composer.

The scratch project at `/tmp/club-kdl-shortest.GzNGmz` is disposable and may
disappear. Its location is not a build dependency or a substitute for checked-in
regressions. It has no legacy options/glob support, custom reader, dependency
tracking, case restriction or generation consistency. No production source was
changed by that experiment.

## Do not do

- Do not infer imports from quotation marks, empty nodes or file existence.
- Do not treat `>` as a new KDL operator or require a custom parser syntax.
- Do not add `(>)path` or `>path` aliases solely because they were discussed.
- Do not change legacy extensionless paths while adding shorthand completion.
- Do not reimplement filtering/renaming separately for the new notation.
- Do not mark the specification Active based on the scratch prototype alone.

## Status log

- 2026-11-03: Drafted composer integration, compatibility boundaries and
  acceptance work for spec 02. Public I/O APIs and production implementation
  remain pending.
- 2026-11-03: Deep review corrected the transform-before-splice order; linked
  the follow-up correction for existing glob traversal errors.
