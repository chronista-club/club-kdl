# unison-kdl

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)

Rust製の高速なKDL（KDL Document Language）シリアライズ/デシリアライズライブラリ。deriveマクロ付き。

> **KDLのためのSerde体験** - Rust構造体を定義し、deriveを付けるだけ。あとはunison-kdlにお任せ。

## 特徴

- **Deriveマクロ** - `#[derive(KdlDeserialize, KdlSerialize)]`で自動実装
- **属性ベースのマッピング** - `#[kdl(...)]`属性できめ細かく制御
- **可能な限りゼロコピー** - KDLソースから文字列を直接借用
- **型安全** - コンパイル時にKDLスキーマを保証

## インストール

```toml
[dependencies]
unison-kdl = { git = "https://github.com/chronista-club/unison-kdl" }
```

## クイックスタート

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

このKDLをパースできます：

```kdl
service "api" image="myapp:latest" version="1.0" {
    port host=8080 container=80
    port host=8443 container=443
}
```

## 属性

### コンテナ属性

| 属性 | 説明 |
|------|------|
| `#[kdl(name = "...")]` | KDLノード名（デフォルトはsnake_caseの構造体名） |

### フィールド属性

| 属性 | 説明 |
|------|------|
| `#[kdl(argument)]` | 位置引数にマッピング（自動インデックス） |
| `#[kdl(argument(index = N))]` | 特定のインデックスの引数にマッピング |
| `#[kdl(arguments)]` | すべての引数を`Vec<T>`に収集 |
| `#[kdl(property)]` | 名前付きプロパティにマッピング（`key=value`） |
| `#[kdl(property(rename = "..."))]` | 別名のプロパティにマッピング |
| `#[kdl(child)]` | 単一の子ノードにマッピング |
| `#[kdl(children, name = "...")]` | 名前で子ノードを`Vec<T>`に収集 |
| `#[kdl(child_map, name = "...")]` | 子ノードを`HashMap<String, String>`に収集 |
| `#[kdl(default)]` | 欠落時に`Default::default()`を使用 |
| `#[kdl(skip)]` | シリアライズ/デシリアライズをスキップ |

## 例

### 複数の引数

```rust
#[derive(KdlDeserialize, KdlSerialize)]
#[kdl(name = "volume")]
struct Volume {
    #[kdl(argument)]  // 最初の引数
    host: String,

    #[kdl(argument)]  // 2番目の引数
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

### 子ノードマップ（環境変数）

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

## 使い方

```rust
use unison_kdl::{KdlDeserialize, KdlNodeExt};

// KDLをパース
let kdl = r#"service "api" image="myapp""#;
let doc: kdl::KdlDocument = kdl.parse().unwrap();
let node = doc.nodes().first().unwrap();

// デシリアライズ
let service = Service::from_kdl_node(node).unwrap();

// シリアライズ
use unison_kdl::KdlSerialize;
let node = service.to_kdl_node().unwrap();
```

## サポートする型

- プリミティブ: `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`, `bool`
- 文字列: `String`, `&str`
- パス: `PathBuf`
- コレクション: `Vec<T>`, `HashMap<String, String>`
- オプショナル: `Option<T>`
- `FromKdlValue` / `ToKdlValue`を実装したカスタム型

## なぜ unison-kdl？

KDLは設定ファイルに最適なドキュメント言語です。unison-kdlは、RustでKDLを扱う際の開発体験を向上させます：

- **宣言的**: 構造体を定義し、属性を付けるだけ
- **型安全**: コンパイル時に構造を検証
- **柔軟**: 引数、プロパティ、子ノードを自由にマッピング
- **実用的**: [FleetFlow](https://github.com/chronista-club/fleetflow)で実戦投入済み

## 謝辞

[kdl](https://crates.io/crates/kdl)クレートをベースにしています。

## ライセンス

MIT License - 詳細は[LICENSE](LICENSE)を参照。
