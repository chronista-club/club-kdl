//! Name shortcut acceptance tests. Task: mem_1CgYBNbHDqGGdtK66QsMCL.
#![cfg(feature = "compact")]

use club_kdl::compact::{self, CompactError, NameTable, from_str, to_string};
use club_kdl::{Document, KdlDeserialize, KdlSerialize};

#[derive(Debug, PartialEq, KdlSerialize, KdlDeserialize)]
#[kdl(name = "object_message")]
struct Message {
    #[kdl(property)]
    sequence_number: u64,
    #[kdl(children)]
    records: Vec<Record>,
}

#[derive(Debug, PartialEq, KdlSerialize, KdlDeserialize)]
#[kdl(name = "record_update")]
struct Record {
    #[kdl(property)]
    object_identifier: String,
    #[kdl(property)]
    position_horizontal: f64,
    #[kdl(property)]
    visible: bool,
}

fn fixture() -> Message {
    Message {
        sequence_number: 42,
        records: (0..100)
            .map(|i| Record {
                object_identifier: format!("object-{i}"),
                position_horizontal: 12.5,
                visible: i % 2 == 0,
            })
            .collect(),
    }
}

#[test]
fn self_contained_file_is_smaller_and_restores_the_original_object() {
    let value = fixture();
    let normal = club_kdl::to_string(&value).unwrap();
    let compact = to_string(&value).unwrap();
    assert!(
        compact.starts_with("kdl-names 1 {"),
        "missing embedded name table"
    );
    assert!(compact.len() < normal.len());
    assert_eq!(from_str::<Message>(&compact).unwrap(), value);
    assert!(normal.starts_with("object_message sequence_number=42"));
    let _: club_kdl::kdl::KdlDocument = compact.parse().unwrap();
}

#[test]
fn shared_table_can_be_exchanged_and_reused_for_objects() {
    let value = fixture();
    let table = NameTable::from_value(&value).unwrap();
    let peer = NameTable::from_kdl(&table.to_kdl()).unwrap();
    assert_eq!(table.id(), peer.id());
    for sequence_number in [42, 100, 999] {
        let value = Message {
            sequence_number,
            ..fixture()
        };
        let wire = table.encode(&value).unwrap();
        assert!(wire.starts_with("kdl-message 1 "));
        assert!(!wire.contains("object_message"));
        assert_eq!(peer.decode::<Message>(&wire).unwrap(), value);
        let _: club_kdl::kdl::KdlDocument = wire.parse().unwrap();
    }
}

#[test]
fn names_are_deterministic_and_extend_past_one_character_without_collisions() {
    let mut names: Vec<String> = (0..100).map(|n| format!("field_{n:03}")).collect();
    names.extend(["a".into(), "b".into(), "aa".into(), "xy".into()]);
    let table = NameTable::from_names(&names);
    names.reverse();
    names.push("field_001".into());
    assert_eq!(table.to_kdl(), NameTable::from_names(&names).to_kdl());
    let dictionary = table.to_kdl();
    let doc = Document::parse(&dictionary).unwrap();
    let entries = doc.nodes()[0].children();
    let aliases: std::collections::BTreeSet<_> = entries.iter().map(|n| n.name()).collect();
    assert_eq!(aliases.len(), 104);
    for original in ["a", "b", "aa", "xy"] {
        let entry = entries
            .iter()
            .find(|n| n.arg(0).unwrap().as_string() == Some(original))
            .unwrap();
        assert_eq!(entry.name(), original);
    }
    let first = entries
        .iter()
        .find(|n| n.arg(0).unwrap().as_string() == Some("field_000"))
        .unwrap();
    assert_eq!(first.name(), "c");
    assert!(aliases.contains("ab"));
    assert!(entries.iter().filter(|n| n.name().len() == 2).count() > 40);
}

#[test]
fn document_round_trip_preserves_types_values_duplicates_and_protocol_like_names() {
    let input = r#"
        (object_message)object_message (record_update)"object_message" a="a" repeated_key=1 repeated_key=2 {
            (value_type)record_update field_name=(value_type)"record_update"
            "0" "1"="0" "日本語の名前"="日本語の値"
            kdl-names 1
            kdl-message "not a header"
        }
        "" ""=#null
    "#;
    let doc = Document::parse(input).unwrap();
    let table = NameTable::from_document(&doc);
    let wire = table.encode_document(doc.clone()).unwrap();
    assert_eq!(table.decode_document(&wire).unwrap(), doc);
    let file = compact::encode_document(doc.clone()).unwrap();
    assert_eq!(compact::decode_document(&file).unwrap(), doc);
    let _: club_kdl::kdl::KdlDocument = file.parse().unwrap();
}

#[test]
fn empty_and_document_level_values_round_trip() {
    #[derive(Debug, PartialEq, KdlSerialize, KdlDeserialize)]
    #[kdl(document)]
    struct Records {
        #[kdl(children)]
        records: Vec<Record>,
    }
    for value in [
        Records { records: vec![] },
        Records {
            records: fixture().records,
        },
    ] {
        assert_eq!(
            from_str::<Records>(&to_string(&value).unwrap()).unwrap(),
            value
        );
    }
    let empty = Document::default();
    let table = NameTable::from_document(&empty);
    let wire = table.encode_document(empty.clone()).unwrap();
    assert_eq!(table.decode_document(&wire).unwrap(), empty);
}

