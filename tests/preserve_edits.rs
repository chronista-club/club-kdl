//! Minimal source-preserving edits: mem_1CggW2QewS8jT8ejr6jRC6.

#![cfg(feature = "edit")]

use club_kdl::edit::{EditError, EditableDocument};
use club_kdl::{Document, Value};

#[test]
fn unedited_document_keeps_comments_and_layout_exactly() {
    let src = "// 接続設定\r\nserver\t\"localhost\" port=0x1F90 // 開発用\r\n";
    let doc = EditableDocument::parse(src).unwrap();
    assert_eq!(doc.to_string(), src);
    assert_eq!(doc.as_str(), src);
    assert_eq!(doc.into_string(), src);
}

#[test]
fn empty_comment_only_and_bom_documents_are_retained() {
    for src in [
        "",
        "\u{feff}",
        "  // コメント without newline",
        "/* outer /* nested */ comment */\r\n\t",
        "\u{feff}// heading\r\n/- disabled 1\r\nnode; // tail",
    ] {
        assert_eq!(EditableDocument::parse(src).unwrap().into_string(), src);
    }
}

#[test]
fn property_edit_changes_only_the_value_token() {
    let src = "\u{feff}// 接続設定\r\nserver\t\"localhost\" \
        \"port\" /* key */ = /* eq */ ( /* type */ u16 ) /* value */ 0x1F90 // 開発用\r\n";
    let mut doc = EditableDocument::parse(src).unwrap();
    doc.set_property(&[0], "port", 9090).unwrap();
    assert_eq!(doc.as_str(), src.replace("0x1F90", "9090"));
}

#[test]
fn argument_indices_ignore_properties_and_slashdashed_entries() {
    let src = "node key=1 /- \"ignored\" \"first\" other=#true (text)\"second\" // tail\n";
    let mut doc = EditableDocument::parse(src).unwrap();
    doc.set_argument(&[0], 1, "new value").unwrap();
    assert_eq!(doc.as_str(), src.replace("\"second\"", "\"new value\""));
}

#[test]
fn nested_paths_ignore_slashdashed_nodes_and_children() {
    let src = "/- parent 10 { child 20 }\n\
parent /- 30 40 /- { child 50 } {\n\
    /- child 60\n\
    child 70 /- skip=80 value=90\n\
} /- { child 100 }\n";
    let mut doc = EditableDocument::parse(src).unwrap();
    doc.set_argument(&[0, 0], 0, 71).unwrap();
    doc.set_property(&[0, 0], "value", 91).unwrap();
    doc.set_argument(&[0], 0, 41).unwrap();
    assert_eq!(
        doc.as_str(),
        src.replace("child 70", "child 71")
            .replace("value=90", "value=91")
            .replace("30 40", "30 41")
    );
}

#[test]
fn duplicate_nodes_are_indexed_and_duplicate_properties_use_the_rightmost() {
    let src = "node p=0\nnode p=1 p=2 /- p=3 p=4\n";
    let mut doc = EditableDocument::parse(src).unwrap();
    doc.set_property(&[1], "p", 99).unwrap();
    assert_eq!(doc.as_str(), "node p=0\nnode p=1 p=2 /- p=3 p=99\n");
}

#[test]
fn repeated_edits_recompute_offsets_after_unicode_and_length_changes() {
    let src = "node \"a\" x=2 {\n  child \"b\" y=4 // keep\n}\n";
    let mut doc = EditableDocument::parse(src).unwrap();
    doc.set_argument(&[0], 0, "長い 日本語").unwrap();
    doc.set_property(&[0, 0], "y", false).unwrap();
    doc.set_argument(&[0], 0, "z").unwrap();
    doc.set_property(&[0], "x", 12345).unwrap();
    assert_eq!(
        doc.as_str(),
        "node z x=12345 {\n  child \"b\" y=#false // keep\n}\n"
    );
}

#[test]
fn raw_multiline_and_typed_value_ranges_exclude_surrounding_comments() {
    for (src, expected) in [
        (
            "node ##\"raw /* text */ 日本語\"##/* keep */\n",
            "node #false/* keep */\n",
        ),
        (
            "node (text) \"\"\"\n  first\n  second\n  \"\"\" // keep\n",
            "node (text) #false // keep\n",
        ),
        (
            "node (text) #\"\"\"\n  first\n  second\n  \"\"\"# // keep\n",
            "node (text) #false // keep\n",
        ),
        ("node +1_000 // keep", "node #false // keep"),
    ] {
        let mut doc = EditableDocument::parse(src).unwrap();
        doc.set_argument(&[0], 0, false).unwrap();
        assert_eq!(doc.as_str(), expected);
    }
}

#[test]
fn replacement_values_are_escaped_and_remain_values() {
    for value in [
        Value::String("\"\n} evil #true\n/*\0\\日本語".into()),
        Value::String("".into()),
        Value::Integer(i128::MIN),
        Value::Float(-0.0),
        Value::Float(f64::INFINITY),
        Value::Float(f64::NEG_INFINITY),
        Value::Float(f64::NAN),
        Value::Bool(true),
        Value::Null,
    ] {
        let mut doc = EditableDocument::parse("// keep\nnode (tag)0 /* tail */\n").unwrap();
        doc.set_argument(&[0], 0, &value).unwrap();
        assert!(doc.as_str().starts_with("// keep\nnode (tag)"));
        assert!(doc.as_str().ends_with(" /* tail */\n"));
        let parsed = Document::parse(doc.as_str()).unwrap();
        assert_eq!(parsed.nodes().len(), 1);
        let node = &parsed.nodes()[0];
        assert_eq!(node.entries().len(), 1);
        assert_eq!(node.entries()[0].ty(), Some("tag"));
        match (&value, node.arg(0).unwrap()) {
            (Value::Float(a), Value::Float(b)) if a.is_nan() => assert!(b.is_nan()),
            (Value::Float(a), Value::Float(b)) => assert_eq!(a.to_bits(), b.to_bits()),
            (a, b) => assert_eq!(a, b),
        }
    }
}

#[test]
fn missing_targets_return_errors_without_changing_text() {
    let src = "node 1 /- 2 p=3 /- gone=4 { child } // keep\n";
    let mut doc = EditableDocument::parse(src).unwrap();
    for path in [&[][..], &[1], &[0, 1], &[0, 0, 0], &[usize::MAX]] {
        assert!(matches!(
            doc.set_argument(path, 0, 99),
            Err(EditError::MissingNode { .. })
        ));
        assert_eq!(doc.as_str(), src);
    }
    for index in [1, usize::MAX] {
        assert!(matches!(
            doc.set_argument(&[0], index, 99),
            Err(EditError::MissingArgument { .. })
        ));
        assert_eq!(doc.as_str(), src);
    }
    assert!(matches!(
        doc.set_property(&[0], "gone", 99),
        Err(EditError::MissingProperty { .. })
    ));
    assert_eq!(doc.as_str(), src);
}

#[test]
fn malformed_documents_are_rejected() {
    for src in ["/* unclosed", "node {", "node p=", "node #unknown", "/-"] {
        assert!(matches!(
            EditableDocument::parse(src),
            Err(EditError::Parse(_))
        ));
    }
}
