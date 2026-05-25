//! Multi-file composition for KDL documents — resolves `(<)file` and
//! `(<)glob` directives into a single composed [`KdlDocument`].
//!
//! `club-kdl` itself is a **pure parser** — `from_str` takes a string, returns
//! a value, no IO. This crate adds the **composer** layer on top: it reads
//! files, walks the parsed AST, and splices in the contents of any files
//! referenced by an include directive — including transitively, with cycle
//! detection.
//!
//! ## Why a separate crate
//!
//! Putting include resolution in core would force every `club-kdl` user to
//! buy into filesystem IO. Keeping it in a companion crate lets users who
//! want pure-parser behavior keep it (parse a string, deserialize), and lets
//! users who want multi-file composition opt in with a single `use`.
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
//! ```no_run
//! use kdl::KdlDocument;
//!
//! // Resolve all (<) directives and return the composed document.
//! let doc: KdlDocument = kdl_compose::compose("./schema.kdl")?;
//!
//! // Or deserialize directly into a typed value via club-kdl.
//! # #[derive(club_kdl::KdlDeserialize)]
//! # #[kdl(document)]
//! # struct Config { /* ... */ }
//! let cfg: Config = kdl_compose::from_path("./config.kdl")?;
//! # Ok::<_, kdl_compose::ComposeError>(())
//! ```

#![warn(missing_docs)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kdl::{KdlDocument, KdlNode, KdlValue};

pub mod error;
pub use error::{ComposeError, Result};

// =============================================================================
// Public API
// =============================================================================

/// Compose a KDL document from `path`, resolving every `(<)` directive
/// recursively. Returns the composed [`KdlDocument`] — equivalent to the file
/// the user wrote, but with every include spliced in place.
///
/// `path` is canonicalized once at entry so cycle detection compares stable
/// identifiers; child paths from `(<)file/glob` are resolved relative
/// to the parent file's directory.
///
/// See the [crate docs](crate) for directive syntax and design limits.
pub fn compose(path: impl AsRef<Path>) -> Result<KdlDocument> {
    let entry = path.as_ref();
    let canonical = canonicalize(entry)?;
    let mut stack = Vec::new();
    let nodes = resolve(&canonical, &mut stack)?;
    let mut doc = KdlDocument::new();
    *doc.nodes_mut() = nodes;
    Ok(doc)
}

