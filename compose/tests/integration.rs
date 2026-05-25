//! End-to-end tests for `club-kdl-compose`.
//!
//! Every test points at a fixture under `tests/fixtures/<scenario>/`. The
//! fixture files are real `.kdl` so that the path-resolution and IO paths
//! get exercised end-to-end — that is the point of the crate, and inlining
//! the schemas as strings would defeat it.

use std::path::{Path, PathBuf};

use kdl_compose::{ComposeError, compose};

/// Resolve `<cargo manifest>/tests/fixtures/<scenario>/<entry>`. Every
/// scenario has its own subdirectory so the relative paths inside each
/// fixture file stay short and self-contained.
fn fixture(scenario: &str, entry: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(scenario)
        .join(entry)
}

/// Names (first string arg) of each top-level node, in source order.
fn top_level_names(doc: &kdl::KdlDocument) -> Vec<String> {
    doc.nodes()
        .iter()
        .map(|n| {
            n.entries()
                .iter()
                .find(|e| e.name().is_none())
                .and_then(|e| e.value().as_string())
                .unwrap_or("")
                .to_string()
        })
        .collect()
}

// =============================================================================
// MVP — (<)file / cycle / IO / invalid directive
// =============================================================================

#[test]
fn simple_include_splices_child_at_directive_position() {
    let doc = compose(fixture("simple", "root.kdl")).expect("compose");
    let nodes = doc.nodes();
    // child's two nodes splice in where (<)file used to be, then top remains.
    assert_eq!(nodes.len(), 3);
    assert_eq!(nodes[0].name().value(), "child");
    assert_eq!(nodes[1].name().value(), "child");
    assert_eq!(nodes[2].name().value(), "top");
    assert_eq!(top_level_names(&doc), vec!["first", "second", "root-node"]);
}

#[test]
fn recursive_include_resolves_transitively() {
    // a → b → c, every file contributes one `from` node + its include.
    let doc = compose(fixture("recursive", "a.kdl")).expect("compose");
    assert_eq!(top_level_names(&doc), vec!["c", "b", "a"]);
}

#[test]
fn cycle_is_detected_and_reported() {
    let err = compose(fixture("cycle", "a.kdl")).expect_err("cycle should fail");
    match err {
        ComposeError::Cycle { stack } => {
            // The cycle stack runs a.kdl → b.kdl → a.kdl (canonicalized).
            assert_eq!(stack.len(), 3);
            assert!(stack[0].ends_with("a.kdl"));
            assert!(stack[1].ends_with("b.kdl"));
            assert!(stack[2].ends_with("a.kdl"));
        }
        other => panic!("expected Cycle, got {other:?}"),
    }
}

#[test]
fn missing_included_file_surfaces_io_error() {
    let err = compose(fixture("missing", "root.kdl")).expect_err("missing");
    assert!(
        matches!(err, ComposeError::Io { .. }),
        "expected Io, got {err:?}"
    );
}

#[test]
fn unknown_variant_is_rejected() {
    let err = compose(fixture("invalid-variant", "root.kdl")).expect_err("unknown variant");
    match err {
        ComposeError::InvalidDirective { message, .. } => {
            assert!(message.contains("unknown (<) variant"));
            assert!(message.contains("wat"));
        }
        other => panic!("expected InvalidDirective, got {other:?}"),
    }
}

#[test]
fn file_directive_without_path_is_rejected() {
    let err = compose(fixture("invalid-missing-path", "root.kdl"))
        .expect_err("missing path arg should fail");
    assert!(
        matches!(err, ComposeError::InvalidDirective { .. }),
        "expected InvalidDirective, got {err:?}"
    );
}

#[test]
fn directive_node_does_not_appear_in_output() {
    // The `simple` fixture's root has a (<)file directive followed by `top`.
    // After composition no node should be tagged `(<)`.
    let doc = compose(fixture("simple", "root.kdl")).expect("compose");
    for node in doc.nodes() {
        assert_ne!(
            node.ty().map(kdl::KdlIdentifier::value),
            Some("<"),
            "directive node leaked into output: {node}"
        );
    }
}

