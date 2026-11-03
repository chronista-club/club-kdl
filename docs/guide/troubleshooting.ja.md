# トラブルシューティング

**[English](./troubleshooting.md)** | 日本語

club-kdl でよく遭遇するエラーの原因と対処をまとめる。

## ライフタイム

### `implicit elided lifetime not allowed here` (E0726)

借用フィールド（`&'a str`）を持つ構造体を `async fn` の引数に使い、その型のライフタイム引数を省くと、次のエラーになる。

```rust
#[derive(KdlDeserialize)]
#[kdl(name = "service")]
struct Service<'a> {
    #[kdl(argument)]
    name: &'a str,
}

async fn handle(service: Service) {
    //                  ^^^^^^^ E0726: specify the lifetime argument
    println!("{}", service.name);
}
```

**原因**: この `async fn` の引数では、型のライフタイム引数を明示する必要がある。edition 2024 でライフタイムの省略が一律に禁止されたわけではない。ローカル変数の `let s: Service = ...` などでは、引き続き推論できる。

**対処**: `Service<'_>` と書き、匿名ライフタイムを明示する。

```rust
async fn handle(service: Service<'_>) {
    println!("{}", service.name);
}
```

借用が不要なら、フィールドを `String` にすることでライフタイム引数そのものをなくせる。[Rust の E0726 解説](https://doc.rust-lang.org/error_codes/E0726.html)も参照。

## 子ノードの対応付け

### `#[kdl(children)]` で子ノードが集まらない

```rust
#[derive(KdlDeserialize)]
#[kdl(document)]
struct Config {
    #[kdl(children)]
    stages: Vec<Stage>,
}
```

**原因**: `#[kdl(children)]` は、子の型に付けた `#[kdl(name = "...")]` からノード名を決める。子の型に `#[kdl(name)]` がないと、フィールド名の `stages` を使うため、実際のノード名 `stage` と一致しない。

**対処**: 子の型に `#[kdl(name)]` を付けるか、収集する側で名前を明示する。

```rust
#[kdl(children(name = "stage"))]
stages: Vec<Stage>,
```

### `#[kdl(child)]` に `Vec<T>` を指定している

**原因**: `child` は単一の子ノード用で、繰り返しには使えない。

**対処**: 繰り返しには `#[kdl(children)]` と `Vec<T>` を使う。省略可能な単一の子には、`#[kdl(child)]` と `Option<T>` を使う。

## 文書全体の読み込み

### トップレベルに複数のノードがある KDL を読めない

```kdl
stage "build"
stage "deploy"
service "api"
```

**原因**: 既定では「1 ノードを 1 構造体に対応させる」動作になる。複数のトップレベルノードを一つの構造体にまとめるには、`#[kdl(document)]` が必要。

**対処**:

```rust
#[derive(KdlDeserialize)]
#[kdl(document)]
struct Config {
    #[kdl(children)]
    stages: Vec<Stage>,
    #[kdl(children)]
    services: Vec<Service>,
}
```

## 値の型

### `type mismatch: expected ...`

**原因**: KDL の値の種類（文字列・整数・浮動小数点数・真偽値）が Rust のフィールド型と一致していない。たとえば `port="8080"` は文字列なので、`u16` では受け取れない。

**対処**: KDL 側を `port=8080` として整数にするか、Rust 側の型を合わせる。独自の変換が必要なら、[カスタム型ガイド](./custom-types.ja.md)を参照。

### プロパティ名とフィールド名が違う

```kdl
service "api" image-name="myapp"
```

**原因**: KDL の `image-name` と Rust の `image_name` は、ハイフンとアンダースコアの違いによって一致しない。

**対処**: `#[kdl(property(rename = "image-name"))]` で対応を明示する。

## カスタム型

### `only traits defined in the current crate can be implemented...`

**原因**: 外部クレートの型に club-kdl のトレイトを直接実装しようとして、Rust の orphan rule に違反している。

**対処**: 自分のクレートで定義した newtype で包む。[カスタム型ガイド](./custom-types.ja.md)で具体例を説明している。

## デバッグのヒント

- **デシリアライズ結果が想定と違う**: まず `club_kdl::from_str` が返した `Err` を確認する。`Error::InContext` に、エラーに関係するノードやフィールドの情報が入る。
- **シリアライズ結果が想定と違う**: `to_string_pretty` の出力を表示し、KDL を直接確認する。
- **derive が生成したコードを見たい**: `cargo expand` でマクロの展開結果を確認する。

## 関連項目

- [カスタム型ガイド](./custom-types.ja.md)
- [KDL 設計のベストプラクティス](./best-practices.ja.md)
