# Changelog

このプロジェクトの主要な変更を記録します。

フォーマットは [Keep a Changelog](https://keepachangelog.com/ja/1.0.0/) に基づいており、
このプロジェクトは [セマンティックバージョニング](https://semver.org/lang/ja/) に準拠しています。

## [Unreleased]

## [0.14.0] - 2026-10-14

### 変更 (破壊的)

- **`club-kdl-compose` crate を `club-kdl` の `compose` feature に統合した**。
  `kdl_compose::compose` / `from_path` / `ComposeError` は
  `club_kdl::compose::{compose, from_path, ComposeError}` になった。 compose を
  使うのは codegen だけで、 crate を分けておく理由が薄かったため。 crate が
  4 → 3 になり、 リリース時に公開する crate も 1 つ減る。 `club-kdl-compose` は
  0.13.0 で公開を終える。
- **kdl-rs との相互運用を `kdl-rs` feature の裏に移した**。 対象は
  `from_kdl_document` / `from_kdl_node` / `to_kdl_document`、 `Document` / `Node` /
  `Value` と kdl-rs の型の間の `From` 変換、 `KdlNodeExt`、 `club_kdl::kdl` の
  再エクスポート。 エンジン本体は kdl-rs を使っていないので、 既定のビルドから
  kdl-rs (と miette / winnow などの依存) が消える。 既定のビルドの依存は
  `club-kdl-derive` と `thiserror` だけになる。
- どちらの feature も既定は off。

### 追加

- 読み込み側でも `u8` / `i8` / `i16` / `isize` / `f32` に対応した
  (`FromKdlValue`)。 0.13.0 では書き出しだけ対応していたため、 これらの型の
  フィールドを持つ struct は書けても読み戻せなかった。 範囲外の値はエラーにする
  (`f32` は、 有限の値が `f32` の範囲を超える場合にエラー。 `#inf` / `#nan` は通す)。

### ドキュメント

- README (英・日) に「How it works / 仕組み」節を追加した。 自前の KDL v2 エンジンの
  読み書きの経路、 速い理由、 手放したもの (コメントとレイアウトを保持しない)、
  正しさの担保を説明している。 あわせて、 ツリーの直接操作と kdl-rs との相互運用の例、
  読み込み / 書き出し別の対応型の表を加えた。
- crate のドキュメント (docs.rs) の機能説明を、 自前エンジンの実態に合わせた。

### 移行

- `club-kdl-compose` を使っていた場合: 依存を
  `club-kdl = { version = "0.14", features = ["compose"] }` に替え、
  `kdl_compose::` を `club_kdl::compose::` に置き換える。
- `from_kdl_document` / `from_kdl_node` / `to_kdl_document`、 `club_kdl::kdl`、
  kdl-rs の型との `From` 変換を使っていた場合: `features = ["kdl-rs"]` を足す。

## [0.13.0] - 2026-10-14

### 変更 (破壊的)

- **読み込みを独自の KDL v2 パーサーに置き換えた**。 従来は kdl-rs (`kdl` 6) で
  書式保持の AST を組み立ててから struct に写していたため、 読み込み時間の 97% が
  kdl-rs のパースに消えていた。 新パーサーは書式を捨てた軽量ツリー
  (`club_kdl::ast::{Document, Node, Entry, Value}`) を作り、 エスケープ不要な文字列は
  ソースから借用する。 公式 KDL テストスイート (320 件) 全件で準拠を検証
  (`tests/spec_compliance.rs`)。 壊した入力でも panic しないこと、 任意の文字列が
  書き出し → 読み戻しで変わらないことも常設テストで確かめる (`tests/robustness.rs`、
  `CLUB_KDL_FUZZ_SCALE` で反復回数を増やせる)。
  - read (1.7 KB → struct): 1288 µs → 20.0 µs (同一計測内の比較、 約 64 倍)
  - parse スループット: 約 1.3 MiB/s → 約 110 MiB/s (85 KB 文書、 同一計測)
  - 引用符付き文字列と行コメントは、 8 バイト単位の SWAR 判定でまとめて読み飛ばす
  - パースエラーに行・列が付く (`line 2, column 3: ...`)
- `KdlDeserialize::from_kdl_node` / `from_kdl_doc` の引数を
  `&KdlNode` / `&KdlDocument` から `&Node<'de>` / `&Document<'de>` に変更。
  `FromKdlValue::from_kdl_value` も `&KdlValue` から `&Value<'de>` に変更。
- `from_doc` / `from_node` は新しい `Document` / `Node` を受け取る。
- `doc_to_wrapper_node` を削除 (document 構造体は `Document` をコピーせずに読む)。
- **書き出しも独自の writer に置き換えた**。 `KdlSerialize::to_kdl_node` は
  `Result<Node<'_>>` を返し、 文字列は struct から借用する (コピーしない)。
  `ToKdlValue::to_kdl_value` は `Value<'_>` を返す。 `NodeBuilder<'a>` は値を
  参照で受け取る (`arg(&value)`)。
  - derive は中間ツリーを作らずに直接テキストへ書くコードも生成する
    (`to_string` / `to_string_into` / `append_node` が使う)。 ツリー経由と
    バイト単位で同じ出力になることをテストで固定 (`tests/write_equivalence.rs`)
  - write (struct → テキスト): 146 µs → 4.4 µs (同一計測内の比較、 約 33 倍。 JSON 4.3 µs と同等)
- **`to_string` / `to_string_pretty` の出力書式が変わる**。 従来は kdl-rs の
  未整形出力そのままで、 `{` が名前に密着し、 子ノードのインデントも崩れていた
  (`port host=80{` / 閉じ括弧だけ字下げ)。 これを 4 スペースのインデントで
  `name args {` の形に整えた。 値の書き方 (裸の識別子にできる文字列は引用符なし、
  `1.0`、 `#inf` など) は従来どおり。 制御文字は `\u{7}` のようにエスケープする
  (従来は不正な KDL を出していた)。 2 つの関数は同じ出力になる。
- **compose (`club-kdl-compose`) も独自パーサーで読む**。 `compose()` は
  `club_kdl::Document<'static>` を返し、 `ComposeError::Parse` の `source` は
  `club_kdl::Error` になった。 ノードの複製をやめて所有権を移すようにした。
  - codegen の `parse_path` (vantage-point の vp-daemon.kdl、 15 KB):
    10.5 ms → 0.31 ms (約 34 倍)
- codegen の `parse_doc` は `&club_kdl::Document` を受け取る。 compose と codegen は
  `kdl` crate に依存しなくなった。
- `club_kdl::{KdlDocument, KdlIdentifier, KdlNode, KdlValue}` の再エクスポートを
  やめ、 kdl-rs crate そのものを `club_kdl::kdl` として再エクスポートする。

### 修正

- `Option` の argument / property / `child, unwrap_arg` が `#null` を読めなかった
  (v0.12 以前から)。 `None` の argument は位置を保つため `#null` と書き出すので、
  書き出した結果を読み戻せなかった。 `#null` は `None` として読む。

### 追加

- `from_kdl_document` / `from_kdl_node`: kdl-rs の `KdlDocument` / `KdlNode`
  から読む互換 API (文字列はコピーせず借用)。
- `Document::into_nodes`、 `Document::parse`、 `Node` の accessor (`arg` / `args` / `prop` / `child` /
  `children_named` / `children`)、 kdl-rs との相互変換 (`From`)。
- `to_string_into`: 既存の `String` に追記する (多数の値を書くときにバッファを使い回せる)。
- `to_kdl_document`: 書き出し結果を kdl-rs の `KdlDocument` で受け取る
  (`autoformat` やコメント挿入など、 書式を保った編集をしたいとき)。
- `Document` / `Node` の `Display` と `write_to(&mut String)`。
- `NodeBuilder::flatten`、 `Node::set_name` / `into_children` / `into_parts`。
- `ToKdlValue` を `i8` / `i16` / `isize` / `u8` / `f32` / `Cow<str>` / `Value`
  にも実装し、 `&T` は blanket impl にした。

### 移行

- kdl-rs でパースしてから `T::from_kdl_node(&kdl_node)` を呼んでいた箇所は、
  テキストからなら `club_kdl::from_str` (最速)、 kdl-rs のツリーを持っているなら
  `club_kdl::from_kdl_node(&kdl_node)` / `club_kdl::from_kdl_document(&doc)` に置き換える。
- 手書きの `FromKdlValue` impl はシグネチャを `fn from_kdl_value(value: &'de Value<'de>)`
  に変える (`as_string` / `as_integer` などのメソッド名は同じ)。
- 手書きの `ToKdlValue` impl は `fn to_kdl_value(&self) -> Value<'_>` に変える。
  文字列は `Value::String(Cow::Borrowed(&self.0))`、 その場で組み立てる文字列は
  `Value::String(s.into())`。
- `to_kdl_node()` の結果を kdl-rs で整形していた箇所 (sizma など) は
  `club_kdl::to_string(&value)` で済む。 kdl-rs のツリーが必要なら
  `club_kdl::to_kdl_document(&value)`。
- `club_kdl::KdlDocument` などは `club_kdl::kdl::KdlDocument` に置き換える。
- `kdl_compose::compose` の結果を kdl-rs として扱っていた箇所は
  `club_kdl::kdl::KdlDocument::from(&doc)` で変換する。

## [0.12.0] - 2026-07-26

### 追加

- **codegen**: channel の **event** に対しても discriminated-union envelope
  (`{Channel}EventEnvelope`) を生成するようにした。従来 `envelope="<tag>"` は
  **request にしか** union を出さず、event は payload 型だけが生成されて
  受け手が判別に使える型が無かった（受信側が手書きで match するしかなく、
  envelope を置く意味が失われていた）。rust / typescript / zod の 3 emitter が対象
  （surrealql には envelope の概念が無い）。
  request 側の `{Channel}Envelope` は名前も内容も不変で、envelope 生成は
  **opt-in のまま**（tag 未宣言の channel は event があっても何も増えない）。PR #33

### 変更

- **MSRV: 1.94 → 1.95**。依存の `kdl` が 6.6.0 で MSRV を 1.82 → 1.95 に
  引き上げたため、宣言 MSRV では **そもそもビルドできない**状態になっていた
  （`Cargo.lock` は untracked なので CI は毎回解決し直し、6.6+ を引く）。
  1.94 を維持するには `kdl` を 6.5 に固定して upstream の修正を止める必要があり、
  実態に合わせて宣言側を上げる判断。PR #33


## [0.11.1] - 2026-05-25

### 修正

- **CI/release**: `release.yml` の crates.io publish 順を依存グラフ topological
  に修正 — `derive → main → compose → codegen` の順に並べ替え。PR #26
  で codegen が compose に依存するようになったが release.yml の順は古い
  まま放置していたため、v0.11.0 release で codegen publish 時点で
  `club-kdl-compose = "^0.11.0"` を crates.io 上で解決できず**部分公開**
  状態（derive と main のみ 0.11.0 published、codegen と compose は
  0.10.0 のまま）になっていた。本リリースで 4 crate 全部を 0.11.1 で
  揃え直す。

### 含まれる機能

- v0.11.0 の機能内容（codegen が compose 経由で multi-file schema 対応 / 新公開
  API `parse_doc` / `parse_path` / `ParseError` の `#[non_exhaustive]` 化）
  は変わらず。本リリースは workflow ordering bug の fix のみ。

## [0.11.0] - 2026-05-25

### 追加

- **codegen: `(<)` directive を透過的に解決** — codegen の CLI と library
  公開 API が `club-kdl-compose` 経由で動作するようになり、`(<)file` /
  `(<)glob` directive を含む multi-file schema を「読んで合成してから
  lower」する流れが標準に。`(<)` を含まない schema は compose を no-op で
  通過し、出力は v0.10.0 と byte 一致（後方互換）。
- **codegen: 新しい公開 API 2 つ**:
  - `parser::parse_doc(&KdlDocument) -> Result<Schema, ParseError>` —
    既に手元に `KdlDocument` がある consumer 向けの IO 無し低レベル API。
  - `parser::parse_path(impl AsRef<Path>) -> Result<Schema, ParseError>` —
    ファイルパスから compose 経由でロード・解決する高レベル API。CLI も
    このパスを通る。
- 既存 `parser::parse(&str)` は不変、引き続き pure-text 入力 path として
  維持される。

### 変更 (BREAKING)

- **codegen: `ParseError` に `#[non_exhaustive]` 属性 + `Compose(ComposeError)`
  variant を追加** — 将来の variant 追加 (Io / Network / 新 dialect の
  validation 等) を non-breaking で行えるようにするための一度きりの調整。
  下流 consumer が `match err { Kdl(_) | Validation(_) => ... }` で網羅
  していた場合は `_ => ...` arm を足す必要がある。

## [0.10.0] - 2026-05-25

### 追加

- **`club-kdl-compose` crate を新設** — KDL ドキュメントの複数ファイル合成を
  提供する独立 crate（bare lib 名 `kdl_compose`）。`(<)file` / `(<)glob`
  directive を解決して指定ファイルの top-level node を取り込み位置に
  splice する。
  - **directive 構文** — タグ `(<)` + variant (`file` / `glob`) の二段構造。
    KDL の `()` タグ慣用「型表明」と区別するため、単語ではなく**記号タグ**を
    採用（`<` の mnemonic は「他ファイルからこの位置に内容が流入する」）。
  - **公開 API**: `compose(path) -> KdlDocument` で解決済みドキュメントを返し、
    `from_path<T>(path) -> T` で `club-kdl` 連携の typed deserialize まで一気通貫。
  - **位置自由** — directive は top-level だけでなく任意の `protocol` / `channel`
    等の block 内にも書ける（block の children として展開）。
  - **取り込み元相対パス** のみ、CWD / search path / 環境変数解決はしない。
  - **cycle 検出** — A→B→A も A→A 自己 include も明示エラーで止める。
  - **glob 0 件マッチ** はエラーではなく空（仕様）。
  - **Phase 2 オプション**（children block 形）: `only "A" "B"` / `except "X"` /
    `rename "Old" "New"` + scalar property `as="ns"` で namespace prefix。
    apply 順は **filter → rename → as= prefix**。`as=` は top-level node の
    first string arg のみ rewrite し、内部参照は author の責務とする
    （compose は schema semantics 非依存）。
  - core `club-kdl` は **pure parser のまま** — `from_str` は IO ゼロを維持し、
    composer は opt-in（`tokio` / `tokio-util` パターン）。
  - **64 テスト**: unit 38 + integration 25 + runnable doc-test 1。
    `transform_node` / `parse_directive` / `parse_options_block` を unit で
    直接叩く設計で、ピラミッドを Small 59 / Medium 39 / Large 2 % に整えた。

## [0.9.1] - 2026-05-20

### 修正

- **codegen: TypeScript の channel type map を `interface` → `type` に** —
  `<Channel>ChannelEventTypes` / `<Channel>ChannelRequestTypes` を `interface`
  で出力していたため、SDK の `ChannelMeta.__types`（`Record<string, unknown>`）
  に代入できなかった（`interface` は暗黙の index signature を持たない）。
  `type` 別名で出力し、生成 `ChannelMeta` const が `ChannelMeta` を満たすようにする。

## [0.9.0] - 2026-05-19

### 追加

- **codegen: protocol 方言に channel envelope enum** — `channel` に
  `envelope="<tag>"` を付けると、その channel の全 `request` を束ねる
  discriminated union を生成する。Rust は `#[serde(tag="…")]` の内部タグ
  enum（field 無し request は unit variant）、TypeScript は
  `({ t: "…" } & Payload)` の判別 union、Zod は `z.discriminatedUnion`。
  `envelope` 無しの channel は従来どおり per-request の型のみで、出力は
  v0.8.0 と byte 一致（後方互換）。

### 変更

- **codegen: identifier sanitize** — `request` / `event` 名に含まれる
  `:` `/` `.` を識別子の語区切りとして扱い、wire 名（`lane:delete`）を
  正当な型名（`LaneDelete`）へ変換する。元の wire 名は `#[serde(rename)]`
  で保持。これまで `:` 入りの名前は生成コードをコンパイル不能にしていた。
  これに伴い protocol 方言の request / event 由来の型名は PascalCase に
  揃う。

### 内部

- 生成出力を実コンパイルする integration test を追加（Rust は使い捨て
  crate で `cargo build`、TypeScript は `bunx tsc --noEmit`）。文字列
  アサーションでは取りこぼす serde 属性の構文ミス等を検出する。CI の
  `test` job に Bun セットアップを追加。

## [0.8.0] - 2026-05-17

### 追加

- **codegen: KDL spec v2 Tier 1** — `record`（実体テーブル）/ `relation`
  （グラフエッジ）/ `link<T>`（レコード参照）/ union 型 / literal 型を追加。
  「構造 + 繋がり」を持つデータモデルを KDL で第一級表現できる。
- **codegen: KDL spec v2 Tier 2** — `field` および型定義の `description`
  （→ doc コメント / `COMMENT` / `.describe()`）、`constraints`
  （min / max / min_length / max_length / pattern → Zod `.min()` 等・
  SurrealQL `ASSERT`）。

### 変更

- **codegen (BREAKING): `field` のデフォルトを required に反転** — 無印の
  `field` は **required**。optional は `optional=#true` で opt-in
  （従来は optional がデフォルト）。
- **codegen: SurrealQL** — `ASSERT … IN` を `INSIDE` に統一。optional
  field の `ASSERT` は `$value = NONE OR …` でガード。

## [0.7.0] - 2026-05-17

### 追加

- **codegen: Zod emitter** — KDL schema から Zod schema (TypeScript の
  ランタイム validator) を生成。 enum は `z.enum`、 struct は `z.object`。
  Zod schema は値で前方参照できないため enum を struct より先に出力する。
  unison のブラウザクライアントが KDL schema 由来の型 + 検証を持つための
  基盤。
- **codegen: SurrealQL emitter** — KDL schema から SurrealDB の schema DDL
  (`DEFINE TABLE` / `DEFINE FIELD`) を生成。 enum 参照は `ASSERT $value IN
  [...]`、 struct 参照は `record<table>` link、 optional は `option<T>`。
  protocol 方言は DB 表現を持たないため data 方言のみが対象。
- **codegen: CLI に `zod` / `surrealql` ターゲット追加** — これで 4 ターゲット
  (rust / typescript / zod / surrealql) が出揃った。
- **codegen**: CLI 統合テストと parse→emit の end-to-end テストを追加
  (`codegen/tests/integration.rs`)。

### 修正

- **codegen**: parser が未定義の型参照 (`field type="..."` が `struct` /
  `enum` として未定義の名前を指す) を検証するように。 これまでは未定義型が
  emitter まで素通りし、 生成コードのコンパイルエラーとして遠くで顕在化
  していた。
- **codegen**: Rust 予約語 (`type` など) の field 名を raw identifier
  (`r#type`) でエスケープ。 生成 Rust コードのコンパイル不能を防ぐ。

### ドキュメント

- **codegen**: `lib.rs` の対応ターゲット記述を実態に同期 (4 ターゲット)。

## [0.6.0] - 2026-05-17

### 追加

- **`club-kdl-codegen` crate を新設**: KDL schema ファイルから Rust /
  TypeScript のコードを生成する crate。 IR (data / protocol の 2 方言) +
  parser + emitter + CLI で構成。 workspace の 3 つ目の member。 (PR #12)
- **NDKDL — `append_node`**: 値を 1 つの KDL ノードとしてファイル末尾に追記する
  ヘルパー (`club_kdl::append_node`)。 KDL を 1 ノード = 1 レコードのストリーム
  (ログ・メトリクス・イベント) として扱うための入口。 `to_string_pretty` が
  ドキュメント全体の round-trip 用なのに対し、 ファイル全体を読まずに追記する。
  追記後のファイルは `#[kdl(document)]` 構造体で読み戻せる。 (Refs #4)
- **guide ドキュメント** (`docs/guide/`): カスタム型ガイド / KDL 設計ベスト
  プラクティス / トラブルシュート の 3 本を新設。
- **README en/ja 構成**: `README.md` (日本語) + `README.en.md` (英語) の
  二言語構成に。

### Note

`append_node` は public API への additive な追加です (semver minor)。

## [0.5.1] - 2026-05-16

### 追加

- **dual license**: `MIT OR Apache-2.0` (Rust ecosystem 標準)
- **MSRV** を明示: `rust-version = "1.94"` (workspace)
- **`[package.metadata.docs.rs]`**: docs.rs での all-features build
- **doctest 実行可能化**: `lib.rs` / `de.rs` / `ser.rs` の example を `ignore` から実行可能 doctest へ
- **CI 強化** (`.github/workflows/ci.yml`):
  - `fmt --check` / `clippy -D warnings` / `doc -D warnings`
  - multi-OS (ubuntu / macOS / windows) × multi-toolchain (stable / beta)
  - MSRV テスト (`rust-version` 自動読み取り)
  - `cargo-deny` (license / advisories / bans)
  - `cargo-semver-checks` (PR 時のみ)
- **Release workflow** (`.github/workflows/release.yml`):
  - tag push (`v*.*.*`) で derive → main の順に自動 publish
  - `workflow_dispatch` で dry-run サポート
  - 自動 GitHub Release 生成
- **OSS hygiene**:
  - `CONTRIBUTING.md` / `SECURITY.md` / `CODE_OF_CONDUCT.md`
  - Issue templates (bug / feature)、 PR template
  - `dependabot.yml` (cargo + github-actions weekly)
  - `deny.toml`
- **derive crate metadata** 補完: `keywords` / `categories` / `readme` / `homepage`

### 変更

- LICENSE 年度更新: `2025` → `2025-2026`
- `Cargo.toml` の derive 依存を `=0.5.1` で pin

### 修正

- `cargo fmt --check` で検出された tests/exhaustive_mapping.rs の use 順序
- `cargo clippy` で検出された警告 9 件:
  - `collapsible_if` (derive/src/lib.rs, 4 件)
  - `bool_assert_comparison` (tests/exhaustive_mapping.rs, 3 件)
  - `approx_constant` (tests/exhaustive_mapping.rs, 2 件)
- benches/kdl_vs_json.rs の unused import / dead code

### Note

これは **品質整備リリース** で、 public API への変更はありません (semver patch)。

## [0.5.0] - 2026-05-15

### 変更 (Breaking)

- lib name を package name と統一: `unison_kdl` → **`club_kdl`** / `unison_kdl_derive` → **`club_kdl_derive`**
- v0.4.0 の rename trick (`[lib].name` 据置 + 内部 dep alias) を撤廃、 命名を一貫させた
- 下流の `use unison_kdl::...` は **`use club_kdl::...`** に書き換えが必要

```toml
# Cargo.toml
club-kdl = "0.5"
```

```rust
use club_kdl::{KdlDeserialize, KdlSerialize};
```

## [0.4.0] - 2026-05-15

### 変更 (Breaking — Cargo.toml level only)

- **crate を `unison-kdl` から `club-kdl` に rename** (chronista-club 命名規則に統一)
  - crates.io 上の名前: `unison-kdl` → **`club-kdl`**
  - derive crate も同様: `unison-kdl-derive` → **`club-kdl-derive`**
  - lib name は `unison_kdl` / `unison_kdl_derive` で据置 — **ソースコードの `use unison_kdl::...` は変更不要**
  - 下流 consumer は Cargo.toml の dep 行のみ更新:

    ```toml
    # 旧
    unison-kdl = "0.3"

    # 新
    club-kdl = "0.4"

    # または alias で旧来の import 感覚を維持
    unison_kdl = { package = "club-kdl", version = "0.4" }
    ```

### 内部

- ディレクトリ構造は据置 (`derive/` 等)。 package name のみ rename。
- `[lib].name` を明示的に指定 (`unison_kdl` / `unison_kdl_derive`) して import path を保護。
- 親 crate の `pub use unison_kdl_derive::...` は alias 付き dep 経由で維持
  (`unison_kdl_derive = { package = "club-kdl-derive", ... }`)。

### 命名規則の根拠

chronista-club ecosystem の crates.io 公開 crate は **`club-` prefix** で統一する。

| Layer | Prefix | 例 |
|-------|--------|----|
| 内部ツール / plugin | `cc-` | `ccwire`, `ccws` |
| 公開 crate (library) | `club-` | `club-unison`, `club-kdl` |

- 関連 PR: chronista-club/unison **PR #31** ([`club-unison` への rename](https://github.com/chronista-club/unison/pull/31))
- 命名規則 memory: creo-memories `mem_1Cb2haX6ZicuCweEpxAvj4`

## [0.3.0] - 2026-03-11

### 追加

- enum data variants 対応 (struct / newtype / unit バリアントの KDL シリアライズ・デシリアライズ)

## [0.2.0] - 2026-03-11

### 追加

- `kdl_node_name()` 自動解決
- `#[kdl(alias = "...")]` 属性
- `usize` 型対応
- 網羅テスト整備

[Unreleased]: https://github.com/chronista-club/club-kdl/compare/v0.14.0...HEAD
[0.14.0]: https://github.com/chronista-club/club-kdl/compare/v0.13.0...v0.14.0
[0.13.0]: https://github.com/chronista-club/club-kdl/compare/v0.12.0...v0.13.0
[0.12.0]: https://github.com/chronista-club/club-kdl/compare/v0.11.1...v0.12.0
[0.11.1]: https://github.com/chronista-club/club-kdl/compare/v0.11.0...v0.11.1
[0.11.0]: https://github.com/chronista-club/club-kdl/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/chronista-club/club-kdl/compare/v0.9.0...v0.10.0
[0.9.1]: https://github.com/chronista-club/club-kdl/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/chronista-club/club-kdl/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/chronista-club/club-kdl/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/chronista-club/club-kdl/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/chronista-club/club-kdl/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/chronista-club/club-kdl/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/chronista-club/club-kdl/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/chronista-club/club-kdl/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/chronista-club/club-kdl/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/chronista-club/club-kdl/releases/tag/v0.2.0
