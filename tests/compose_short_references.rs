//! Short reference behavior with real files (spec/design 02).
//! Task: mem_1CggNetBUkKbDNuApathjJ.

#![cfg(feature = "compose")]

use club_kdl::{Document, compose::ComposeError, compose::compose};

fn fixture(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, text) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

#[test]
fn short_reference_matches_file_import_with_or_without_an_extension() {
    let dir = fixture(&[
        ("a.kdl", "item \"included\"\n"),
        ("legacy.kdl", "(<)file \"./a.kdl\"\n"),
    ]);
    let expected = compose(dir.path().join("legacy.kdl")).unwrap();
    for input in [r#"> "a""#, r#"> "./a""#, r#"> "./a.kdl""#] {
        std::fs::write(dir.path().join("root.kdl"), input).unwrap();
        assert_eq!(
            compose(dir.path().join("root.kdl")).unwrap(),
            expected,
            "{input}"
        );
    }
}

#[test]
fn completion_is_deterministic_and_legacy_paths_stay_literal() {
    let dir = fixture(&[
        ("a", "literal\n"),
        ("a.kdl", "completed\n"),
        ("root.kdl", "> \"a\"\n(<)file \"a\"\n"),
    ]);
    assert_eq!(
        compose(dir.path().join("root.kdl")).unwrap(),
        Document::parse("completed\nliteral\n").unwrap()
    );
}

#[test]
fn missing_completed_path_does_not_fall_back_to_extensionless_file() {
    let dir = fixture(&[("a", "literal\n"), ("root.kdl", "> \"a\"\n")]);
    match compose(dir.path().join("root.kdl")).unwrap_err() {
        ComposeError::Io { path, .. } => assert_eq!(path.file_name().unwrap(), "a.kdl"),
        other => panic!("expected missing a.kdl, got {other:?}"),
    }
}

#[test]
fn explicit_extensions_dotfiles_and_parent_directory_dots_are_handled() {
    let dir = fixture(&[
        ("a.data", "explicit\n"),
        (".hidden.kdl", "dotfile\n"),
        ("parts.v1/a.kdl", "parent\n"),
        ("内装.kdl", "japanese\n"),
        (
            "root.kdl",
            "> \"a.data\"\n> \".hidden\"\n> \"parts.v1/a\"\n> \"内装\"\n",
        ),
    ]);
    assert_eq!(
        compose(dir.path().join("root.kdl")).unwrap(),
        Document::parse("explicit\ndotfile\nparent\njapanese\n").unwrap()
    );
}

#[test]
fn nested_transitive_and_repeated_references_preserve_order_and_local_base() {
    let dir = fixture(&[
        (
            "root.kdl",
            "outer { before; > \"parts/a\"; middle; > \"parts/a\"; after; }",
        ),
        ("parts/a.kdl", "> \"../shared\"\nitem \"a\"\n"),
        ("shared.kdl", "item \"shared\"\n"),
    ]);
    assert_eq!(
        compose(dir.path().join("root.kdl")).unwrap(),
        Document::parse("outer { before; item \"shared\"; item \"a\"; middle; item \"shared\"; item \"a\"; after; }").unwrap()
    );
}

#[test]
fn options_share_legacy_filter_rename_prefix_order_after_recursive_expansion() {
    let dir = fixture(&[
        ("a.kdl", "> \"b\"\n"),
        (
            "b.kdl",
            "meta 2\nitem \"A\" { link \"A\"; }\nitem \"B\"\nitem \"C\"\n",
        ),
    ]);
    let options = r#"as="shared" { only "A" "B"; except "B"; rename "A" "Renamed"; }"#;
    for reference in [r#"> "a""#, r#"(<)file "./a.kdl""#] {
        std::fs::write(
            dir.path().join("root.kdl"),
            format!("{reference} {options}"),
        )
        .unwrap();
        assert_eq!(
            compose(dir.path().join("root.kdl")).unwrap(),
            Document::parse("meta 2\nitem \"shared.Renamed\" { link \"A\"; }").unwrap(),
            "{reference}"
        );
    }
}

#[test]
fn plain_and_typed_data_are_preserved_even_when_a_file_exists() {
    let data = "a\n\"a\"\nenabled\npath \"a\"\n(custom)> \"a\"\n(>)path \"a\"\n>path\n";
    let dir = fixture(&[("a.kdl", "should-not-appear\n"), ("root.kdl", data)]);
    assert_eq!(
        compose(dir.path().join("root.kdl")).unwrap(),
        Document::parse(data).unwrap()
    );
}

#[test]
fn pure_parsing_and_writing_preserve_reference_meaning_without_doing_io() {
    let dir = fixture(&[("a.kdl", "included\n")]);
    for input in [r#"> "a""#, r#"">" a"#] {
        let parsed = Document::parse(input).unwrap();
        assert_eq!(parsed.nodes()[0].name(), ">");
        assert_eq!(parsed.nodes()[0].arg(0).unwrap().as_string(), Some("a"));
        #[cfg(feature = "kdl-rs")]
        {
            let independent: club_kdl::kdl::KdlDocument = input.parse().unwrap();
            assert_eq!(independent.nodes()[0].name().value(), ">");
        }
        let normalized = parsed.to_string();
        assert_eq!(Document::parse(&normalized).unwrap(), parsed);
        std::fs::write(dir.path().join("root.kdl"), normalized).unwrap();
        assert_eq!(
            compose(dir.path().join("root.kdl")).unwrap(),
            Document::parse("included\n").unwrap()
        );
    }
}

