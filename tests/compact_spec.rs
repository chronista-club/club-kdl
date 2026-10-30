//! Compact writer coverage against the official KDL corpus.
//! Task: mem_1CgYGA5N9F8xynDW6t9N2A. Excluded from published packages with the corpus.
#![cfg(feature = "compact")]

use club_kdl::{
    Document,
    compact::{self, NameTable},
};

#[test]
fn unindented_compact_round_trips_valid_spec_documents() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/kdl-spec/input");
    let mut count = 0;
    for entry in std::fs::read_dir(root).unwrap() {
        let source = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        let Ok(doc) = Document::parse(&source) else {
            continue;
        };
        count += 1;
        let table = NameTable::from_document(&doc);
        let shared = table.encode_document(doc.clone()).unwrap();
        let file = compact::encode_document(doc.clone()).unwrap();
        for text in [&shared, &file] {
            assert!(text.lines().all(|line| !line.starts_with([' ', '\t'])));
            let external: club_kdl::kdl::KdlDocument = text.parse().unwrap();
            assert_eq!(
                Document::from(&external).to_string(),
                Document::parse(text).unwrap().to_string()
            );
        }
        assert_eq!(
            table.decode_document(&shared).unwrap().to_string(),
            doc.to_string()
        );
        assert_eq!(
            compact::decode_document(&file).unwrap().to_string(),
            doc.to_string()
        );
    }
    assert!(count >= 233, "spec corpus missing: {count}");
}
