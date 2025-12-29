# unison-kdl

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)

High-performance KDL (KDL Document Language) serialization and deserialization for Rust with derive macros.

> **Serde-like experience for KDL** - Define your Rust structs, add derives, and let unison-kdl handle the rest.

## Features

- **Derive macros** - `#[derive(KdlDeserialize, KdlSerialize)]` for automatic implementation
- **Attribute-based mapping** - Fine-grained control with `#[kdl(...)]` attributes
- **Zero-copy where possible** - Borrow strings directly from KDL source
- **Type-safe** - Compile-time guarantees for your KDL schema

## Installation

```toml
[dependencies]
unison-kdl = { git = "https://github.com/chronista-club/unison-kdl" }
```

## Quick Start

```rust
use unison_kdl::{KdlDeserialize, KdlSerialize};

#[derive(Debug, KdlDeserialize, KdlSerialize)]
#[kdl(name = "service")]
struct Service {
    #[kdl(argument)]
    name: String,

    #[kdl(property)]
    image: String,

    #[kdl(property)]
    version: Option<String>,

    #[kdl(children, name = "port")]
    ports: Vec<Port>,
}

#[derive(Debug, KdlDeserialize, KdlSerialize)]
#[kdl(name = "port")]
struct Port {
    #[kdl(property)]
    host: u16,

    #[kdl(property)]
    container: u16,
}
```

Parses this KDL:

```kdl
service "api" image="myapp:latest" version="1.0" {
    port host=8080 container=80
    port host=8443 container=443
}
```

## Attributes

### Container Attributes

| Attribute | Description |
|-----------|-------------|
| `#[kdl(name = "...")]` | KDL node name (defaults to snake_case struct name) |

### Field Attributes

| Attribute | Description |
|-----------|-------------|
| `#[kdl(argument)]` | Map to positional argument (auto-indexed) |
| `#[kdl(argument(index = N))]` | Map to specific argument index |
| `#[kdl(arguments)]` | Collect all arguments into `Vec<T>` |
| `#[kdl(property)]` | Map to named property (`key=value`) |
| `#[kdl(property(rename = "..."))]` | Map to property with different name |
| `#[kdl(child)]` | Map to single child node |
| `#[kdl(children, name = "...")]` | Collect child nodes by name into `Vec<T>` |
| `#[kdl(child_map, name = "...")]` | Collect child nodes into `HashMap<String, String>` |
| `#[kdl(default)]` | Use `Default::default()` if missing |
| `#[kdl(skip)]` | Skip during serialization/deserialization |

## Examples

### Multiple Arguments

```rust
#[derive(KdlDeserialize, KdlSerialize)]
#[kdl(name = "volume")]
struct Volume {
    #[kdl(argument)]  // First argument
    host: String,

    #[kdl(argument)]  // Second argument
    container: String,

    #[kdl(property, default)]
    read_only: bool,
}
```

```kdl
volume "./data" "/app/data" read_only=#true
```

### Collecting Arguments

```rust
#[derive(KdlDeserialize, KdlSerialize)]
#[kdl(name = "depends_on")]
struct DependsOn {
    #[kdl(arguments)]
    services: Vec<String>,
}
```

```kdl
depends_on "db" "redis" "cache"
```

### Child Map (Environment Variables)

```rust
#[derive(KdlDeserialize, KdlSerialize)]
#[kdl(name = "service")]
struct Service {
    #[kdl(argument)]
    name: String,

    #[kdl(child_map, name = "env")]
    environment: HashMap<String, String>,
}
```

```kdl
service "api" {
    env {
        DATABASE_URL "postgres://localhost/db"
        API_KEY "secret"
    }
}
```

## Usage

```rust
use unison_kdl::{KdlDeserialize, KdlNodeExt};

// Parse KDL
let kdl = r#"service "api" image="myapp""#;
let doc: kdl::KdlDocument = kdl.parse().unwrap();
let node = doc.nodes().first().unwrap();

// Deserialize
let service = Service::from_kdl_node(node).unwrap();

// Serialize
use unison_kdl::KdlSerialize;
let node = service.to_kdl_node().unwrap();
```

## Supported Types

- Primitives: `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`, `bool`
- Strings: `String`, `&str`
- Paths: `PathBuf`
- Collections: `Vec<T>`, `HashMap<String, String>`
- Optional: `Option<T>`
- Custom types implementing `FromKdlValue` / `ToKdlValue`

## Why unison-kdl?

KDLは設定ファイルに最適なドキュメント言語です。unison-kdlは、RustでKDLを扱う際の開発体験を向上させます：

- **宣言的**: 構造体を定義し、属性を付けるだけ
- **型安全**: コンパイル時に構造を検証
- **柔軟**: 引数、プロパティ、子ノードを自由にマッピング
- **実用的**: [FleetFlow](https://github.com/chronista-club/fleetflow)で実戦投入済み

## Acknowledgements

Built on top of the excellent [kdl](https://crates.io/crates/kdl) crate.

## License

MIT License - see [LICENSE](LICENSE) for details.
