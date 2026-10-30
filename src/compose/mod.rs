//! Multi-file composition for KDL documents — resolves `(<)file` and
//! `(<)glob` directives into a single composed [`Document`].
//!
//! `club-kdl` itself is a **pure parser** — `from_str` takes a string, returns
//! a value, no IO. This crate adds the **composer** layer on top: it reads
//! files, walks the parsed AST, and splices in the contents of any files
//! referenced by an include directive — including transitively, with cycle
//! detection.
//!
//! ## Why a feature
//!
//! Putting include resolution in the default build would make every
//! `club-kdl` user buy into filesystem IO and the `glob` dependency. Behind
//! the `compose` feature, users who want pure-parser behavior keep it (parse
//! a string, deserialize), and users who want multi-file composition opt in:
//!
//! ```toml
//! club-kdl = { version = "0.15", features = ["compose"] }
//! ```
//!
//! Until 0.13 this lived in the separate `club-kdl-compose` crate
//! (`kdl_compose::compose` is now `club_kdl::compose::compose`).
//!
//! ## Directive syntax — `(<)variant`
//!
//! KDL has no native include directive — the spec deliberately leaves
//! composition to the host language. KDL **type annotations** (tags) are by
//! convention used to type values (`(date)"2026-05-19"`, `(u8)42`), so a word
//! tag like `(include)` would visually compete with type names. This crate
//! uses a **symbol tag** `(<)` instead — directional, single character,
//! unambiguously *not* a type name. The `<` mnemonic is "content flowing
//! into this position from another file".
//!
//! ```kdl
//! // MVP — inline a single file
//! (<)file "./types.kdl"
//!
//! // Glob — inline every matching file
//! (<)glob "./protocols/*.kdl"
//!
//! // Namespace prefix — top-level nodes' first string arg get `shared.`
//! (<)file "./types.kdl" as="shared"
//!
//! // Children-block options — list-valued filters
//! (<)file "./types.kdl" as="shared" {
//!     only "User" "Memory"
//!     except "Internal"
//!     rename "User" "Acct"
//! }
//! ```
//!
//! - **tag** = directive marker (`<` for include; future categories like
//!   overlay / template would pick their own symbol)
//! - **node name** = variant (`file`, `glob`; future `remote` / `module`)
//! - **`as=` property** = single-value namespace prefix
//! - **children block** = list-valued options (`only` / `except` / `rename`)
//!
//! KDL property values are scalars only — array-valued options would not
//! parse, so list options live in a children block instead. The order of
//! transformation is **filter (`only` / `except`) → `rename` → `as=` prefix**.
//!
//! ## What gets composed
//!
//! For each `(<)file/glob` directive node anywhere in the document —
//! top-level or nested inside any block — the directive node is replaced by
//! the composed top-level nodes of the referenced file(s), spliced into the
//! same position. Non-directive nodes are recursively walked: their children
//! are composed too.
//!
//! ## Limits, by design
//!
//! - **Paths are resolved relative to the importing file.** No search paths,
//!   no CWD, no environment lookups. A schema's directory is the only base.
//! - **`as=` renames the first string argument of each top-level included
//!   node only.** Cross-references inside an included file (`type="Foo"` in
//!   a `field`) are not rewritten — `compose` is schema-agnostic. Authors
//!   who use `as=` should keep included files flat (no cross-refs) or qualify
//!   refs themselves.
//! - **No duplicate-name detection.** Two `struct "User"` nodes after
//!   composition are not flagged here — consumers (e.g., codegen) catch
//!   that, since "what counts as a duplicate" is schema-specific.
//!
//! ## Example
//!
//! Each example is an **executable** doc-test — `cargo test --doc` runs it
//! against a real filesystem (under `tempfile::tempdir`), so an API drift in
//! `compose` / `from_path` breaks the docs and the test suite at once.
//!
//! ```
//! # fn main() -> Result<(), club_kdl::compose::ComposeError> {
//! // Lay out a two-file schema under a scratch directory.
//! let dir = tempfile::tempdir().unwrap();
//! std::fs::write(
//!     dir.path().join("types.kdl"),
//!     r#"struct "User" { field "id" type="string" }"#,
//! ).unwrap();
//! std::fs::write(
//!     dir.path().join("schema.kdl"),
//!     "(<)file \"./types.kdl\"\nlocal \"trailing-node\"",
//! ).unwrap();
//!
//! // Resolve all (<) directives, returning a single composed Document.
//! let doc = club_kdl::compose::compose(dir.path().join("schema.kdl"))?;
//! assert_eq!(doc.nodes().len(), 2);                          // struct + local
//! assert_eq!(doc.nodes()[0].name(), "struct");
//! assert_eq!(doc.nodes()[1].name(), "local");
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use std::borrow::Cow;

