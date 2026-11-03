# カスタム型ガイド

**[English](./custom-types.md)** | 日本語

club-kdl が標準では扱わない型を、KDL の値として読み書きする方法を説明する。外部クレートの型や、独自の newtype が対象となる。

## 概要

club-kdl が標準で扱う型は次のとおり。

- 整数（`i8` `i16` `i32` `i64` `i128` `isize` `u8` `u16` `u32` `u64` `usize`）、浮動小数点数（`f32` `f64`）、真偽値（`bool`）
- 文字列（`String` `&str`）、パス（`PathBuf`）
- コレクション（`Vec<T>` `HashMap<String, String>`）、`Option<T>`

`chrono::NaiveDate`、独自の `UserId(u64)`、外部クレートの列挙型などを `#[kdl(argument)]` / `#[kdl(property)]` のフィールドで使うには、その型に **`FromKdlValue` / `ToKdlValue` を実装する**。

## 前提

- `#[derive(KdlDeserialize, KdlSerialize)]` と `#[kdl(...)]` 属性の基本的な使い方を理解していること。
- 対象は、引数やプロパティに対応するスカラー値であること。子ノードの構造を持つ型は、通常の構造体と derive で表す。

## トレイト

```rust
use club_kdl::{FromKdlValue, ToKdlValue};

pub trait FromKdlValue<'de>: Sized {
    fn from_kdl_value(value: &'de Value<'de>) -> Result<Self>;
}

pub trait ToKdlValue {
    fn to_kdl_value(&self) -> Value<'_>;
}
```

- デシリアライズには `FromKdlValue`、シリアライズには `ToKdlValue` が必要。片方だけ実装してもよい。
- どちらも、club-kdl の軽量ツリーで値を表す `club_kdl::Value` を使う。`ToKdlValue` は `self` から借用した `Value`（`Value::String(Cow::Borrowed(..))`）を返せる。計算して作った文字列には、所有する文字列（`.to_string().into()`）を使う。
- `'de` は、KDL のソースからコピーせずに借用するためのライフタイム。値を所有する必要がある場合は `.to_owned()` を使う。

## 使用例

### 例 1: newtype で既存の型を包む

`UserId(u64)` のような newtype は、内側の型の実装に処理を委ねるだけでよい。

```rust
use club_kdl::{FromKdlValue, Result, ToKdlValue, Value};

struct UserId(u64);

impl<'de> FromKdlValue<'de> for UserId {
    fn from_kdl_value(value: &'de Value<'de>) -> Result<Self> {
        u64::from_kdl_value(value).map(UserId)
    }
}

impl ToKdlValue for UserId {
    fn to_kdl_value(&self) -> Value<'_> {
        self.0.to_kdl_value()
    }
}
```

### 例 2: `chrono::NaiveDate` を文字列として扱う

外部クレートの型である `chrono::NaiveDate` に、club-kdl のトレイトを直接実装することはできない。型とトレイトの両方が自分のクレート外にあるため、Rust の **orphan rule** に違反する。自分のクレートで定義した **newtype で包む**ことで対応できる。

```rust
use club_kdl::{Error, FromKdlValue, Result, ToKdlValue, Value};
use chrono::NaiveDate;

struct Date(NaiveDate);

impl<'de> FromKdlValue<'de> for Date {
    fn from_kdl_value(value: &'de Value<'de>) -> Result<Self> {
        // First extract as &str, then parse with chrono
        let s = <&str>::from_kdl_value(value)?;
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Date)
            .map_err(|e| Error::Custom(e.to_string()))
    }
}

impl ToKdlValue for Date {
    fn to_kdl_value(&self) -> Value<'_> {
        Value::String(self.0.format("%Y-%m-%d").to_string().into())
    }
}
```

この `Date` を `#[kdl(property)] created: Date` として使うと、次の KDL を読み書きできる。

```kdl
record created="2026-05-17"
```

### 例 3: 値を検証する

`from_kdl_value` は `Result` を返すため、変換時に値を検証できる。

```rust
struct Port(u16);

impl<'de> FromKdlValue<'de> for Port {
    fn from_kdl_value(value: &'de Value<'de>) -> Result<Self> {
        let n = u16::from_kdl_value(value)?;
        if n == 0 {
            return Err(Error::Custom("port must not be 0".into()));
        }
        Ok(Port(n))
    }
}
```

## トラブルシューティング

| 症状 | 原因 | 対処 |
|---|---|---|
| `only traits defined in the current crate can be implemented for types defined outside of the crate` | 外部の型に外部のトレイトを実装しようとして、orphan rule に違反した | newtype で包む（例 2） |
| `borrowed value does not live long enough` | `&'de str` を借用したまま変換しようとした | 所有する値が必要なら、`.to_owned()` で `String` にする |
| 型の不一致をエラーにしたい | KDL の値の種類が想定と異なる | `Error::type_mismatch("expected", value)` を返す |

## 関連項目

- README の「サポートされる型」
- スカラー列挙型の対応は derive（`#[kdl(rename = "...")]`）が生成するため、手動での実装は不要
