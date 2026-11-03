//! Corpus validation for mem_1CggW2QewS8jT8ejr6jRC6 (spec 03).
#![cfg(feature = "edit")]

use std::fs;
use std::path::PathBuf;

use club_kdl::edit::EditableDocument;
use club_kdl::{Document, Node, Value};

fn inputs() -> impl Iterator<Item = (PathBuf, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/kdl-spec/input");
    let mut paths: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    paths.sort();
    paths.into_iter().map(|path| {
        let source = fs::read_to_string(&path).unwrap();
        (path, source)
    })
}

#[test]
fn parser_acceptance_and_unedited_bytes_match_the_spec_corpus() {
    let mut valid = 0;
    let mut invalid = 0;
    for (path, source) in inputs() {
        let edited = EditableDocument::parse(&source);
        if Document::parse(&source).is_ok() {
            valid += 1;
            assert_eq!(edited.unwrap().into_string(), source, "{path:?}");
        } else {
            invalid += 1;
            assert!(edited.is_err(), "{path:?}");
        }
    }
    assert!(valid >= 233, "incomplete valid corpus: {valid}");
    assert!(invalid > 0, "missing invalid corpus");
    eprintln!("preserving parser: {valid} valid and {invalid} invalid documents");
}

#[derive(Clone, Debug, PartialEq)]
struct EntrySnapshot {
    name: Option<String>,
    ty: Option<String>,
    value: String,
}

#[derive(Clone, Debug, PartialEq)]
struct NodeSnapshot {
    path: Vec<usize>,
    name: String,
    ty: Option<String>,
    entries: Vec<EntrySnapshot>,
}

fn snapshot(doc: &Document<'_>) -> Vec<NodeSnapshot> {
    fn visit(nodes: &[Node<'_>], path: &mut Vec<usize>, out: &mut Vec<NodeSnapshot>) {
        for (index, node) in nodes.iter().enumerate() {
            path.push(index);
            out.push(NodeSnapshot {
                path: path.clone(),
                name: node.name().to_owned(),
                ty: node.ty().map(str::to_owned),
                entries: node
                    .entries()
                    .iter()
                    .map(|entry| EntrySnapshot {
                        name: entry.name().map(str::to_owned),
                        ty: entry.ty().map(str::to_owned),
                        // Canonical values also compare NaNs without NaN != NaN.
                        value: entry.value().to_string(),
                    })
                    .collect(),
            });
            visit(node.children(), path, out);
            path.pop();
        }
    }
    let mut out = Vec::new();
    visit(doc.nodes(), &mut Vec::new(), &mut out);
    out
}

#[test]
fn every_addressable_value_can_be_changed_without_changing_other_data() {
    let replacement = "new value\n\"日本語\"";
    let expected_value = Value::String(replacement.into()).to_string();
    let mut changed = 0;
    for (path, source) in inputs() {
        let Ok(doc) = Document::parse(&source) else {
            continue;
        };
        let original = snapshot(&doc);
        for (node_index, node) in original.iter().enumerate() {
            let mut argument_index = 0;
            for (entry_index, entry) in node.entries.iter().enumerate() {
                let mut edit = EditableDocument::parse(&source).unwrap();
                if let Some(name) = &entry.name {
                    // Earlier duplicate properties are deliberately not addressable
                    // by name; only the effective rightmost property is updated.
                    if node.entries[entry_index + 1..]
                        .iter()
                        .any(|later| later.name.as_ref() == Some(name))
                    {
                        continue;
                    }
                    edit.set_property(&node.path, name, replacement).unwrap();
                } else {
                    edit.set_argument(&node.path, argument_index, replacement)
                        .unwrap();
                    argument_index += 1;
                }
                let actual = Document::parse(edit.as_str())
                    .unwrap_or_else(|error| panic!("{path:?}: {error}\n{}", edit.as_str()));
                let mut expected = original.clone();
                expected[node_index].entries[entry_index].value = expected_value.clone();
                assert_eq!(
                    snapshot(&actual),
                    expected,
                    "{path:?}: node {:?}, entry {entry_index}",
                    node.path
                );
                changed += 1;
            }
        }
    }
    // The vendored corpus has 198 addressable values (earlier duplicate
    // properties and slashdashed values are intentionally not targets).
    assert_eq!(
        changed, 198,
        "the spec edit corpus changed; review coverage"
    );
    eprintln!("preserving edits: {changed} values checked");
}
