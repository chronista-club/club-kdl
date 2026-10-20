//! Run club-kdl's parser against the official KDL 2.0 test suite.
//!
//! See `tests/kdl-spec/README.md` for where the cases come from.

use std::fs;
use std::path::PathBuf;

use club_kdl::ast::{Document, Entry, Node};

/// Normalize a tree the way the suite's `expected_kdl` files are written:
/// arguments first in order, then properties deduplicated (rightmost wins)
/// and sorted by key.
fn normalize(node: &Node<'_>) -> String {
    let mut out = String::new();
    write_node(&mut out, node);
    out
}

fn write_node(out: &mut String, node: &Node<'_>) {
    out.push_str(&format!("({:?}){:?}", node.ty(), node.name()));
    let mut props: Vec<&Entry<'_>> = Vec::new();
    for e in node.entries() {
        match e.name() {
            None => out.push_str(&format!(" [{:?}]{:?}", e.ty(), e.value())),
            Some(k) => {
                props.retain(|p| p.name() != Some(k));
                props.push(e);
            }
        }
    }
    props.sort_by_key(|p| p.name());
    for p in props {
        out.push_str(&format!(
            " {:?}=[{:?}]{:?}",
            p.name().unwrap(),
            p.ty(),
            p.value()
        ));
    }
    if !node.children().is_empty() {
        out.push_str(" {");
        for c in node.children() {
            out.push(' ');
            write_node(out, c);
            out.push(';');
        }
        out.push_str(" }");
    }
}

fn normalize_doc(doc: &Document<'_>) -> String {
    normalize(doc.as_node())
}

/// Expected outputs that upstream kdl-org/kdl has but the kdl-rs 6.7.1
/// distribution we copied from lacks.
const MISSING_EXPECTED: &[(&str, &str)] = &[("hex.kdl", "node 12379813812177893520\n")];

#[test]
fn spec_compliance() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/kdl-spec");
    let mut entries: Vec<_> = fs::read_dir(root.join("input"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();

    let mut failures = Vec::new();
    for path in &entries {
        let name = path.file_name().unwrap().to_owned();
        let src = fs::read_to_string(path).unwrap().replace("\r\n", "\n");
        let expected_path = root.join("expected_kdl").join(&name);
        let ours = Document::parse(&src);
        let expected = fs::read_to_string(&expected_path).or_else(|e| {
            MISSING_EXPECTED
                .iter()
                .find(|(n, _)| name == *n)
                .map(|(_, exp)| exp.to_string())
                .ok_or(e)
        });
        match expected {
            Ok(expected_src) => {
                let reference: club_kdl::kdl::KdlDocument = match expected_src.parse() {
                    Ok(d) => d,
                    // kdl-rs itself cannot read a handful of expected files;
                    // fall back to parsing the expected file with our parser.
                    Err(_) => {
                        match (&ours, Document::parse(&expected_src)) {
                            (Ok(a), Ok(b)) if normalize_doc(a) == normalize_doc(&b) => {}
                            (Ok(a), Ok(b)) => failures.push(format!(
                                "{name:?}: mismatch\n  ours:     {}\n  expected: {}",
                                normalize_doc(a),
                                normalize_doc(&b)
                            )),
                            (Err(e), _) => failures.push(format!("{name:?}: should parse: {e}")),
                            (_, Err(e)) => failures.push(format!("{name:?}: expected file: {e}")),
                        }
                        continue;
                    }
                };
                let reference = Document::from(&reference);
                match &ours {
                    Ok(doc) => {
                        let (a, b) = (normalize_doc(doc), normalize_doc(&reference));
                        if a != b {
                            failures.push(format!(
                                "{name:?}: mismatch\n  ours:     {a}\n  expected: {b}"
                            ));
                        }
                    }
                    Err(e) => failures.push(format!("{name:?}: should parse: {e}")),
                }
            }
            Err(_) => {
                if ours.is_ok() {
                    failures.push(format!("{name:?}: should fail to parse"));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} spec cases failed:\n{}",
        failures.len(),
        entries.len(),
        failures.join("\n")
    );
}

/// Every valid spec document must survive parse → write → parse unchanged.
/// This exercises the writer's quoting, escaping and number formatting
/// against the full variety of the suite.
#[test]
fn writer_round_trips_spec_documents() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/kdl-spec");
    let mut failures = Vec::new();
    let mut count = 0usize;
    for entry in fs::read_dir(root.join("input")).unwrap() {
        let path = entry.unwrap().path();
        let src = fs::read_to_string(&path).unwrap();
        let Ok(doc) = Document::parse(&src) else {
            continue;
        };
        count += 1;
        let written = doc.to_string();
        let name = path.file_name().unwrap().to_owned();
        match Document::parse(&written) {
            Ok(again) if normalize_doc(&again) == normalize_doc(&doc) => {}
            Ok(again) => failures.push(format!(
                "{name:?}: changed\n  before: {}\n  after:  {}\n  text: {written:?}",
                normalize_doc(&doc),
                normalize_doc(&again)
            )),
            Err(e) => failures.push(format!("{name:?}: written text fails: {e}\n  {written:?}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {count} round trips failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
