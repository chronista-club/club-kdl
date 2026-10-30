use club_kdl::{Document, compact};
use indent_pack_research::{Layout, write};

#[test]
fn no_indent_omits_only_layout_spaces() {
    let doc = Document::parse("parent {\n    child \"  keep  \"\n}\n").unwrap();
    assert_eq!(
        write(&doc, Layout::NoIndent),
        "parent {\nchild \"  keep  \"\n}\n"
    );
}

#[test]
fn single_line_preserves_node_boundaries() {
    let doc = Document::parse("parent { child 1; child 2; }; next 3").unwrap();
    assert_eq!(
        write(&doc, Layout::SingleLine),
        "parent{child 1;child 2;};next 3;"
    );
}

fn check(source: &str) {
    let doc = Document::parse(source).unwrap();
    let canonical = doc.to_string();
    assert_eq!(write(&doc, Layout::Pretty), canonical);
    for layout in [Layout::Pretty, Layout::NoIndent, Layout::SingleLine] {
        let text = write(&doc, layout);
        let parsed = Document::parse(&text).unwrap();
        // Canonical text also compares NaNs, unlike PartialEq on float values.
        assert_eq!(parsed.to_string(), canonical, "{layout:?}: {source}");
        if let Ok(reference) = canonical.parse::<club_kdl::kdl::KdlDocument>() {
            let external: club_kdl::kdl::KdlDocument = text.parse().unwrap();
            assert_eq!(
                Document::from(&external).to_string(),
                Document::from(&reference).to_string()
            );
        }
    }
}

#[test]
fn whitespace_values_types_and_duplicates_survive() {
    check(
        r##"
        ("型")root key=(text)"  leading\ttrailing  " dup=1 dup=2 {
            child "" "a\nb\r\n c" "\\\"{};" #true #false #null #nan #inf #-inf
            "日本語" raw=#"  raw {; }  "# multiline="""
                first
                  indented content
                last
                """
        }
        /- ignored "do not serialize"
        empty {}
    "##,
    );
    check("");
}

#[test]
fn naive_line_trimming_is_not_a_valid_pack_algorithm() {
    let source = "n \"\"\"\n  first\n    keep two spaces\n  last\n  \"\"\"\n";
    let naive = source
        .lines()
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(
        Document::parse(source).unwrap().to_string(),
        Document::parse(&naive).unwrap().to_string()
    );
    check(source);
}

#[test]
fn compact_envelopes_remain_compatible() {
    let doc =
        Document::parse("root_name { item_name long_property=1; item_name long_property=2; }")
            .unwrap();
    let table = compact::NameTable::from_document(&doc);
    let shared = table.encode_document(doc.clone()).unwrap();
    let file = compact::encode_document(doc.clone()).unwrap();
    for layout in [Layout::NoIndent, Layout::SingleLine] {
        let shared = write(&Document::parse(&shared).unwrap(), layout);
        let file = write(&Document::parse(&file).unwrap(), layout);
        assert_eq!(table.decode_document(&shared).unwrap(), doc);
        assert_eq!(compact::decode_document(&file).unwrap(), doc);
    }
}

#[test]
fn valid_spec_corpus_round_trips() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/kdl-spec/input");
    let mut valid = 0;
    let mut external = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let source = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        if let Ok(doc) = Document::parse(&source) {
            valid += 1;
            external += usize::from(
                doc.to_string()
                    .parse::<club_kdl::kdl::KdlDocument>()
                    .is_ok(),
            );
            check(&source);
        }
    }
    assert!(valid > 200);
    println!("CORPUS valid={valid} independent_parser={external}");
}
