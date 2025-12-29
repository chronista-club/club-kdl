# unison-kdl

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)

Rust製の高速なKDL（KDL Document Language）シリアライズ/デシリアライズライブラリである。deriveマクロを備える。

> 構造体を定義し、deriveを付すのみ。煩雑な実装は本ライブラリに委ねられたい。

## 特徴

- **Deriveマクロ** - `#[derive(KdlDeserialize, KdlSerialize)]`にて自動実装
- **属性による対応付け** - `#[kdl(...)]`属性により精緻な制御が可能
- **可能な限り零複写** - KDL原文より文字列を直接借用する
- **型安全** - 翻訳時にKDL構造の正当性を保証する

## 導入

```toml
[dependencies]
unison-kdl = { git = "https://github.com/chronista-club/unison-kdl" }
```

## 手始めに

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

上記の構造体は、以下のKDLを解析する：

```kdl
service "api" image="myapp:latest" version="1.0" {
    port host=8080 container=80
    port host=8443 container=443
}
```

## 属性一覧

### 構造体属性

| 属性 | 説明 |
|------|------|
| `#[kdl(name = "...")]` | KDL節点名を指定する。省略時は構造体名をsnake_caseに変換して用いる |

### 欄属性

| 属性 | 説明 |
|------|------|
| `#[kdl(argument)]` | 位置引数に対応付ける。順序は自動的に定まる |
| `#[kdl(argument(index = N))]` | 特定の位置の引数に対応付ける |
| `#[kdl(arguments)]` | 全引数を`Vec<T>`として収集する |
| `#[kdl(property)]` | 名前付き属性（`key=value`形式）に対応付ける |
| `#[kdl(property(rename = "..."))]` | 異なる名称の属性に対応付ける |
| `#[kdl(child)]` | 単一の子節点に対応付ける |
| `#[kdl(children, name = "...")]` | 指定名の子節点群を`Vec<T>`として収集する |
| `#[kdl(child_map, name = "...")]` | 子節点群を`HashMap<String, String>`として収集する |
| `#[kdl(default)]` | 欠落時に`Default::default()`を用いる |
| `#[kdl(skip)]` | 変換処理より除外する |

## 用例

### 複数の引数

```rust
#[derive(KdlDeserialize, KdlSerialize)]
#[kdl(name = "volume")]
struct Volume {
    #[kdl(argument)]  // 第一引数
    host: String,

    #[kdl(argument)]  // 第二引数
    container: String,

    #[kdl(property, default)]
    read_only: bool,
}
```

```kdl
volume "./data" "/app/data" read_only=#true
```

### 引数の収集

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

### 子節点の辞書化

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

## 使用法

```rust
use unison_kdl::{KdlDeserialize, KdlNodeExt};

// KDLを解析する
let kdl = r#"service "api" image="myapp""#;
let doc: kdl::KdlDocument = kdl.parse().unwrap();
let node = doc.nodes().first().unwrap();

// 復元する
let service = Service::from_kdl_node(node).unwrap();

// 直列化する
use unison_kdl::KdlSerialize;
let node = service.to_kdl_node().unwrap();
```

## 対応する型

- 基本型: `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`, `bool`
- 文字列: `String`, `&str`
- 経路: `PathBuf`
- 集合: `Vec<T>`, `HashMap<String, String>`
- 任意: `Option<T>`
- `FromKdlValue` / `ToKdlValue`を実装した独自型

## 本ライブラリを用いる理由

KDLは設定記述に適したドキュメント言語である。本ライブラリは、RustにてKDLを扱う際の開発体験を向上せしめる。

- **宣言的**: 構造体を定義し、属性を付すのみ
- **型安全**: 翻訳時に構造の正当性を検証する
- **柔軟**: 引数、属性、子節点を自在に対応付ける
- **実用**: [FleetFlow](https://github.com/chronista-club/fleetflow)にて実戦に供されている

## 謝辞

本ライブラリは[kdl](https://crates.io/crates/kdl)クレートを基盤とする。

## 許諾

MIT License - 詳細は[LICENSE](LICENSE)を参照されたい。