#[test]
fn unknown_originals_aliases_and_mismatched_tables_are_rejected() {
    let doc = Document::parse("long_node long_field=1").unwrap();
    let table = NameTable::from_document(&doc);
    let unknown_node = Document::parse("other_node long_field=1").unwrap();
    let unknown_key = Document::parse("long_node other_field=1").unwrap();
    for unknown in [unknown_node, unknown_key] {
        assert!(matches!(
            table.encode_document(unknown),
            Err(CompactError::UnknownName(_))
        ));
    }
    let wire = table.encode_document(doc).unwrap();
    let wrong = NameTable::from_names(["other_node", "other_field"]);
    assert!(matches!(
        wrong.decode_document(&wire),
        Err(CompactError::TableMismatch { .. })
    ));
    for payload in ["unknown_name a=1", "b unknown_key=1"] {
        let forged = format!("{}\n{payload}", wire.lines().next().unwrap());
        assert!(matches!(
            table.decode_document(&forged),
            Err(CompactError::UnknownAlias(_))
        ));
    }
}

#[test]
fn invalid_tables_are_rejected_without_silent_overwrites() {
    for input in [
        "",
        "kdl-names 1; extra",
        "wrong 1",
        "kdl-names 1 2",
        "kdl-names version=1",
        "(typed)kdl-names 1",
        "kdl-names (i32)1",
        "kdl-names 1 { a first; a second; }",
        "kdl-names 1 { a same; b same; }",
        "kdl-names 1 { a long_name; b a; }",
        "kdl-names 1 { a; }",
        "kdl-names 1 { a 123; }",
        "kdl-names 1 { a original extra; }",
        "kdl-names 1 { a original prop=1; }",
        "kdl-names 1 { a original { child; }; }",
        "kdl-names 1 { (t)a original; }",
        "kdl-names 1 { a (t)original; }",
    ] {
        assert!(
            NameTable::from_kdl(input).is_err(),
            "accepted invalid table: {input}"
        );
    }
    assert!(matches!(
        NameTable::from_kdl("kdl-names 2"),
        Err(CompactError::UnsupportedVersion(2))
    ));
}

#[test]
fn imported_tables_canonicalize_order_and_support_quoted_names() {
    let a = NameTable::from_kdl("kdl-names 1 { a first_name; \"0\" second_name; }").unwrap();
    let b =
        NameTable::from_kdl("kdl-names 1 { \"0\" \"second_name\"; a \"first_name\"; }").unwrap();
    assert_eq!(a.id(), b.id());
    assert_eq!(a.to_kdl(), b.to_kdl());
    let doc = Document::parse("first_name second_name=2").unwrap();
    let wire = a.encode_document(doc.clone()).unwrap();
    assert!(wire.contains("\"0\"=2"));
    assert_eq!(b.decode_document(&wire).unwrap(), doc);
}

#[test]
fn table_id_has_a_versioned_format_independent_of_kdl_writer_layout() {
    // SHA-256 of domain + sorted (u64 BE byte-length, UTF-8) string pairs.
    let table = NameTable::from_names(["long_name", "a"]);
    assert_eq!(
        table.id(),
        "720c76b21c9457f52285de44ec74341f7a8fa8fc82b3e73bcdab6fbcdbc38c15"
    );
}

#[test]
fn malformed_message_headers_and_plain_input_are_rejected() {
    let table = NameTable::from_names(["node_name"]);
    for input in [
        "",
        "node_name",
        "kdl-message 1",
        "kdl-message 1 123",
        "kdl-message version=1 id=abc",
        "(t)kdl-message 1 abc",
        "kdl-message (t)1 abc",
        "kdl-message 1 (t)abc",
        "kdl-message 1 abc extra",
        "kdl-message 1 abc { a; }",
    ] {
        assert!(table.decode_document(input).is_err(), "accepted {input}");
    }
    assert!(matches!(
        table.decode_document("kdl-message 2 abc"),
        Err(CompactError::UnsupportedVersion(2))
    ));
    assert!(compact::decode_document("node_name").is_err());
    assert!(compact::decode_document("kdl-names 1 { a long_name; }; unknown_alias").is_err());
}

#[test]
fn three_character_aliases_skip_keywords_that_would_need_quotes() {
    let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut names = Vec::new();
    for a in alphabet {
        names.push(String::from_utf8(vec![*a]).unwrap());
        for b in alphabet {
            names.push(String::from_utf8(vec![*a, *b]).unwrap());
        }
    }
    'prefixes: for a in alphabet {
        for b in alphabet {
            for c in alphabet {
                let s = String::from_utf8(vec![*a, *b, *c]).unwrap();
                if s == "inf" {
                    break 'prefixes;
                }
                names.push(s);
            }
        }
    }
    names.push("zzzz".into());
    let table = NameTable::from_names(names);
    let wire = table
        .encode_document(Document::parse("zzzz").unwrap())
        .unwrap();
    assert!(
        wire.ends_with("ing\n"),
        "must skip quoted inf and allocate ing"
    );
    assert_eq!(
        table.decode_document(&wire).unwrap().nodes()[0].name(),
        "zzzz"
    );
}