use crate::{Document, Node, Value};

pub mod error;
pub use error::{ComposeError, Result};

// =============================================================================
// Public API
// =============================================================================

/// Compose a KDL document from `path`, resolving every `(<)` directive
/// recursively. Returns the composed [`Document`] — equivalent to the file
/// the user wrote, but with every include spliced in place.
///
/// Files are read with club-kdl's own KDL v2 parser; the result owns its
/// strings, so it outlives the source files' contents.
///
/// `path` is canonicalized once at entry so cycle detection compares stable
/// identifiers; child paths from `(<)file/glob` are resolved relative
/// to the parent file's directory.
///
/// See the [crate docs](crate) for directive syntax and design limits.
pub fn compose(path: impl AsRef<Path>) -> Result<Document<'static>> {
    let entry = path.as_ref();
    let canonical = canonicalize(entry)?;
    let mut stack = Vec::new();
    let nodes = resolve(&canonical, &mut stack)?;
    Ok(Document::from_nodes(nodes))
}

/// Compose then deserialize via [`crate::from_doc`] — a thin convenience
/// for the common case of "I have a typed schema and a root KDL file".
pub fn from_path<T>(path: impl AsRef<Path>) -> Result<T>
where
    T: for<'de> crate::KdlDeserialize<'de>,
{
    let doc = compose(path)?;
    crate::from_doc(&doc).map_err(|source| ComposeError::Deserialize { source })
}

// =============================================================================
// Resolver — walks one file's nodes, recursing into included files
// =============================================================================

/// Canonicalize a path with a friendly [`ComposeError::Io`] on failure. The
/// resolver needs canonical paths for cycle detection; relative paths cannot
/// be compared reliably.
fn canonicalize(p: &Path) -> Result<PathBuf> {
    std::fs::canonicalize(p).map_err(|source| ComposeError::Io {
        path: p.to_path_buf(),
        source,
    })
}

/// Read, parse, and compose the file at `canonical_path`, returning its
/// composed top-level nodes. `stack` carries the active include chain so the
/// caller can detect cycles.
fn resolve(canonical_path: &Path, stack: &mut Vec<PathBuf>) -> Result<Vec<Node<'static>>> {
    if stack.iter().any(|p| p == canonical_path) {
        let mut cycle = stack.clone();
        cycle.push(canonical_path.to_path_buf());
        return Err(ComposeError::Cycle { stack: cycle });
    }
    stack.push(canonical_path.to_path_buf());

    let text = std::fs::read_to_string(canonical_path).map_err(|source| ComposeError::Io {
        path: canonical_path.to_path_buf(),
        source,
    })?;
    let doc = Document::parse(&text).map_err(|source| ComposeError::Parse {
        path: canonical_path.to_path_buf(),
        source,
    })?;

    let base_dir = canonical_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let nodes = process_nodes(doc.into_nodes(), &base_dir, canonical_path, stack)?;

    stack.pop();
    Ok(nodes)
}

/// Walk `nodes`. For each one, if it is a `(<)` directive, splice in the
/// resolved content; otherwise keep it (taking ownership of its strings) and
/// recurse into its children so nested includes (inside a `channel` or
/// `protocol` block) are resolved too.
fn process_nodes(
    nodes: Vec<Node<'_>>,
    base_dir: &Path,
    current_file: &Path,
    stack: &mut Vec<PathBuf>,
) -> Result<Vec<Node<'static>>> {
    let mut out = Vec::with_capacity(nodes.len());
    for mut node in nodes {
        if let Some(directive) = parse_directive(&node, current_file)? {
            let included = apply_directive(&directive, base_dir, current_file, stack)?;
            out.extend(included);
        } else {
            let children = std::mem::take(node.children_mut());
            let children = process_nodes(children, base_dir, current_file, stack)?;
            let mut owned = node.into_owned();
            *owned.children_mut() = children;
            out.push(owned);
        }
    }
    Ok(out)
}