/// Compose then deserialize via [`club_kdl::from_doc`] — a thin convenience
/// for the common case of "I have a typed schema and a root KDL file".
pub fn from_path<T>(path: impl AsRef<Path>) -> Result<T>
where
    T: for<'de> club_kdl::KdlDeserialize<'de>,
{
    let doc = compose(path)?;
    club_kdl::from_doc(&doc).map_err(|source| ComposeError::Deserialize { source })
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
fn resolve(canonical_path: &Path, stack: &mut Vec<PathBuf>) -> Result<Vec<KdlNode>> {
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
    let doc: KdlDocument = text.parse().map_err(|source| ComposeError::Parse {
        path: canonical_path.to_path_buf(),
        source,
    })?;

    let base_dir = canonical_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let nodes = process_nodes(doc.nodes(), &base_dir, canonical_path, stack)?;

    stack.pop();
    Ok(nodes)
}

/// Walk `nodes`. For each one, if it is a `(<)` directive, splice in the
/// resolved content; otherwise clone it and recurse into its children so
/// nested includes (inside a `channel` or `protocol` block) are resolved too.
fn process_nodes(
    nodes: &[KdlNode],
    base_dir: &Path,
    current_file: &Path,
    stack: &mut Vec<PathBuf>,
) -> Result<Vec<KdlNode>> {
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes {
        if let Some(directive) = parse_directive(node, current_file)? {
            let included = apply_directive(&directive, base_dir, current_file, stack)?;
            out.extend(included);
        } else {
            let mut new_node = node.clone();
            if let Some(children) = new_node.children_mut() {
                let new_children = process_nodes(children.nodes(), base_dir, current_file, stack)?;
                *children.nodes_mut() = new_children;
            }
            out.push(new_node);
        }
    }
    Ok(out)
}

// =============================================================================
// Directive — parsed (<) node
// =============================================================================

/// A parsed `(<)` directive node, with all its options collected.
struct Directive {
    kind: DirectiveKind,
    as_prefix: Option<String>,
    only: Option<Vec<String>>,
    except: Vec<String>,
    rename: BTreeMap<String, String>,
}

/// The variant — `(<)file` or `(<)glob`.
enum DirectiveKind {
    /// A single path, relative to the importing file's directory.
    File(PathBuf),
    /// A glob pattern, expanded relative to the importing file's directory.
    Glob(String),
}

/// Return `Some(directive)` if `node` is tagged `(<)`, else `None`. Returns
/// `Err` if the tag is `<` but the node is structurally invalid (unknown
/// variant, missing path, unsupported property, etc.).
fn parse_directive(node: &KdlNode, current_file: &Path) -> Result<Option<Directive>> {
    let Some(tag) = node.ty() else {
        return Ok(None);
    };
    if tag.value() != "<" {
        return Ok(None);
    }

    let variant = node.name().value();
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
        match key.value() {
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
    node: &KdlNode,
    current_file: &Path,
) -> Result<(Option<Vec<String>>, Vec<String>, BTreeMap<String, String>)> {
    let mut only_acc: Option<Vec<String>> = None;
    let mut except = Vec::new();
    let mut rename = BTreeMap::new();

    let Some(children) = node.children() else {
        return Ok((only_acc, except, rename));
    };

    for child in children.nodes() {
        match child.name().value() {
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
fn positional_strings(node: &KdlNode, label: &str, current_file: &Path) -> Result<Vec<String>> {
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
) -> Result<Vec<KdlNode>> {
    let paths: Vec<PathBuf> = match &directive.kind {
        DirectiveKind::File(rel) => vec![base_dir.join(rel)],
        DirectiveKind::Glob(pattern) => expand_glob(base_dir, pattern, current_file)?,
    };

    let mut out = Vec::new();
    for path in paths {
        let canonical = canonicalize(&path)?;
        let included_nodes = resolve(&canonical, stack)?;
        for node in included_nodes {
            if let Some(transformed) = transform_node(&node, directive) {
                out.push(transformed);
            }
        }
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
fn transform_node(node: &KdlNode, directive: &Directive) -> Option<KdlNode> {
    let original_name = first_string_arg(node).map(str::to_string);

    // Filter: only / except — operate on original (pre-rename) name.
    if let Some(only) = &directive.only {
        match &original_name {
            Some(n) if only.contains(n) => {}
            _ => return None,
        }
    }
    if let Some(n) = &original_name
        && directive.except.contains(n)
    {
        return None;
    }

    // Rename, then prefix. Both touch only the first string arg.
    let mut new_node = node.clone();
    if let Some(orig) = original_name {
        let renamed = directive.rename.get(&orig).cloned().unwrap_or(orig);
        let final_name = match &directive.as_prefix {
            Some(prefix) => format!("{prefix}.{renamed}"),
            None => renamed,
        };
        set_first_string_arg(&mut new_node, &final_name);
    }
    Some(new_node)
}

// =============================================================================
// Small helpers on KdlNode
// =============================================================================

/// The first positional (un-named) string argument of `node`, if any.
fn first_string_arg(node: &KdlNode) -> Option<&str> {
    node.entries()
        .iter()
        .find(|e| e.name().is_none())
        .and_then(|e| e.value().as_string())
}

/// Replace the first positional string argument of `node` with `new_value`.
/// No-op when the node has no such argument.
fn set_first_string_arg(node: &mut KdlNode, new_value: &str) {
    for entry in node.entries_mut() {
        if entry.name().is_none() && entry.value().is_string() {
            *entry.value_mut() = KdlValue::String(new_value.to_string());
            return;
        }
    }
}

// =============================================================================
// Inline tests — exercise small surface; integration tests cover the rest.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_string_arg_skips_properties() {
        let doc: KdlDocument = r#"node prop="ignored" "the-arg""#.parse().unwrap();
        let node = &doc.nodes()[0];
        assert_eq!(first_string_arg(node), Some("the-arg"));
    }

    #[test]
    fn set_first_string_arg_replaces_in_place() {
        let mut doc: KdlDocument = r#"struct "User" { field "id" type="string" }"#.parse().unwrap();
        let node = &mut doc.nodes_mut()[0];
        set_first_string_arg(node, "Renamed");
        assert_eq!(first_string_arg(node), Some("Renamed"));
        // Children are untouched — only the first string arg of `struct`.
        let child = &node.children().unwrap().nodes()[0];
        assert_eq!(first_string_arg(child), Some("id"));
    }
}