#[test]
fn mixed_short_and_legacy_cycles_are_reported() {
    for (root, child) in [
        ("> \"a\"\n", "(<)file \"root.kdl\"\n"),
        ("(<)file \"a.kdl\"\n", "> \"root\"\n"),
    ] {
        let dir = fixture(&[("root.kdl", root), ("a.kdl", child)]);
        match compose(dir.path().join("root.kdl")).unwrap_err() {
            ComposeError::Cycle { stack } => {
                assert_eq!(stack.len(), 3);
                assert_eq!(stack.first(), stack.last());
                assert!(stack[1].ends_with("a.kdl"));
            }
            other => panic!("expected Cycle, got {other:?}"),
        }
    }
}

#[test]
fn imported_parse_errors_propagate_instead_of_returning_partial_output() {
    let dir = fixture(&[
        ("a.kdl", "broken {"),
        ("root.kdl", "before\n> \"a\"\nafter\n"),
    ]);
    match compose(dir.path().join("root.kdl")).unwrap_err() {
        ComposeError::Parse { path, .. } => assert!(path.ends_with("a.kdl")),
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn a_directory_is_not_imported_as_a_file() {
    let dir = fixture(&[
        ("a.kdl/child.kdl", "child\n"),
        ("root.kdl", "> \"a.kdl\"\n"),
    ]);
    assert!(matches!(
        compose(dir.path().join("root.kdl")),
        Err(ComposeError::Io { .. })
    ));
}

#[test]
fn short_reference_requires_exactly_one_unannotated_string_argument() {
    let dir = fixture(&[("a.kdl", "included\n")]);
    for input in [
        ">",
        "> 42",
        "> #null",
        "> #true",
        r#"> (path)"a""#,
        r#"> "a" "b""#,
        r#"> "a" 42"#,
    ] {
        Document::parse(input).expect("test input must be valid KDL");
        std::fs::write(dir.path().join("root.kdl"), input).unwrap();
        assert!(
            matches!(
                compose(dir.path().join("root.kdl")),
                Err(ComposeError::InvalidDirective { .. })
            ),
            "expected InvalidDirective for {input}"
        );
    }
}

#[test]
fn empty_directory_like_and_trailing_dot_paths_are_rejected() {
    let dir = fixture(&[]);
    for path in [
        "", ".", "..", "parts/.", "parts/..", "parts/", "a.", "parts/a.",
    ] {
        let input = format!("> \"{path}\"");
        std::fs::write(dir.path().join("root.kdl"), input).unwrap();
        match compose(dir.path().join("root.kdl")).unwrap_err() {
            ComposeError::InvalidDirective {
                path: importer,
                message,
            } => {
                assert!(importer.ends_with("root.kdl"));
                assert!(message.contains(path));
            }
            other => panic!("expected InvalidDirective for {path:?}, got {other:?}"),
        }
    }
}

#[test]
fn nul_in_path_is_a_directive_error() {
    let input = r#"> "a\u{0}""#;
    Document::parse(input).expect("escaped NUL is syntactically valid KDL");
    let dir = fixture(&[("root.kdl", input)]);
    assert!(matches!(
        compose(dir.path().join("root.kdl")),
        Err(ComposeError::InvalidDirective { .. })
    ));
}

#[test]
fn native_separators_are_used_for_completion_and_directory_rejection() {
    let sep = std::path::MAIN_SEPARATOR;
    let dir = fixture(&[("parts/a.kdl", "included\n")]);
    std::fs::write(dir.path().join("root.kdl"), format!("> #\"parts{sep}a\"#")).unwrap();
    assert_eq!(
        compose(dir.path().join("root.kdl")).unwrap(),
        Document::parse("included\n").unwrap()
    );
    for suffix in ["", ".", ".."] {
        std::fs::write(
            dir.path().join("root.kdl"),
            format!("> #\"parts{sep}{suffix}\"#"),
        )
        .unwrap();
        assert!(matches!(
            compose(dir.path().join("root.kdl")),
            Err(ComposeError::InvalidDirective { .. })
        ));
    }
}

#[test]
fn short_reference_reuses_option_validation() {
    let dir = fixture(&[("a.kdl", "included\n")]);
    for input in [
        r#"> "a" unknown="x""#,
        r#"> "a" as=42"#,
        r#"> "a" { unknown "x"; }"#,
        r#"> "a" { rename "one"; }"#,
    ] {
        Document::parse(input).expect("test input must be valid KDL");
        std::fs::write(dir.path().join("root.kdl"), input).unwrap();
        assert!(
            matches!(
                compose(dir.path().join("root.kdl")),
                Err(ComposeError::InvalidDirective { .. })
            ),
            "{input}"
        );
    }
}

#[test]
fn from_path_deserializes_a_short_reference() {
    #[derive(Debug, club_kdl::KdlDeserialize)]
    #[kdl(document)]
    struct Root {
        #[kdl(child)]
        item: Item,
    }
    #[derive(Debug, club_kdl::KdlDeserialize)]
    struct Item {
        #[kdl(argument)]
        name: String,
    }
    let dir = fixture(&[("root.kdl", "> \"a\"\n"), ("a.kdl", "item \"loaded\"\n")]);
    let parsed: Root = club_kdl::compose::from_path(dir.path().join("root.kdl")).unwrap();
    assert_eq!(parsed.item.name, "loaded");
}
