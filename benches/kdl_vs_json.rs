use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::collections::HashMap;
use std::hint::black_box;

// ============================================================
// KDL用の構造体（club-kdl）
// ============================================================

use club_kdl::{KdlDeserialize, KdlSerialize};

#[derive(Debug, Clone, KdlDeserialize, KdlSerialize)]
#[kdl(name = "service")]
struct KdlService {
    #[kdl(argument)]
    name: String,

    #[kdl(property)]
    image: String,

    #[kdl(child)]
    ports: Option<KdlPorts>,

    #[kdl(child_map, name = "env")]
    env: HashMap<String, String>,

    #[kdl(child)]
    volumes: Option<KdlVolumes>,

    #[kdl(child)]
    depends_on: Option<KdlDependsOn>,
}

#[derive(Debug, Clone, KdlDeserialize, KdlSerialize)]
#[kdl(name = "ports")]
struct KdlPorts {
    #[kdl(children, name = "port")]
    ports: Vec<KdlPort>,
}

#[derive(Debug, Clone, KdlDeserialize, KdlSerialize)]
#[kdl(name = "port")]
struct KdlPort {
    #[kdl(property)]
    host: u16,

    #[kdl(property)]
    container: u16,
}

#[derive(Debug, Clone, KdlDeserialize, KdlSerialize)]
#[kdl(name = "volumes")]
struct KdlVolumes {
    #[kdl(children, name = "volume")]
    volumes: Vec<KdlVolume>,
}

#[derive(Debug, Clone, KdlDeserialize, KdlSerialize)]
#[kdl(name = "volume")]
struct KdlVolume {
    #[kdl(argument)]
    host: String,

    #[kdl(argument)]
    container: String,

    #[kdl(property, default)]
    read_only: bool,
}

#[derive(Debug, Clone, KdlDeserialize, KdlSerialize)]
#[kdl(name = "depends_on")]
struct KdlDependsOn {
    #[kdl(arguments)]
    services: Vec<String>,
}

// ============================================================
// JSON用の構造体（serde_json）
// ============================================================

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonConfig {
    project: String,
    services: Vec<JsonService>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonService {
    name: String,
    image: String,
    #[serde(default)]
    ports: Vec<JsonPort>,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default)]
    volumes: Vec<JsonVolume>,
    #[serde(default)]
    depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonPort {
    host: u16,
    container: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonVolume {
    host: String,
    container: String,
    #[serde(default)]
    read_only: bool,
}

// ============================================================
// テストデータ
// ============================================================

const KDL_DATA: &str = r#"
project "creo-memories"

service "surrealdb" image="surrealdb/surrealdb:v2.4.0" {
    ports {
        port host=12000 container=8000
    }
    env {
        SURREAL_LOG "info"
        SURREAL_USER "root"
        SURREAL_PASS "root"
    }
    volumes {
        volume "./data/surrealdb" "/data"
    }
}

service "qdrant" image="qdrant/qdrant:v1.16.2" {
    ports {
        port host=12001 container=6333
        port host=12002 container=6334
    }
    volumes {
        volume "./data/qdrant" "/qdrant/storage"
    }
}

service "seaweedfs" image="chrislusf/seaweedfs:latest" {
    ports {
        port host=12100 container=8333
        port host=12101 container=9333
        port host=12102 container=8888
    }
    env {
        S3_ACCESS_KEY "seaweedfs"
        S3_SECRET_KEY "seaweedfs-local-dev"
    }
    volumes {
        volume "./data/seaweedfs" "/data"
        volume "./config/seaweedfs" "/etc/seaweedfs" read_only=#true
    }
}

service "creo-app-server" image="ghcr.io/chronista-club/creo-memories-app-server:latest" {
    depends_on "surrealdb" "qdrant"
    ports {
        port host=12301 container=3000
    }
    env {
        SURREALDB_URL "ws://surrealdb:8000/rpc"
        SURREALDB_NAMESPACE "creo"
        SURREALDB_DATABASE "memories"
        QDRANT_URL "http://qdrant:6333"
        PORT "3000"
        NODE_ENV "production"
    }
}

service "caddy" image="caddy:2-alpine" {
    depends_on "creo-app-server"
    ports {
        port host=80 container=80
        port host=443 container=443
    }
    volumes {
        volume "./config/caddy/Caddyfile" "/etc/caddy/Caddyfile" read_only=#true
        volume "./data/caddy/data" "/data"
        volume "./data/caddy/config" "/config"
    }
}
"#;

const JSON_DATA: &str = r#"
{
  "project": "creo-memories",
  "services": [
    {
      "name": "surrealdb",
      "image": "surrealdb/surrealdb:v2.4.0",
      "ports": [{"host": 12000, "container": 8000}],
      "env": {"SURREAL_LOG": "info", "SURREAL_USER": "root", "SURREAL_PASS": "root"},
      "volumes": [{"host": "./data/surrealdb", "container": "/data", "read_only": false}],
      "depends_on": []
    },
    {
      "name": "qdrant",
      "image": "qdrant/qdrant:v1.16.2",
      "ports": [{"host": 12001, "container": 6333}, {"host": 12002, "container": 6334}],
      "env": {},
      "volumes": [{"host": "./data/qdrant", "container": "/qdrant/storage", "read_only": false}],
      "depends_on": []
    },
    {
      "name": "seaweedfs",
      "image": "chrislusf/seaweedfs:latest",
      "ports": [{"host": 12100, "container": 8333}, {"host": 12101, "container": 9333}, {"host": 12102, "container": 8888}],
      "env": {"S3_ACCESS_KEY": "seaweedfs", "S3_SECRET_KEY": "seaweedfs-local-dev"},
      "volumes": [{"host": "./data/seaweedfs", "container": "/data", "read_only": false}, {"host": "./config/seaweedfs", "container": "/etc/seaweedfs", "read_only": true}],
      "depends_on": []
    },
    {
      "name": "creo-app-server",
      "image": "ghcr.io/chronista-club/creo-memories-app-server:latest",
      "ports": [{"host": 12301, "container": 3000}],
      "env": {"SURREALDB_URL": "ws://surrealdb:8000/rpc", "SURREALDB_NAMESPACE": "creo", "SURREALDB_DATABASE": "memories", "QDRANT_URL": "http://qdrant:6333", "PORT": "3000", "NODE_ENV": "production"},
      "volumes": [],
      "depends_on": ["surrealdb", "qdrant"]
    },
    {
      "name": "caddy",
      "image": "caddy:2-alpine",
      "ports": [{"host": 80, "container": 80}, {"host": 443, "container": 443}],
      "env": {},
      "volumes": [{"host": "./config/caddy/Caddyfile", "container": "/etc/caddy/Caddyfile", "read_only": true}, {"host": "./data/caddy/data", "container": "/data", "read_only": false}, {"host": "./data/caddy/config", "container": "/config", "read_only": false}],
      "depends_on": ["creo-app-server"]
    }
  ]
}
"#;

// ============================================================
// ベンチマーク
// ============================================================

fn read_services(doc: &club_kdl::Document<'_>) -> Vec<KdlService> {
    doc.nodes()
        .iter()
        .filter(|n| n.name() == "service")
        .map(|n| KdlService::from_kdl_node(n).unwrap())
        .collect()
}

/// KDL_DATA を `n` 回繰り返した大きめの入力 (n=50 で約 85KB)。
fn large_kdl(n: usize) -> String {
    KDL_DATA.repeat(n)
}

fn bench_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("read");

    // KDL読み込み (club-kdl 独自パーサ → struct)
    group.bench_function("kdl", |b| {
        b.iter(|| {
            let doc = club_kdl::Document::parse(black_box(KDL_DATA)).unwrap();
            black_box(read_services(&doc))
        })
    });

    // 比較用: v0.12 までの経路 (kdl-rs でパース → struct)
    group.bench_function("kdl-rs", |b| {
        b.iter(|| {
            let doc: kdl::KdlDocument = black_box(KDL_DATA).parse().unwrap();
            black_box(read_services(&club_kdl::Document::from(&doc)))
        })
    });

    // JSON読み込み
    group.bench_function("json", |b| {
        b.iter(|| {
            let config: JsonConfig = serde_json::from_str(black_box(JSON_DATA)).unwrap();
            black_box(config)
        })
    });

    group.finish();
}