// =============================================================================
// Directive — parsed (<) node
// =============================================================================

/// A parsed `(<)` directive node, with all its options collected.
#[derive(Debug)]
struct Directive {
    kind: DirectiveKind,
    as_prefix: Option<String>,
    only: Option<Vec<String>>,
    except: Vec<String>,
    rename: BTreeMap<String, String>,
}

/// The variant — `(<)file` or `(<)glob`.
#[derive(Debug)]
enum DirectiveKind {
    /// A single path, relative to the importing file's directory.
    File(PathBuf),
    /// A glob pattern, expanded relative to the importing file's directory.
    Glob(String),
}

/// Return `Some(directive)` if `node` is tagged `(<)`, else `None`. Returns
/// `Err` if the tag is `<` but the node is structurally invalid (unknown
/// variant, missing path, unsupported property, etc.).
fn parse_directive(node: &Node<'_>, current_file: &Path) -> Result<Option<Directive>> {
    let Some(tag) = node.ty() else {
        return Ok(None);
    };
    if tag != "<" {
        return Ok(None);
    }

    let variant = node.name();
    let kind = match variant {
        "file" => {
            let path = first_string_arg(node).ok_or_else(|| ComposeError::InvalidDirective {
                path: current_file.to_path_buf(),
                message: "(<)file requires a string path as its first argument".to_string(),
            })?;
            DirectiveKind::File(PathBuf::from(path))
        }
        "glob" => {
            let pattern = first_string_arg(node).ok_or_else(|| ComposeError::InvalidDirective {
                path: current_file.to_path_buf(),
                message: "(<)glob requires a string pattern as its first argument".to_string(),
            })?;
            DirectiveKind::Glob(pattern.to_string())
        }
        other => {
            return Err(ComposeError::InvalidDirective {
                path: current_file.to_path_buf(),
                message: format!("unknown (<) variant `{other}`; supported variants: file, glob"),
            });
        }
    };

    // Scalar property: as=
    let mut as_prefix = None;
    for entry in node.entries() {
        let Some(key) = entry.name() else { continue };
        match key {
            "as" => {
                let v =
                    entry
                        .value()
                        .as_string()
                        .ok_or_else(|| ComposeError::InvalidDirective {
                            path: current_file.to_path_buf(),
                            message: "as= must be a string".to_string(),
                        })?;
                as_prefix = Some(v.to_string());
            }
            other => {
                return Err(ComposeError::InvalidDirective {
                    path: current_file.to_path_buf(),
                    message: format!(
                        "unknown directive property `{other}`; supported: as. \
                         (list-valued options like only/except/rename go in a children block.)"
                    ),
                });
            }
        }
    }

    // List-valued options in the children block: only, except, rename.
    let (only, except, rename) = parse_options_block(node, current_file)?;

    Ok(Some(Directive {
        kind,
        as_prefix,
        only,
        except,
        rename,
    }))
}

