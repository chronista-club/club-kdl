//! Integration tests for unison-kdl

use unison_kdl::{KdlDeserialize, KdlDocument, KdlNodeExt, KdlSerialize};

// ============================================================================
// Basic test structures
// ============================================================================

#[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
#[kdl(name = "port")]
struct Port {
    #[kdl(property)]
    host: u16,
    #[kdl(property)]
    container: u16,
    #[kdl(property)]
    protocol: Option<String>,
}

#[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
#[kdl(name = "volume")]
struct Volume {
    #[kdl(property)]
    host: String,
    #[kdl(property)]
    container: String,
    #[kdl(property)]
    read_only: Option<bool>,
}

#[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
#[kdl(name = "service")]
struct Service {
    #[kdl(argument)]
    name: String,

    #[kdl(property)]
    image: Option<String>,

    #[kdl(property)]
    restart: Option<String>,

    #[kdl(children, name = "port")]
    ports: Vec<Port>,

    #[kdl(children, name = "volume")]
    volumes: Vec<Volume>,
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_deserialize_simple_service() {
    // Note: Using key=value syntax for properties
    let kdl = r#"
        service "surrealdb" image="surrealdb/surrealdb:v2.4.0" restart="unless-stopped"
    "#;

    let doc: KdlDocument = kdl.parse().unwrap();
    let node = doc.nodes().first().unwrap();
    let service: Service = Service::from_kdl_node(node).unwrap();

    assert_eq!(service.name, "surrealdb");
    assert_eq!(
        service.image,
        Some("surrealdb/surrealdb:v2.4.0".to_string())
    );
    assert_eq!(service.restart, Some("unless-stopped".to_string()));
}

#[test]
fn test_deserialize_with_ports() {
    // KDL style: direct port nodes without wrapper
    let kdl = r#"
        service "qdrant" image="qdrant/qdrant:v1.16.2" {
            port host=12001 container=6333
            port host=12002 container=6334
        }
    "#;

    let doc: KdlDocument = kdl.parse().unwrap();
    let node = doc.nodes().first().unwrap();
    let service: Service = Service::from_kdl_node(node).unwrap();

    assert_eq!(service.name, "qdrant");
    assert_eq!(service.ports.len(), 2);
    assert_eq!(service.ports[0].host, 12001);
    assert_eq!(service.ports[0].container, 6333);
    assert_eq!(service.ports[1].host, 12002);
    assert_eq!(service.ports[1].container, 6334);
}

#[test]
fn test_deserialize_with_volumes() {
    // KDL style: direct volume nodes without wrapper
    let kdl = r#"
        service "surrealdb" {
            volume host="/data/surrealdb" container="/data"
            volume host="/config" container="/etc/config" read_only=#true
        }
    "#;

    let doc: KdlDocument = kdl.parse().unwrap();
    let node = doc.nodes().first().unwrap();
    let service: Service = Service::from_kdl_node(node).unwrap();

    assert_eq!(service.volumes.len(), 2);
    assert_eq!(service.volumes[0].host, "/data/surrealdb");
    assert_eq!(service.volumes[0].read_only, None);
    assert_eq!(service.volumes[1].read_only, Some(true));
}

#[test]
fn test_serialize_simple() {
    let port = Port {
        host: 8080,
        container: 80,
        protocol: None,
    };

    let node = port.to_kdl_node().unwrap();
    assert_eq!(node.name().value(), "port");

    // Check properties
    assert_eq!(node.prop("host").and_then(|v| v.as_integer()), Some(8080));
    assert_eq!(
        node.prop("container").and_then(|v| v.as_integer()),
        Some(80)
    );
}

#[test]
fn test_roundtrip() {
    let original = Port {
        host: 3000,
        container: 3000,
        protocol: Some("tcp".to_string()),
    };

    // Serialize
    let node = original.to_kdl_node().unwrap();

    // Deserialize back
    let restored: Port = Port::from_kdl_node(&node).unwrap();

    assert_eq!(original, restored);
}

#[test]
fn test_multiple_children() {
    // Test that we can collect multiple children of the same type
    let kdl = r#"
        service "app" {
            port host=80 container=80
            port host=443 container=443
            volume host="/app" container="/app"
        }
    "#;

    let doc: KdlDocument = kdl.parse().unwrap();
    let node = doc.nodes().first().unwrap();

    // Access port children directly
    let ports = node.children_by_name("port");
    assert_eq!(ports.len(), 2);

    // Access volume children directly
    let volumes = node.children_by_name("volume");
    assert_eq!(volumes.len(), 1);
}

// ============================================================================
// Test new features: arguments and child_map
// ============================================================================

/// Test `#[kdl(arguments)]` - collect all args into Vec
#[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
#[kdl(name = "depends_on")]
struct DependsOn {
    #[kdl(arguments)]
    services: Vec<String>,
}

#[test]
fn test_arguments_collection() {
    let kdl = r#"depends_on "db" "redis" "cache""#;

    let doc: KdlDocument = kdl.parse().unwrap();
    let node = doc.nodes().first().unwrap();
    let deps: DependsOn = DependsOn::from_kdl_node(node).unwrap();

    assert_eq!(deps.services, vec!["db", "redis", "cache"]);
}

#[test]
fn test_arguments_roundtrip() {
    let original = DependsOn {
        services: vec!["postgres".to_string(), "redis".to_string()],
    };

    let node = original.to_kdl_node().unwrap();
    let restored: DependsOn = DependsOn::from_kdl_node(&node).unwrap();

    assert_eq!(original, restored);
}

/// Test `#[kdl(child_map)]` - collect child nodes into HashMap
#[derive(Debug, PartialEq, KdlDeserialize, KdlSerialize)]
#[kdl(name = "service")]
struct ServiceWithEnv {
    #[kdl(argument)]
    name: String,

    #[kdl(child_map, name = "env")]
    environment: std::collections::HashMap<String, String>,
}

#[test]
fn test_child_map_with_wrapper() {
    let kdl = r#"
        service "api" {
            env {
                DATABASE_URL "postgres://localhost/db"
                REDIS_URL "redis://localhost"
                LOG_LEVEL "debug"
            }
        }
    "#;

    let doc: KdlDocument = kdl.parse().unwrap();
    let node = doc.nodes().first().unwrap();
    let svc: ServiceWithEnv = ServiceWithEnv::from_kdl_node(node).unwrap();

    assert_eq!(svc.name, "api");
    assert_eq!(svc.environment.len(), 3);
    assert_eq!(
        svc.environment.get("DATABASE_URL"),
        Some(&"postgres://localhost/db".to_string())
    );
    assert_eq!(svc.environment.get("LOG_LEVEL"), Some(&"debug".to_string()));
}

#[test]
fn test_child_map_roundtrip() {
    let mut env = std::collections::HashMap::new();
    env.insert("KEY1".to_string(), "value1".to_string());
    env.insert("KEY2".to_string(), "value2".to_string());

    let original = ServiceWithEnv {
        name: "test".to_string(),
        environment: env,
    };

    let node = original.to_kdl_node().unwrap();
    let restored: ServiceWithEnv = ServiceWithEnv::from_kdl_node(&node).unwrap();

    assert_eq!(original.name, restored.name);
    assert_eq!(original.environment.len(), restored.environment.len());
    assert_eq!(
        original.environment.get("KEY1"),
        restored.environment.get("KEY1")
    );
}
