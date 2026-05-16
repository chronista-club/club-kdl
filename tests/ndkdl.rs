//! NDKDL (newline-delimited KDL) の append_node ヘルパーのテスト。

use club_kdl::{KdlDeserialize, KdlSerialize, append_node};

#[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
#[kdl(name = "event")]
struct Event {
    #[kdl(argument)]
    kind: String,
    #[kdl(property)]
    seq: u32,
}

#[derive(Debug, KdlDeserialize)]
#[kdl(document)]
struct Log {
    #[kdl(children)]
    events: Vec<Event>,
}

/// プロセス + テスト名で一意な一時ファイルパス (並列テストでの衝突を避ける)。
fn temp_path(name: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("club-kdl-ndkdl-{}-{name}.kdl", std::process::id()));
    p
}

#[test]
fn append_to_new_file_creates_it() {
    let path = temp_path("new");
    let _ = std::fs::remove_file(&path);

    append_node(
        &path,
        &Event {
            kind: "start".into(),
            seq: 1,
        },
    )
    .unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("event"));
    assert!(content.contains("start"));

    std::fs::remove_file(&path).ok();
}

#[test]
fn append_twice_yields_two_nodes() {
    let path = temp_path("twice");
    let _ = std::fs::remove_file(&path);

    append_node(
        &path,
        &Event {
            kind: "a".into(),
            seq: 1,
        },
    )
    .unwrap();
    append_node(
        &path,
        &Event {
            kind: "b".into(),
            seq: 2,
        },
    )
    .unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let log: Log = club_kdl::from_str(&text).unwrap();
    assert_eq!(log.events.len(), 2);
    assert_eq!(log.events[0].kind, "a");
    assert_eq!(log.events[1].kind, "b");

    std::fs::remove_file(&path).ok();
}

#[test]
fn appended_file_is_valid_document() {
    let path = temp_path("doc");
    let _ = std::fs::remove_file(&path);

    for i in 1..=3 {
        append_node(
            &path,
            &Event {
                kind: format!("e{i}"),
                seq: i,
            },
        )
        .unwrap();
    }

    let text = std::fs::read_to_string(&path).unwrap();
    let log: Log = club_kdl::from_str(&text).unwrap();
    assert_eq!(log.events.len(), 3);
    assert_eq!(log.events[2].seq, 3);

    std::fs::remove_file(&path).ok();
}