/// Parse `only` / `except` / `rename` children of a directive node.
///
/// - `only "A" "B"` — keep only top-level included nodes whose first string
///   arg matches. Multiple `only` nodes accumulate.
/// - `except "X"` — drop matching nodes. Accumulates similarly.
/// - `rename "Old" "New"` — rename matching first string args. The mapping
///   is many-to-one; later entries override earlier ones for the same key.
#[allow(clippy::type_complexity)]
fn parse_options_block(
    node: &Node<'_>,
    current_file: &Path,
) -> Result<(Option<Vec<String>>, Vec<String>, BTreeMap<String, String>)> {
    let mut only_acc: Option<Vec<String>> = None;
    let mut except = Vec::new();
    let mut rename = BTreeMap::new();

    for child in node.children() {
        match child.name() {
            "only" => {
                let entry_only = only_acc.get_or_insert_with(Vec::new);
                for s in positional_strings(child, "only", current_file)? {
                    entry_only.push(s);
                }
            }
            "except" => {
                for s in positional_strings(child, "except", current_file)? {
                    except.push(s);
                }
            }
            "rename" => {
                let pair = positional_strings(child, "rename", current_file)?;
                if pair.len() != 2 {
                    return Err(ComposeError::InvalidDirective {
                        path: current_file.to_path_buf(),
                        message: format!(
                            "`rename` expects exactly two string arguments \
                             (`rename \"Old\" \"New\"`), got {}",
                            pair.len()
                        ),
                    });
                }
                rename.insert(pair[0].clone(), pair[1].clone());
            }
            other => {
                return Err(ComposeError::InvalidDirective {
                    path: current_file.to_path_buf(),
                    message: format!(
                        "unknown directive option `{other}`; supported: only, except, rename"
                    ),
                });
            }
        }
    }

    Ok((only_acc, except, rename))
}

/// Collect every positional (un-named) string argument of `node` into a Vec.
fn positional_strings(node: &Node<'_>, label: &str, current_file: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for entry in node.entries() {
        if entry.name().is_some() {
            continue;
        }
        let s = entry
            .value()
            .as_string()
            .ok_or_else(|| ComposeError::InvalidDirective {
                path: current_file.to_path_buf(),
                message: format!("`{label}` arguments must be strings"),
            })?;
        out.push(s.to_string());
    }
    Ok(out)
}

// =============================================================================
// Apply a directive — resolve target(s), transform top-level nodes
// =============================================================================

/// Expand the directive's target paths, recursively compose each, and apply
/// `only` / `except` / `rename` / `as=` to the resulting top-level nodes.
fn apply_directive(
    directive: &Directive,
    base_dir: &Path,
    current_file: &Path,
    stack: &mut Vec<PathBuf>,
) -> Result<Vec<Node<'static>>> {
    let paths: Vec<PathBuf> = match &directive.kind {
        DirectiveKind::File(rel) => vec![base_dir.join(rel)],
        DirectiveKind::Glob(pattern) => expand_glob(base_dir, pattern, current_file)?,
    };

    let mut out = Vec::new();
    for path in paths {
        let canonical = canonicalize(&path)?;
        let included_nodes = resolve(&canonical, stack)?;
        out.extend(
            included_nodes
                .into_iter()
                .filter_map(|node| transform_node(node, directive)),
        );
    }
    Ok(out)
}

/// Expand a glob pattern relative to `base_dir`. Matches are sorted for
/// determinism (filesystem iteration order is not portable). A pattern that
/// matches nothing returns an empty vec — empty is not an error here.
fn expand_glob(base_dir: &Path, pattern: &str, current_file: &Path) -> Result<Vec<PathBuf>> {
    let full = base_dir.join(pattern);
    let pat_str = full.to_string_lossy();
    let paths = glob::glob(&pat_str).map_err(|source| ComposeError::Glob {
        path: current_file.to_path_buf(),
        source,
    })?;
    let mut matches: Vec<PathBuf> = paths
        .filter_map(std::result::Result::ok)
        .filter(|p| p.is_file())
        .collect();
    matches.sort();
    Ok(matches)
}

/// Apply this directive's filter / rename / prefix to a single included
/// top-level node. Returns `None` if the node is filtered out.
///
/// Rules apply in order: **filter (`only` / `except`) → `rename` → `as=`
/// prefix.** Lookup keys for filter and rename are the node's first string
/// argument — nodes without one are always kept and never renamed.
fn transform_node<'a>(mut node: Node<'a>, directive: &Directive) -> Option<Node<'a>> {
    let original_name = first_string_arg(&node).map(str::to_string);

    // Filter: only / except — operate on the original (pre-rename) name.
    // Nodes without a first string argument have no name to test against the
    // filter, so they pass through unconditionally — this lets meta directives
    // like `kdl-version 2` survive an `only`/`except` clause that targets
    // sibling type definitions.
    if let Some(only) = &directive.only
        && let Some(n) = &original_name
        && !only.contains(n)
    {
        return None;
    }
    if let Some(n) = &original_name
        && directive.except.contains(n)
    {
        return None;
    }

    // Rename, then prefix. Both touch only the first string arg.
    if let Some(orig) = original_name {
        let renamed = directive.rename.get(&orig).cloned().unwrap_or(orig);
        let final_name = match &directive.as_prefix {
            Some(prefix) => format!("{prefix}.{renamed}"),
            None => renamed,
        };
        set_first_string_arg(&mut node, final_name);
    }
    Some(node)
}

