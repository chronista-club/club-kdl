# カスタム型ガイド

**[English](./custom-types.en.md)** | 日本語

club-kdl が標準で対応しない型 (外部 crate の型・独自 newtype など) を KDL 値にマッピングする方法。

## Overview

club-kdl は以下の型を標準でサポートします:

- 整数 (`i32` `i64` `i128` `u16` `u32` `u64` `usize`) / 浮動小数 (`f64`) / 真偽値 (`bool`)
- 文字列 (`String` `&str`) / パス (`PathBuf`)
- コレクション (`Vec<T>` `HashMap<String, String>`) / `Option<T>`

これ以外の型 ―― 例えば `chrono::NaiveDate`、 独自の `UserId(u64)`、 外部 crate の列挙型 ―― を
`#[kdl(argument)]` / `#[kdl(property)]` フィールドで使いたい場合、 その型に
**`FromKdlValue` / `ToKdlValue` trait を実装**します。

## Prerequisites

- club-kdl の基本 (`#[derive(KdlDeserialize, KdlSerialize)]` と `#[kdl(...)]` 属性) を理解していること
- 対象は argument / property にマッピングされる「スカラー値」 としての型。 子ノード構造を持つ型は通常の struct + derive で表現します

## The traits

```rust
use club_kdl::{FromKdlValue, ToKdlValue};

pub trait FromKdlValue<'de>: Sized {
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self>;
}

pub trait ToKdlValue {
    fn to_kdl_value(&self) -> KdlValue;
}
```

- デシリアライズには `FromKdlValue`、 シリアライズには `ToKdlValue` が必要。 片方だけの実装も可
- `'de` ライフタイムは KDL ソースからの借用 (zero-copy) を可能にする。 値を所有したい場合は `.to_owned()` する

## Usage

### 例 1: newtype ―― 既存型に薄いラッパ

`UserId(u64)` のような newtype は、 内部型の実装にデリゲートするだけです。

```rust
use club_kdl::{FromKdlValue, ToKdlValue, KdlValue, Result};

struct UserId(u64);

impl<'de> FromKdlValue<'de> for UserId {
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        u64::from_kdl_value(value).map(UserId)
    }
}

impl ToKdlValue for UserId {
    fn to_kdl_value(&self) -> KdlValue {
        self.0.to_kdl_value()
    }
}
```

### 例 2: 外部型 ―― `chrono::NaiveDate` を文字列にマッピング

外部 crate の型 (`chrono::NaiveDate`) に club-kdl の trait を直接実装することは
**orphan rule によりできません** (型も trait も自分の crate の外にあるため)。
**newtype でラップ**して回避します。

```rust
use club_kdl::{FromKdlValue, ToKdlValue, KdlValue, Error, Result};
use chrono::NaiveDate;

struct Date(NaiveDate);

impl<'de> FromKdlValue<'de> for Date {
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        // まず &str として取り出し、 chrono でパース
        let s = <&str>::from_kdl_value(value)?;
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Date)
            .map_err(|e| Error::Custom(e.to_string()))
    }
}

impl ToKdlValue for Date {
    fn to_kdl_value(&self) -> KdlValue {
        KdlValue::String(self.0.format("%Y-%m-%d").to_string())
    }
}
```

これで `#[kdl(property)] created: Date` のように使えます:

```kdl
record created="2026-05-17"
```

### 例 3: 値の検証を挟む

`from_kdl_value` は `Result` を返すので、 パースと同時にバリデーションできます。

```rust
struct Port(u16);

impl<'de> FromKdlValue<'de> for Port {
    fn from_kdl_value(value: &'de KdlValue) -> Result<Self> {
        let n = u16::from_kdl_value(value)?;
        if n == 0 {
            return Err(Error::Custom("port must not be 0".into()));
        }
        Ok(Port(n))
    }
}
```

## Troubleshooting

| 症状 | 原因 | 対処 |
|------|------|------|
| `only traits defined in the current crate can be implemented for types defined outside of the crate` | orphan rule ―― 外部型に外部 trait を実装しようとした | newtype でラップする (例 2) |
| `borrowed value does not live long enough` | `&'de str` を借用したまま型変換しようとした | 所有が必要なら `String` 経由で `.to_owned()` |
| 型不一致を返したい | KDL 値の種類が期待と違う | `Error::type_mismatch("expected", value)` を返す |

## 関連

- README の「サポートする型」 セクション
- enum のスカラーマッピングは derive が自動対応 (`#[kdl(rename = "...")]`) ―― 手書き実装は不要