// =============================================================================
// Nested directive — (<) inside a `protocol`/`channel` block
// =============================================================================

#[test]
fn directive_inside_block_splices_into_that_block() {
    let doc = compose(fixture("nested", "root.kdl")).expect("compose");
    // Top-level has one `protocol` node.
    assert_eq!(doc.nodes().len(), 1);
    let protocol = &doc.nodes()[0];
    assert_eq!(protocol.name().value(), "protocol");

    // Drill into protocol > channel > children.
    let channel = &protocol.children().unwrap().nodes()[0];
    assert_eq!(channel.name().value(), "channel");
    let children = channel.children().unwrap().nodes();

    // reqs.kdl contributed two `request` nodes, then the local `request` follows.
    let names: Vec<&str> = children.iter().map(|n| n.name().value()).collect();
    assert_eq!(names, vec!["request", "request", "request"]);

    let request_args: Vec<&str> = children
        .iter()
        .map(|n| {
            n.entries()
                .iter()
                .find(|e| e.name().is_none())
                .and_then(|e| e.value().as_string())
                .unwrap_or("")
        })
        .collect();
    assert_eq!(request_args, vec!["log:info", "log:error", "specific:save"]);
}

// =============================================================================
// Phase 2.1 — (<)glob and as= prefix
// =============================================================================

#[test]
fn glob_inlines_every_matching_file_sorted() {
    let doc = compose(fixture("glob", "root.kdl")).expect("compose");
    // types/a.kdl + types/b.kdl + the trailing `top` node.
    assert_eq!(top_level_names(&doc), vec!["Alpha", "Beta", "after-glob"]);
}

#[test]
fn as_prefix_renames_first_string_arg_of_each_top_level_node() {
    let doc = compose(fixture("as-prefix", "root.kdl")).expect("compose");
    // types.kdl had `User` / `Memory`; with as="shared" they become prefixed.
    assert_eq!(
        top_level_names(&doc),
        vec!["shared.User", "shared.Memory", "after-include"]
    );

    // Internal references inside children are *not* touched by as=.
    let user = &doc.nodes()[0];
    let field = &user.children().unwrap().nodes()[0];
    assert_eq!(
        field
            .entries()
            .iter()
            .find(|e| e.name().is_none())
            .and_then(|e| e.value().as_string()),
        Some("id"),
        "as= must not touch nested nodes' identifiers"
    );
}

// =============================================================================
// Phase 2.2 — only / except / rename + apply order
// =============================================================================

#[test]
fn only_filters_drop_rename_and_as_apply_in_order() {
    // filters/root.kdl applies, in order:
    //   only=["User" "Memory"]   → drop `Internal`
    //   rename=User->Acct        → rename `User` to `Acct`
    //   as="shared"              → prefix every kept node with `shared.`
    //
    // The trailing `trailing "kept"` node is outside the directive, untouched.
    let doc = compose(fixture("filters", "root.kdl")).expect("compose");
    assert_eq!(
        top_level_names(&doc),
        vec!["shared.Acct", "shared.Memory", "kept"]
    );
    // Internal must be filtered out entirely.
    assert!(
        !top_level_names(&doc).iter().any(|n| n.contains("Internal")),
        "`only` must filter out `Internal`"
    );
}

// =============================================================================
// Public API smoke — from_path<T>
// =============================================================================

#[derive(Debug, club_kdl::KdlDeserialize)]
#[kdl(document)]
struct SimpleDoc {
    #[kdl(child)]
    top: TopNode,
}

#[derive(Debug, club_kdl::KdlDeserialize)]
struct TopNode {
    #[kdl(argument)]
    name: String,
}

#[test]
fn from_path_deserializes_through_compose() {
    // The `simple` fixture composes to: child "first", child "second", top "root-node".
    // We deserialize only the `top` field — child entries are extra siblings,
    // ignored by the derive (it picks the named child).
    let parsed: SimpleDoc = kdl_compose::from_path(fixture("simple", "root.kdl"))
        .expect("from_path should resolve and deserialize");
    assert_eq!(parsed.top.name, "root-node");
}
