//! `to_string` writes derived types straight to text. These tests pin its
//! output to the tree path (`to_kdl_doc` → `Document` → text), which is the
//! reference: both must produce byte-identical KDL for every derive shape.

use std::collections::HashMap;

use club_kdl::{KdlSerialize, ToKdlValue};

/// Text produced by building the tree first, then writing it.
fn via_tree<T: KdlSerialize>(value: &T) -> String {
    value.to_kdl_doc().unwrap().to_string()
}

fn assert_same<T: KdlSerialize>(value: &T) {
    let direct = club_kdl::to_string(value).unwrap();
    assert_eq!(direct, via_tree(value));
    // Must also read back as valid KDL.
    club_kdl::Document::parse(&direct).unwrap();
}

#[derive(KdlSerialize)]
#[kdl(name = "port")]
struct Port {
    #[kdl(property)]
    host: u16,
    #[kdl(property)]
    container: u16,
}

#[derive(KdlSerialize)]
#[kdl(name = "meta")]
struct Meta {
    #[kdl(property)]
    owner: String,
    #[kdl(child, unwrap_arg)]
    note: Option<String>,
}

#[derive(KdlSerialize, Clone, Copy)]
enum Level {
    Info,
    #[kdl(rename = "warn")]
    Warning,
}

#[derive(KdlSerialize)]
#[kdl(name = "service")]
struct Service {
    #[kdl(argument)]
    name: String,
    #[kdl(argument(index = 1))]
    second: Option<String>,
    #[kdl(arguments)]
    extra: Vec<i64>,
    #[kdl(property)]
    image: String,
    #[kdl(property(rename = "log-level"))]
    level: Level,
    #[kdl(property)]
    ratio: Option<f64>,
    #[kdl(children, name = "port")]
    ports: Vec<Port>,
    #[kdl(child, unwrap_arg)]
    command: String,
    #[kdl(child, unwrap_args)]
    tags: Vec<String>,
    #[kdl(child_map, name = "env")]
    env: HashMap<String, String>,
    #[kdl(flatten)]
    meta: Meta,
    #[kdl(skip)]
    _cache: Vec<u8>,
}

#[derive(KdlSerialize)]
enum Command {
    Move {
        #[kdl(property)]
        x: i64,
        #[kdl(property)]
        y: i64,
    },
    Resize(Port),
    Quit,
    Echo {
        #[kdl(arguments)]
        words: Vec<String>,
        #[kdl(children, name = "port")]
        ports: Vec<Port>,
    },
}

#[derive(KdlSerialize)]
#[kdl(document)]
struct Compose {
    #[kdl(children, name = "service")]
    services: Vec<Service>,
    #[kdl(child)]
    level_hint: Option<Meta>,
}

fn service(name: &str, with_children: bool) -> Service {
    let mut env = HashMap::new();
    if with_children {
        env.insert("PORT".to_string(), "3000".to_string());
    }
    Service {
        name: name.to_string(),
        second: with_children.then(|| "two words".to_string()),
        extra: if with_children { vec![1, -2] } else { vec![] },
        image: "ghcr.io/a/b:1.0".to_string(),
        level: Level::Warning,
        ratio: with_children.then_some(0.5),
        ports: if with_children {
            vec![Port {
                host: 80,
                container: 8080,
            }]
        } else {
            vec![]
        },
        command: "run \"quoted\"\n".to_string(),
        tags: if with_children {
            vec!["a".into(), "b c".into()]
        } else {
            vec![]
        },
        env,
        meta: Meta {
            owner: "team".to_string(),
            note: with_children.then(|| "hello".to_string()),
        },
        _cache: vec![],
    }
}

#[test]
fn struct_with_every_field_kind() {
    assert_same(&service("api", true));
    assert_same(&service("bare", false));
}

#[test]
fn flattened_entries_and_children_keep_tree_order() {
    // Flattened entries come after the parent's own entries, flattened
    // children after the parent's own children.
    let text = club_kdl::to_string(&service("api", true)).unwrap();
    let first_line = text.lines().next().unwrap();
    assert!(first_line.ends_with("owner=team {"), "{first_line}");
    assert_same(&service("api", true));
}

#[test]
fn data_enum_variants() {
    assert_same(&Command::Move { x: 1, y: -2 });
    assert_same(&Command::Resize(Port {
        host: 1,
        container: 2,
    }));
    assert_same(&Command::Quit);
    assert_same(&Command::Echo {
        words: vec!["hi".into()],
        ports: vec![Port {
            host: 3,
            container: 4,
        }],
    });
    assert_eq!(
        club_kdl::to_string(&Command::Resize(Port {
            host: 1,
            container: 2
        }))
        .unwrap(),
        "resize host=1 container=2\n"
    );
}

#[test]
fn scalar_enum() {
    assert_same(&Level::Info);
    assert_eq!(Level::Warning.to_kdl_value().as_string(), Some("warn"));
}

#[test]
fn document_struct() {
    let doc = Compose {
        services: vec![service("a", true), service("b", false)],
        level_hint: Some(Meta {
            owner: "x".into(),
            note: None,
        }),
    };
    assert_same(&doc);
    assert_same(&Compose {
        services: vec![],
        level_hint: None,
    });
}

#[test]
fn to_string_into_appends() {
    let mut out = String::from("// header\n");
    club_kdl::to_string_into(
        &Port {
            host: 1,
            container: 2,
        },
        &mut out,
    )
    .unwrap();
    club_kdl::to_string_into(
        &Port {
            host: 3,
            container: 4,
        },
        &mut out,
    )
    .unwrap();
    assert_eq!(
        out,
        "// header\nport host=1 container=2\nport host=3 container=4\n"
    );
}