fn bench_parse(c: &mut Criterion) {
    let large = large_kdl(50);
    let mut group = c.benchmark_group("parse");

    for (label, src) in [("small", KDL_DATA), ("large", large.as_str())] {
        group.throughput(Throughput::Bytes(src.len() as u64));
        group.bench_with_input(BenchmarkId::new("club-kdl", label), src, |b, src| {
            b.iter(|| black_box(club_kdl::Document::parse(black_box(src)).unwrap()))
        });
        group.bench_with_input(BenchmarkId::new("kdl-rs", label), src, |b, src| {
            b.iter(|| black_box(black_box(src).parse::<kdl::KdlDocument>().unwrap()))
        });
    }

    group.finish();
}

fn to_doc(services: &[KdlService]) -> club_kdl::Document<'_> {
    club_kdl::Document::from_nodes(services.iter().map(|s| s.to_kdl_node().unwrap()).collect())
}

fn bench_write(c: &mut Criterion) {
    // 事前にデータを準備
    let kdl_services = read_services(&club_kdl::Document::parse(KDL_DATA).unwrap());

    let json_config: JsonConfig = serde_json::from_str(JSON_DATA).unwrap();

    let mut group = c.benchmark_group("write");

    // KDL書き出し (struct → テキスト)
    group.bench_function("kdl", |b| {
        b.iter(|| {
            let doc = to_doc(black_box(&kdl_services));
            let mut out = String::with_capacity(2048);
            doc.write_to(&mut out);
            black_box(out)
        })
    });

    // 比較用: v0.12 までの経路 (kdl-rs のノードを組んでテキスト化)
    group.bench_function("kdl-rs", |b| {
        b.iter(|| {
            let doc = to_doc(black_box(&kdl_services));
            black_box(kdl::KdlDocument::from(&doc).to_string())
        })
    });

    // JSON書き出し
    group.bench_function("json", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&json_config)).unwrap();
            black_box(json)
        })
    });

    group.finish();
}

criterion_group!(benches, bench_read, bench_parse, bench_write);
criterion_main!(benches);
