# 01. ルールベースの自動エイリアス

**[English](./01-compact-names.md)** | 日本語

`compact` feature は、機械が扱う保存データやメッセージ向けにノード名とプロパティ名を短縮し、インデントを省く。出力には通常の KDL v2 の文字列と構文を使う。人が読む設定ファイルには、従来どおり読みやすい名前を出力する `club_kdl::to_string/from_str` を使う。

v0.15.0 以降で利用できる。

```toml
[dependencies]
club-kdl = { version = "0.15", features = ["compact"] }
```

## 名前対応表を含むファイル

通常形式とコンパクト形式には、同じ derive を使える。

```rust
use club_kdl::{KdlDeserialize, KdlSerialize, compact};

#[derive(Debug, PartialEq, KdlSerialize, KdlDeserialize)]
#[kdl(name = "position_update")]
struct Update {
    #[kdl(property)]
    object_identifier: u64,
}

let update = Update { object_identifier: 42 };
let readable = club_kdl::to_string(&update)?;
let stored = compact::to_string(&update)?;
let restored: Update = compact::from_str(&stored)?;
assert_eq!(restored, update);
```

`stored` の先頭には `kdl-names 1 { ... }` という名前対応表があり、その後に短縮した文書が続く。対応表もファイルに含むため、後から読む側が過去のセッション情報を持つ必要はない。API はテキストを返すので、保存にはアプリケーション側のファイル I/O を使う。小さな文書では対応表の分だけサイズが増えることもある。汎用の圧縮方式とは用途が異なる。

任意の KDL 文書には `compact::encode_document(doc)` と `compact::decode_document(&text)` を使う。デコード結果は元の名前を持つ所有ツリーになる。`doc.to_string()` で読みやすい KDL に戻せるが、通常のツリー API と同様、元のコメントやレイアウトは保持しない。

## レイアウトと互換性

コンパクト出力は 1 行に 1 ノードを置き、行頭のインデントを省く。埋め込んだ名前対応表にも同じ規則を使う。親子関係は `{ }` で表し、**文字列値の中にある空白・タブ・改行**の意味は保つ。既存のリーダーは、同じ対応表 ID と形式バージョンで、インデントの有無にかかわらず読み込める。別の解凍処理は不要。

通常の `club_kdl::to_string`、`to_string_pretty`、`Document::to_string` は、従来どおり空白 4 文字でインデントする。コンパクトファイルを読みやすい KDL として確認するには、次のようにする。

```rust
let readable = compact::decode_document(&stored)?.to_string();
```

元の名前と通常の書式は復元されるが、入力時のコメントや空白配置まで再現するわけではない。インデントの省略は、深い入れ子を持つ文書ほどサイズ削減に効く。小さな文書や平坦な文書では効果が小さい。

## メッセージ間で名前対応表を再利用する

```rust
let sender = compact::NameTable::from_value(&update)?;
let table_text = sender.to_kdl(); // send/store once, outside the message stream
let receiver = compact::NameTable::from_kdl(&table_text)?;

let message = sender.encode(&update)?;
let restored: Update = receiver.decode(&message)?;
```

各メッセージは `kdl-message 1 <table-id>` で始まる。受信側は名前を展開する前にバージョンと対応表 ID を検証する。SHA-256 の ID は対応表全体を識別するためのもので、メッセージ認証やフィールドの型判定には使わない。通信方式の取り決め、メッセージ境界の扱い、対応表の有効期間はアプリケーション側で管理する。

対応表には、**使う可能性のある名前をすべて**含める。`from_value` が収集するのは、渡した値に実際に現れる名前だけである。その値に含まれない省略可能な子ノードや、将来追加するマップのキーは収集されない。必要なら、使用する名前の和集合を明示する。

```rust
let table = compact::NameTable::from_names([
    "position_update", "object_identifier", "optional_child", "dynamic_key",
]);
```

対応表は作成後に変更できない。名前が増減したら新しい表を作り、送受信側で交換する。表にない元の名前のエンコードや、未知のエイリアスのデコードはエラーになる。

対応表の `encode_document(doc)` / `decode_document(&message)` は、任意のツリーに対する同等の API である。共有表を使ったデコード結果は、対応表と入力から借用する場合がある。それらより長く文書を保持するなら `into_owned()` を呼ぶ。

## 割り当て規則とエラー

名前をソートして重複を除いた後、`a..z`、`A..Z`、`aa` などの ASCII 文字を順に割り当てる。元の名前や、引用符が必要な KDL キーワードは割り当てから除外する。候補になるのは、シリアライズ後の UTF-8 表現が 3 バイト以上の名前だけで、元より短くなる場合に限って置き換える。短い名前も、変換前後が同じ対応として表に残す。値と型注釈は変更しない。

エラーは `compact::CompactError` で返る。

- `UnknownName`: 名前が対応表にない。その名前を含む新しい表を作り、交換する。
- `UnknownAlias`: ペイロードに、対応表で定義されていない名前が含まれている。
- `TableMismatch`: 対応する表を選び直す。推測した表や別の表でデコードしない。
- `UnsupportedVersion`: リーダーが対応していないプロトコルバージョンである。
- `InvalidTable` / `InvalidEnvelope`: メタデータが不正なため、受け入れない。
- `Kdl`: KDL の構文解析、またはオブジェクトへの変換に失敗した。

表の検証では、重複した対応、余分な引数やプロパティ、型注釈、入れ子のエントリを厳格に拒否する。読み込む表では `"0"` のような引用文字列もエイリアスに使える。自動割り当てには、引用符の分だけサイズが増えるのを避けるため英字を使う。

プロトコルの詳細は [設計 01](../design/01-name-shortcuts.ja.md)、サイズと処理時間は [v0.15.0 の実装 API の測定結果](../benchmarks/indent-pack.ja.md) を参照。ペイロードが小さくなっても、実行時のツリー構築や名前検索によって、通常の直接書き込みより CPU コストが増える場合がある。
