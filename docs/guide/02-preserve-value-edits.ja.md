# 02. 原文を保持した値の編集

**[English](./02-preserve-value-edits.md)** | 日本語

`edit` feature は、コメント・空白・改行・他の値を書き換えずに、既存の値を更新する。
**未リリース**で、公開済みの 0.16.0 には含まれていない。このチェックアウトを隣接する
プロジェクトから使う場合は、次のように指定する。

```toml
club-kdl = { path = "../club-kdl", features = ["edit"] }
```

## 値を変更する

```rust
use club_kdl::edit::EditableDocument;

let mut doc = EditableDocument::parse(
    "// connection\nserver \"localhost\" port=8080 // development\n",
)?;
doc.set_property(&[0], "port", 9090)?;
assert_eq!(
    doc.as_str(),
    "// connection\nserver \"localhost\" port=9090 // development\n",
);
```

`parse` は原文を所有し、KDL v2 として検証する。`as_str()` は現在のテキストを借用し、
`to_string()` はコピーを返す。`into_string()` は編集用の文書を消費し、コピーせずに
テキストを返す。未編集なら、BOM や CRLF を含めて入力とバイト単位で一致する。

`set_argument(&[0], 0, "new host")` は、最初の位置引数を変更する。値は `ToKdlValue`
を実装した型を受け付け、文字列・数値・真偽値・`Value`・`Option<T>` を使える。引用符と
エスケープは自動で付く。指定した値の表記は、意味が同じ値を設定した場合でも正規化
されることがある。他の値の表記は変えない。

## 更新対象を指定する

| 指定 | 意味 |
|---|---|
| `&[0]` | 最初のトップレベルノード |
| `&[1, 0]` | 2 番目のトップレベルノードの、最初の子ノード |
| 引数 index `1` | 2 番目の位置引数。プロパティは数えない |
| プロパティ名 `"port"` | 対象ノードの、有効な `port` プロパティのうち右端のもの |

index は有効なノード・引数だけを数える。`/-` で無効化されたノード・entry・子ブロック
は原文として保持するが、編集対象にはならない。空のパスや存在しない対象を指定すると
`EditError` を返し、文書を変更しない。現在の値は、通常どおり
`Document::parse(doc.as_str())` や `club_kdl::from_str` で読める。

更新のたびに現在のテキストを解析するため、連続した編集で値の長さが変わっても対応できる。
ファイルの入出力は行わず、自動保存もしない。

## 対象範囲

最初の版では、ノード・entry の追加・削除・並べ替え、Rust 構造体から元文書への自動反映、
compose の参照先ファイルの更新は扱わない。既存の型注釈は原文のまま残し、スキーマの制約は
検証しない。通常の `Document` と struct のシリアライザーは、従来どおり整形した KDL を出力する。

[spec 03](../spec/03-preserve-value-edits.ja.md)・
[design 04](../design/04-preserve-value-edits.ja.md) を参照。
