use std::collections::HashMap;
use std::time::Instant;

use unison_kdl::{KdlDeserialize, KdlSerialize};

// KDL構造体
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

// JSON構造体
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

const JSON_DATA: &str = r#"{"project":"creo-memories","services":[{"name":"surrealdb","image":"surrealdb/surrealdb:v2.4.0","ports":[{"host":12000,"container":8000}],"env":{"SURREAL_LOG":"info","SURREAL_USER":"root","SURREAL_PASS":"root"},"volumes":[{"host":"./data/surrealdb","container":"/data","read_only":false}],"depends_on":[]},{"name":"qdrant","image":"qdrant/qdrant:v1.16.2","ports":[{"host":12001,"container":6333},{"host":12002,"container":6334}],"env":{},"volumes":[{"host":"./data/qdrant","container":"/qdrant/storage","read_only":false}],"depends_on":[]},{"name":"seaweedfs","image":"chrislusf/seaweedfs:latest","ports":[{"host":12100,"container":8333},{"host":12101,"container":9333},{"host":12102,"container":8888}],"env":{"S3_ACCESS_KEY":"seaweedfs","S3_SECRET_KEY":"seaweedfs-local-dev"},"volumes":[{"host":"./data/seaweedfs","container":"/data","read_only":false},{"host":"./config/seaweedfs","container":"/etc/seaweedfs","read_only":true}],"depends_on":[]},{"name":"creo-app-server","image":"ghcr.io/chronista-club/creo-memories-app-server:latest","ports":[{"host":12301,"container":3000}],"env":{"SURREALDB_URL":"ws://surrealdb:8000/rpc","SURREALDB_NAMESPACE":"creo","SURREALDB_DATABASE":"memories","QDRANT_URL":"http://qdrant:6333","PORT":"3000","NODE_ENV":"production"},"volumes":[],"depends_on":["surrealdb","qdrant"]},{"name":"caddy","image":"caddy:2-alpine","ports":[{"host":80,"container":80},{"host":443,"container":443}],"env":{},"volumes":[{"host":"./config/caddy/Caddyfile","container":"/etc/caddy/Caddyfile","read_only":true},{"host":"./data/caddy/data","container":"/data","read_only":false},{"host":"./data/caddy/config","container":"/config","read_only":false}],"depends_on":["creo-app-server"]}]}"#;

const ITERATIONS: u32 = 10_000;

fn main() {
    println!("=== KDL vs JSON ベンチマーク ({ITERATIONS}回) ===\n");

    // ウォームアップ
    for _ in 0..100 {
        let doc: kdl::KdlDocument = KDL_DATA.parse().unwrap();
        let _: Vec<KdlService> = doc
            .nodes()
            .iter()
            .filter(|n| n.name().value() == "service")
            .map(|n| KdlService::from_kdl_node(n).unwrap())
            .collect();
        let _: JsonConfig = serde_json::from_str(JSON_DATA).unwrap();
    }

    // === READ ===
    println!("【Read（文字列 → 構造体）】");

    // KDL Read
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let doc: kdl::KdlDocument = KDL_DATA.parse().unwrap();
        let _: Vec<KdlService> = doc
            .nodes()
            .iter()
            .filter(|n| n.name().value() == "service")
            .map(|n| KdlService::from_kdl_node(n).unwrap())
            .collect();
    }
    let kdl_read_total = start.elapsed();
    let kdl_read_avg = kdl_read_total / ITERATIONS;

    // JSON Read
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _: JsonConfig = serde_json::from_str(JSON_DATA).unwrap();
    }
    let json_read_total = start.elapsed();
    let json_read_avg = json_read_total / ITERATIONS;

    println!("  KDL:  合計 {:>8.2?}  平均 {:>8.2?}", kdl_read_total, kdl_read_avg);
    println!("  JSON: 合計 {:>8.2?}  平均 {:>8.2?}", json_read_total, json_read_avg);
    println!("  倍率: JSON が {:.1}倍 速い", kdl_read_avg.as_nanos() as f64 / json_read_avg.as_nanos() as f64);

    // === WRITE ===
    println!("\n【Write（構造体 → 文字列）】");

    // 事前にデータ準備
    let doc: kdl::KdlDocument = KDL_DATA.parse().unwrap();
    let kdl_services: Vec<KdlService> = doc
        .nodes()
        .iter()
        .filter(|n| n.name().value() == "service")
        .map(|n| KdlService::from_kdl_node(n).unwrap())
        .collect();
    let json_config: JsonConfig = serde_json::from_str(JSON_DATA).unwrap();

    // KDL Write
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _: Vec<kdl::KdlNode> = kdl_services
            .iter()
            .map(|s| s.to_kdl_node().unwrap())
            .collect();
    }
    let kdl_write_total = start.elapsed();
    let kdl_write_avg = kdl_write_total / ITERATIONS;

    // JSON Write
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = serde_json::to_string(&json_config).unwrap();
    }
    let json_write_total = start.elapsed();
    let json_write_avg = json_write_total / ITERATIONS;

    println!("  KDL:  合計 {:>8.2?}  平均 {:>8.2?}", kdl_write_total, kdl_write_avg);
    println!("  JSON: 合計 {:>8.2?}  平均 {:>8.2?}", json_write_total, json_write_avg);
    println!("  倍率: JSON が {:.1}倍 速い", kdl_write_avg.as_nanos() as f64 / json_write_avg.as_nanos() as f64);

    println!("\n=== データサイズ ===");
    println!("  KDL:  {} bytes", KDL_DATA.len());
    println!("  JSON: {} bytes", JSON_DATA.len());
}
