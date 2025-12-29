# unison-kdl

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

RustでKDLファイルを読み書きするためのライブラリ。deriveマクロで構造体とKDLを相互変換できる。

## インストール

```toml
[dependencies]
unison-kdl = { git = "https://github.com/chronista-club/unison-kdl" }
```

## 基本的な使い方

```rust
use unison_kdl::{KdlDeserialize, KdlSerialize};

#[derive(Debug, KdlDeserialize, KdlSerialize)]
#[kdl(name = "service")]
struct Service {
    #[kdl(argument)]
    name: String,

    #[kdl(property)]
    image: String,

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

このコードで以下のKDLを読み込める：

```kdl
service "api" image="myapp:latest" {
    port host=8080 container=80
    port host=8443 container=443
}
```

## 属性一覧

### 構造体に付ける属性

| 属性 | 説明 |
|------|------|
| `#[kdl(name = "...")]` | KDLのノード名を指定 |

### フィールドに付ける属性

| 属性 | 説明 |
|------|------|
| `#[kdl(argument)]` | 位置引数にマッピング |
| `#[kdl(argument(index = N))]` | N番目の引数にマッピング |
| `#[kdl(arguments)]` | 全引数を`Vec<T>`で取得 |
| `#[kdl(property)]` | `key=value`形式の属性にマッピング |
| `#[kdl(property(rename = "..."))]` | 別名の属性にマッピング |
| `#[kdl(child)]` | 子ノード1つにマッピング |
| `#[kdl(children, name = "...")]` | 指定名の子ノードを`Vec<T>`で取得 |
| `#[kdl(child_map, name = "...")]` | 子ノードを`HashMap`で取得 |
| `#[kdl(default)]` | 値がない場合はデフォルト値を使用 |
| `#[kdl(skip)]` | 変換対象から除外 |

## 例

### 複数の引数

```rust
#[derive(KdlDeserialize, KdlSerialize)]
#[kdl(name = "volume")]
struct Volume {
    #[kdl(argument)]
    host: String,

    #[kdl(argument)]
    container: String,
}
```

```kdl
volume "./data" "/app/data"
```

### 引数をまとめて取得

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

### 子ノードをHashMapで取得

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

## 読み込みと書き出し

```rust
use unison_kdl::{KdlDeserialize, KdlSerialize, KdlNodeExt};

// 読み込み
let kdl = r#"service "api" image="myapp""#;
let doc: kdl::KdlDocument = kdl.parse().unwrap();
let node = doc.nodes().first().unwrap();
let service = Service::from_kdl_node(node).unwrap();

// 書き出し
let node = service.to_kdl_node().unwrap();
```

## 対応している型

- 数値: `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`
- 真偽値: `bool`
- 文字列: `String`
- パス: `PathBuf`
- コレクション: `Vec<T>`, `HashMap<String, String>`
- Option: `Option<T>`

## ライセンス

MIT
