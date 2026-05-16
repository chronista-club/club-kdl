# トラブルシュート

**[English](./troubleshooting.en.md)** | 日本語

club-kdl でよく遭遇するエラーと、 その原因・対処。

## ライフタイム

### `implicit elided lifetime not allowed here` (E0726)

借用フィールド (`&'a str`) を持つ struct を変数に束縛するとき:

```rust
#[derive(KdlDeserialize)]
#[kdl(name = "service")]
struct Service<'a> {
    #[kdl(argument)]
    name: &'a str,
}

let s: Service = club_kdl::from_str(r#"service "api""#).unwrap();
//     ^^^^^^^ error: ライフタイムパラメータが省略できない
```

**原因**: edition 2024 では型のライフタイムを省略すると E0726。

**対処**: `Service<'_>` と匿名ライフタイムを明示する。

```rust
let s: Service<'_> = club_kdl::from_str(r#"service "api""#).unwrap();
```

借用が不要なら、 フィールドを `String` にすればライフタイムごと消えます。

## 子ノードのマッピング

### `#[kdl(children)]` で子が収集されない

```rust
#[derive(KdlDeserialize)]
#[kdl(document)]
struct Config {
    #[kdl(children)]
    stages: Vec<Stage>,
}
```

**原因**: `#[kdl(children)]` は子型の `#[kdl(name = "...")]` でノード名を解決します。
子型に `#[kdl(name)]` が無いとフィールド名 (`stages`) にフォールバックし、
実際の KDL ノード名 (`stage`) と一致しません。

**対処**: 子型に `#[kdl(name)]` を付ける ―― または収集側で明示する:

```rust
#[kdl(children(name = "stage"))]
stages: Vec<Stage>,
```

### `#[kdl(child)]` に `Vec<T>` を指定してしまう

**原因**: `child` は単一の子 (0..1) 用。 繰り返しには使えません。

**対処**: 繰り返しは `#[kdl(children)]` + `Vec<T>`、 任意単一は `#[kdl(child)]` + `Option<T>`。

## ドキュメント全体

### トップレベルに複数ノードがある KDL が読めない

```kdl
stage "build"
stage "deploy"
service "api"
```

**原因**: 既定では「単一ノード = 単一 struct」。 複数のトップレベルノードを
1 つの struct にまとめるには `#[kdl(document)]` が必要です。

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

## 値型

### `type mismatch: expected ...`

**原因**: KDL の値の種類 (string / integer / float / bool) と Rust のフィールド型が不一致。
例えば `port="8080"` (文字列) を `u16` で受けようとした。

**対処**: KDL 側を `port=8080` (整数) にする、 または Rust 側の型を合わせる。
カスタム変換が必要なら [カスタム型ガイド](./custom-types.md) を参照。

### property 名がフィールド名と違う

```kdl
service "api" image-name="myapp"
```

**原因**: KDL の property 名 (`image-name`) と Rust のフィールド名 (`image_name`) は
ハイフン/アンダースコアの違いで一致しません。

**対処**: `#[kdl(property(rename = "image-name"))]` で明示的に対応付ける。

## カスタム型

### `only traits defined in the current crate can be implemented...`

**原因**: 外部 crate の型に club-kdl の trait を直接実装した (orphan rule 違反)。

**対処**: newtype でラップする。 詳細は [カスタム型ガイド](./custom-types.md) を参照。

## 切り分けのヒント

- **デシリアライズ結果がおかしい**: まず `club_kdl::from_str` の戻り値の `Err` を確認。
  club-kdl のエラーは `Error::InContext` でどのノード/フィールドかの文脈を持つ
- **シリアライズ結果が期待と違う**: `to_string_pretty` の出力を直接 print して KDL を目視確認
- **derive の展開を見たい**: `cargo expand` で derive マクロの生成コードを確認できる

## 関連

- [カスタム型ガイド](./custom-types.md)
- [KDL 設計ベストプラクティス](./best-practices.md)