// =============================================================================
// Small helpers on Node
// =============================================================================

/// The first positional (un-named) string argument of `node`, if any.
fn first_string_arg<'n>(node: &'n Node<'_>) -> Option<&'n str> {
    node.arg(0).and_then(Value::as_string)
}

/// Replace the first positional string argument of `node` with `new_value`.
/// No-op when the node has no such argument.
fn set_first_string_arg(node: &mut Node<'_>, new_value: String) {
    for entry in node.entries_mut() {
        if entry.name().is_none() && entry.value().as_string().is_some() {
            *entry = crate::Entry::arg(Value::String(Cow::Owned(new_value)));
            return;
        }
    }
}

// =============================================================================
// Unit tests — exercise the pure helpers and the directive-parsing /
// transformation pipeline in isolation. The integration test suite catches
// the end-to-end IO + path-resolution paths.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Test helpers — keep each test focused on its actual assertion.
    // -------------------------------------------------------------------------

    /// Parse `s` and return its first top-level node, cloned. Lets every test
    /// build a one-line input without doc/index ceremony.
    fn first_node(s: &str) -> Node<'static> {
        let doc = Document::parse(s).expect("kdl parse");
        doc.into_nodes().remove(0).into_owned()
    }

    /// A throwaway path used for the `current_file` field of the error
    /// reporter. Tests only need it to land in error messages; it never has
    /// to exist on disk.
    fn dummy_path() -> &'static Path {
        Path::new("/tmp/dummy.kdl")
    }

    /// Build a [`Directive`] from the four optional dimensions, so transform
    /// tests stay one screen high.
    fn directive(
        only: Option<&[&str]>,
        except: &[&str],
        rename: &[(&str, &str)],
        as_prefix: Option<&str>,
    ) -> Directive {
        Directive {
            kind: DirectiveKind::File(PathBuf::from("test.kdl")),
            as_prefix: as_prefix.map(str::to_string),
            only: only.map(|v| v.iter().map(|s| (*s).to_string()).collect()),
            except: except.iter().map(|s| (*s).to_string()).collect(),
            rename: rename
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        }
    }

    // -------------------------------------------------------------------------
    // first_string_arg — read the leading positional string of a node.
    // -------------------------------------------------------------------------

    #[test]
    fn first_string_arg_skips_properties() {
        let node = first_node(r#"node prop="ignored" "the-arg""#);
        assert_eq!(first_string_arg(&node), Some("the-arg"));
    }

    #[test]
    fn first_string_arg_returns_none_when_no_positional_arg() {
        let node = first_node(r#"node key="value""#);
        assert_eq!(first_string_arg(&node), None);
    }

    #[test]
    fn first_string_arg_returns_none_when_positional_arg_is_integer() {
        // `kdl-version 2`-style nodes — positional but not a string.
        let node = first_node(r#"node 42"#);
        assert_eq!(first_string_arg(&node), None);
    }

    // -------------------------------------------------------------------------
    // set_first_string_arg — rewrite the leading positional string in place.
    // -------------------------------------------------------------------------

    #[test]
    fn set_first_string_arg_replaces_in_place() {
        let mut node = first_node(r#"struct "User" { field "id" type="string" }"#);
        let node = &mut node;
        set_first_string_arg(node, "Renamed".to_string());
        assert_eq!(first_string_arg(node), Some("Renamed"));
        // Children are untouched — only the first string arg of `struct`.
        let child = &node.children()[0];
        assert_eq!(first_string_arg(child), Some("id"));
    }

    #[test]
    fn set_first_string_arg_noop_when_no_string_positional_arg() {
        // `kdl-version 2` has only an integer arg; nothing to rename.
        let mut node = first_node(r#"node 42 key="value""#);
        set_first_string_arg(&mut node, "wont-apply".to_string());
        // Integer arg stays an integer (not silently converted to a string).
        assert!(matches!(node.entries()[0].value(), Value::Integer(_)));
    }

    // -------------------------------------------------------------------------
    // parse_directive — recognize a `(<)` directive and reject malformed ones.
    // -------------------------------------------------------------------------

    #[test]
    fn parse_directive_returns_none_for_untagged_node() {
        let node = first_node(r#"file "./x.kdl""#);
        assert!(parse_directive(&node, dummy_path()).unwrap().is_none());
    }

    #[test]
    fn parse_directive_returns_none_for_non_lt_tag() {
        // `(date)` is the canonical "this is a type" tag — must not be
        // mistaken for a directive.
        let node = first_node(r#"(date)"2026-05-19""#);
        assert!(parse_directive(&node, dummy_path()).unwrap().is_none());
    }

    #[test]
    fn parse_directive_rejects_unknown_variant() {
        let node = first_node(r#"(<)wat "./x.kdl""#);
        let err = parse_directive(&node, dummy_path()).unwrap_err();
        let ComposeError::InvalidDirective { message, .. } = err else {
            panic!("expected InvalidDirective");
        };
        assert!(message.contains("unknown (<) variant"));
        assert!(message.contains("wat"));
    }

    #[test]
    fn parse_directive_file_variant_extracts_path() {
        let node = first_node(r#"(<)file "./types.kdl""#);
        let d = parse_directive(&node, dummy_path())
            .unwrap()
            .expect("directive");
        let DirectiveKind::File(p) = d.kind else {
            panic!("expected File");
        };
        assert_eq!(p, PathBuf::from("./types.kdl"));
    }

    #[test]
    fn parse_directive_glob_variant_extracts_pattern() {
        let node = first_node(r#"(<)glob "./types/*.kdl""#);
        let d = parse_directive(&node, dummy_path())
            .unwrap()
            .expect("directive");
        let DirectiveKind::Glob(p) = d.kind else {
            panic!("expected Glob");
        };
        assert_eq!(p, "./types/*.kdl");
    }

    #[test]
    fn parse_directive_rejects_file_without_path_arg() {
        let node = first_node(r#"(<)file"#);
        let err = parse_directive(&node, dummy_path()).unwrap_err();
        assert!(matches!(err, ComposeError::InvalidDirective { .. }));
    }

    #[test]
    fn parse_directive_rejects_glob_without_pattern_arg() {
        let node = first_node(r#"(<)glob"#);
        let err = parse_directive(&node, dummy_path()).unwrap_err();
        assert!(matches!(err, ComposeError::InvalidDirective { .. }));
    }

    #[test]
    fn parse_directive_extracts_as_prefix() {
        let node = first_node(r#"(<)file "./x.kdl" as="shared""#);
        let d = parse_directive(&node, dummy_path())
            .unwrap()
            .expect("directive");
        assert_eq!(d.as_prefix.as_deref(), Some("shared"));
    }

    #[test]
    fn parse_directive_rejects_non_string_as_value() {
        let node = first_node(r#"(<)file "./x.kdl" as=42"#);
        let err = parse_directive(&node, dummy_path()).unwrap_err();
        let ComposeError::InvalidDirective { message, .. } = err else {
            panic!("expected InvalidDirective");
        };
        assert!(message.contains("as= must be a string"));
    }

    #[test]
    fn parse_directive_rejects_unknown_property() {
        let node = first_node(r#"(<)file "./x.kdl" wat="x""#);
        let err = parse_directive(&node, dummy_path()).unwrap_err();
        let ComposeError::InvalidDirective { message, .. } = err else {
            panic!("expected InvalidDirective");
        };
        assert!(message.contains("unknown directive property"));
        assert!(message.contains("wat"));
    }

    // -------------------------------------------------------------------------
    // parse_options_block — list-valued options live in a children block.
    //
    // Reached through `parse_directive` since `parse_options_block` is only
    // called once and is not public to the test module's grandparent. These
    // tests use directive nodes with children blocks.
    // -------------------------------------------------------------------------

    fn parse_with_block(node_text: &str) -> Directive {
        let node = first_node(node_text);
        parse_directive(&node, dummy_path())
            .unwrap()
            .expect("directive")
    }

    #[test]
    fn parse_options_block_empty_node_returns_defaults() {
        let d = parse_with_block(r#"(<)file "./x.kdl""#);
        assert!(d.only.is_none());
        assert!(d.except.is_empty());
        assert!(d.rename.is_empty());
    }

    #[test]
    fn parse_options_block_only_single_line_collects_names() {
        let d = parse_with_block(
            r#"(<)file "./x.kdl" {
                only "A" "B"
            }"#,
        );
        assert_eq!(d.only.unwrap(), vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn parse_options_block_only_multi_line_accumulates() {
        // Two `only` nodes — the second should not overwrite the first.
        let d = parse_with_block(
            r#"(<)file "./x.kdl" {
                only "A"
                only "B"
            }"#,
        );
        assert_eq!(
            d.only.unwrap(),
            vec!["A".to_string(), "B".to_string()],
            "multiple `only` nodes must accumulate, not overwrite"
        );
    }

    #[test]
    fn parse_options_block_except_collects_names() {
        let d = parse_with_block(
            r#"(<)file "./x.kdl" {
                except "A" "B"
            }"#,
        );
        assert_eq!(d.except, vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn parse_options_block_rename_single_entry() {
        let d = parse_with_block(
            r#"(<)file "./x.kdl" {
                rename "Old" "New"
            }"#,
        );
        assert_eq!(d.rename.get("Old"), Some(&"New".to_string()));
    }

    #[test]
    fn parse_options_block_rename_same_key_later_wins() {
        // Two `rename` entries for the same key — later overrides earlier.
        let d = parse_with_block(
            r#"(<)file "./x.kdl" {
                rename "User" "First"
                rename "User" "Second"
            }"#,
        );
        assert_eq!(
            d.rename.get("User"),
            Some(&"Second".to_string()),
            "later rename entry must override earlier for the same key"
        );
    }

    #[test]
    fn parse_options_block_rename_wrong_arity_is_rejected() {
        let node = first_node(
            r#"(<)file "./x.kdl" {
                rename "OnlyOne"
            }"#,
        );
        let err = parse_directive(&node, dummy_path()).unwrap_err();
        let ComposeError::InvalidDirective { message, .. } = err else {
            panic!("expected InvalidDirective");
        };
        assert!(message.contains("rename"));
        assert!(message.contains("exactly two"));
    }

    #[test]
    fn parse_options_block_unknown_option_is_rejected() {
        let node = first_node(
            r#"(<)file "./x.kdl" {
                weird "x"
            }"#,
        );
        let err = parse_directive(&node, dummy_path()).unwrap_err();
        let ComposeError::InvalidDirective { message, .. } = err else {
            panic!("expected InvalidDirective");
        };
        assert!(message.contains("unknown directive option"));
        assert!(message.contains("weird"));
    }

    #[test]
    fn parse_options_block_non_string_arg_is_rejected() {
        let node = first_node(
            r#"(<)file "./x.kdl" {
                only 42
            }"#,
        );
        let err = parse_directive(&node, dummy_path()).unwrap_err();
        let ComposeError::InvalidDirective { message, .. } = err else {
            panic!("expected InvalidDirective");
        };
        assert!(message.contains("only"));
        assert!(message.contains("string"));
    }

    // -------------------------------------------------------------------------
    // transform_node — apply order: filter (only/except) → rename → as= prefix.
    // -------------------------------------------------------------------------

    #[test]
    fn transform_node_no_options_returns_node_unchanged() {
        let node = first_node(r#"struct "User""#);
        let d = directive(None, &[], &[], None);
        let out = transform_node(node.clone(), &d).expect("kept");
        assert_eq!(first_string_arg(&out), Some("User"));
    }

    #[test]
    fn transform_node_only_keeps_matching_node() {
        let node = first_node(r#"struct "User""#);
        let d = directive(Some(&["User"]), &[], &[], None);
        assert!(transform_node(node.clone(), &d).is_some());
    }

    #[test]
    fn transform_node_only_drops_non_matching_node() {
        let node = first_node(r#"struct "Other""#);
        let d = directive(Some(&["User"]), &[], &[], None);
        assert!(transform_node(node.clone(), &d).is_none());
    }

    #[test]
    fn transform_node_only_keeps_no_first_string_arg_node() {
        // `kdl-version 2` has no string positional arg → kept regardless of
        // the `only` list. Documented as "nodes without one are always kept".
        let node = first_node(r#"kdl-version 2"#);
        let d = directive(Some(&["User"]), &[], &[], None);
        let out = transform_node(node.clone(), &d).expect("kept despite only filter");
        assert!(matches!(out.entries()[0].value(), Value::Integer(_)));
    }

    #[test]
    fn transform_node_except_drops_matching_node() {
        let node = first_node(r#"struct "Internal""#);
        let d = directive(None, &["Internal"], &[], None);
        assert!(transform_node(node.clone(), &d).is_none());
    }

    #[test]
    fn transform_node_except_keeps_no_first_string_arg_node() {
        // Symmetric to the `only` case — first-string-argless nodes are
        // also never dropped by `except`.
        let node = first_node(r#"kdl-version 2"#);
        let d = directive(None, &["Internal"], &[], None);
        let out = transform_node(node.clone(), &d).expect("kept");
        assert!(matches!(out.entries()[0].value(), Value::Integer(_)));
    }

    #[test]
    fn transform_node_rename_replaces_first_string_arg() {
        let node = first_node(r#"struct "User""#);
        let d = directive(None, &[], &[("User", "Acct")], None);
        let out = transform_node(node.clone(), &d).expect("kept");
        assert_eq!(first_string_arg(&out), Some("Acct"));
    }

    #[test]
    fn transform_node_rename_for_unknown_key_is_silent_noop() {
        let node = first_node(r#"struct "User""#);
        let d = directive(None, &[], &[("Other", "Acct")], None);
        let out = transform_node(node.clone(), &d).expect("kept");
        assert_eq!(
            first_string_arg(&out),
            Some("User"),
            "non-matching rename leaves node intact"
        );
    }

    #[test]
    fn transform_node_as_prefix_prepends_dot_separated() {
        let node = first_node(r#"struct "User""#);
        let d = directive(None, &[], &[], Some("shared"));
        let out = transform_node(node.clone(), &d).expect("kept");
        assert_eq!(first_string_arg(&out), Some("shared.User"));
    }

    #[test]
    fn transform_node_apply_order_is_filter_then_rename_then_prefix() {
        // only=[User] (pre-rename) → keep
        // rename User→Acct           → first string arg becomes Acct
        // as="ns"                    → prefix to ns.Acct
        let node = first_node(r#"struct "User""#);
        let d = directive(Some(&["User"]), &[], &[("User", "Acct")], Some("ns"));
        let out = transform_node(node.clone(), &d).expect("kept");
        assert_eq!(first_string_arg(&out), Some("ns.Acct"));
    }

    #[test]
    fn transform_node_only_matches_against_original_name_not_renamed_name() {
        // `only=["Acct"]` referring to the *renamed* name must NOT match —
        // filter runs first, against the original.
        let node = first_node(r#"struct "User""#);
        let d = directive(Some(&["Acct"]), &[], &[("User", "Acct")], None);
        assert!(
            transform_node(node.clone(), &d).is_none(),
            "only matches pre-rename names, so `Acct` should not match `User`"
        );
    }

    #[test]
    fn transform_node_as_prefix_skips_node_with_no_first_string_arg() {
        // `as=` only rewrites the first *string* arg; integer-only nodes
        // pass through unchanged.
        let node = first_node(r#"kdl-version 2"#);
        let d = directive(None, &[], &[], Some("shared"));
        let out = transform_node(node.clone(), &d).expect("kept");
        assert!(matches!(out.entries()[0].value(), Value::Integer(_)));
    }
}
