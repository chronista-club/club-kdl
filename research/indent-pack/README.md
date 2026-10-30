# インデント省略の検証

対象は main `3f10721716c806948276c4029e331170478cef74`（PR #43）。
ユーザーの着想 `mem_1CgYEeWUHuLPHVaFWe6z71` を受けたローカル実験。
以下はspike時点の記録で、その時点ではライブラリ本体を変更していない。
本実装とAPI全体の測定は [production report](../../docs/benchmarks/indent-pack.md) を参照。

## 結果と判断

インデント省略は深い文書ほど容量を減らし、この測定では読み取りも改善した。
浅い文書の速度差は小さい。書き出しはバッファ再利用で結果が変わるため、
どの入力でも速くなるとは判断しない。

以下は名前短縮済みの共有メッセージ。時間は各入力1文書あたりの点推定（µs）。

| 入力 | pretty → no_indentのバイト数 | 削減率 | write_fresh | parse |
|---|---:|---:|---:|---:|
| 深さ0・100レコード | 6,471 → 6,471 | 0% | 12.25 → 12.17 | 27.47 → 28.33 |
| 深さ1・100レコード | 6,877 → 6,477 | 5.8% | 12.98 → 12.50 | 28.30 → 27.45 |
| 深さ8・100レコード | 9,943 → 6,519 | 34.4% | 13.48 → 12.39 | 32.65 → 28.10 |
| 深さ32・100レコード | 23,431 → 6,663 | 71.6% | 14.31 → 12.90 | 58.85 → 28.89 |
| 深さ128・1レコード（極端例） | 66,448 → 912 | 98.6% | 11.33 → 1.37 | 116.74 → 7.63 |

バッファ再利用時のwriteは、深さ8で12.28 → 12.94 µsと逆に遅くなり、
深さ32では13.42 → 12.32 µsと速くなった。容量が減れば同率で速くなるわけではない。
深さ0の対照でも数%の差が出ており、小さい速度差を一般化しない。

single_lineは、深さ8で6,503 B、深さ32で6,599 B。
no_indentからさらに減るのは各16 B・64 Bだけで、今回のread/writeも
no_indentを一貫して上回らなかった。改行1文字をセミコロン1文字へ変えても
その部分の容量は変わらず、追加削減は各children blockの周辺空白に限られる。

次の候補は **compactの出力時にインデントを省く選択肢**。
標準KDLを維持でき、既存decoderも変更せず利用できる。
導入前には型付きAPI全体（AST構築・名前置換を含む）と利用先の実データで
測定し、通常の人間向け出力とのAPI上の切り分けを決める。
独自の個数符号化、汎用圧縮、完全な元書式復元は今回実装していない。

## 比較する形式

- `pretty`: 現在のwriterと同じ、4スペース/段と改行。
- `no_indent`: インデントを出力せず、改行は維持する。
- `single_line`: インデントと開き波括弧の周囲の空白を出力せず、ノード終端をセミコロンにする。末尾にもセミコロンを残すため、最短表現の探索ではない。

`generate.py` が上記baseline commitのwriterと文字判定処理から実験用コピーを作る。
値の書き方・引用・escapeは共通で、構造上の空白出力だけを変更する。
全行のtrimや文字列置換でKDL本文を加工しない。

階層は波括弧で表せるので、インデントの個数を保存する独自形式は導入しない。
復元するのはデータの意味。元のコメント、タブ、スペース個数の完全復元は対象外。
複数行文字列の空白は値として維持し、出力時は通常のquoted stringとescapeに正規化する。

## 正しさ

最初は全モードで従来writerを呼び、インデント省略・単一行出力の2テストが
期待どおり失敗することを確認。その後に実験用writerを導入した。

- 6テスト成功。空文書、複数node、型注釈、引数、重複property、Unicode、特殊数値、文字列内の空白・改行・記号を確認。
- 公式spec corpusの有効文書233件を、全3形式でparse → write → parseしてcanonical出力が一致。
- 233件すべてについて、独立実装kdl-rsでもbaselineと各形式の意味が一致。
- 共有名前表のメッセージと名前表内包ファイルを、既存compact decoderで復元できる。
- 負の対照として、行頭を単純trimすると複数行文字列の値が変わることも確認。
- ベンチ90ケースのsmoke成功、clippy（warningsをerror扱い）成功。
- Rust 1.95で実験用crateのall-targets check成功。

## 測定条件

2026-10-30、Apple M5（10コア）、macOS 27.2、rustc 1.96.0。
Criterion 0.8.2、40 samples、warmup 200 ms、measurement 1秒、10,000 resamples。
release build、1スレッド、同一run内で比較。

深さ0/1/8/32に100レコード、深さ128に1レコードを配置する合成データを使う。
各レコードは6つのpropertyを持ち、文字列に空白・改行・タブ・波括弧・セミコロンを含む。
深さ128は極端な場合の確認であり、通常の利用例とは扱わない。
本名と自動aliasの2種類を測定し、alias側は共有表メッセージのheaderを含む。

- `write_fresh`: **事前構築したAST**から新しいStringへ書き出し、結果の破棄まで。
- `write_reuse`: 同じ容量のStringをclearして再利用。容量はprettyのサイズに統一。
- `parse`: **事前生成した文字列**を既存パーサーで読み、ASTの破棄まで。

名前表の構築・検索・名前の置換、型付きstructの変換は計測に含めない。
したがって、ここでの倍率をcompact API全体の倍率として扱わない。
ネットワーク、汎用圧縮との比較、総CPU時間、p99、RSSは未測定。
共有マシンで固定順序の1 runの結果で、実行順の無作為化や複数run比較はしていない。
小さな速度差は断定しない。

容量は `artifacts/sizes.csv`、時間と95%信頼区間は `artifacts/results.csv`。
検証・測定ログは `evidence/`。

## 再現

リポジトリのルートから実行する。

```sh
python3 research/indent-pack/generate.py
cargo fmt --manifest-path research/indent-pack/Cargo.toml
cargo test --manifest-path research/indent-pack/Cargo.toml --locked -- --nocapture
cargo bench --manifest-path research/indent-pack/Cargo.toml --bench layout --locked -- --test
cargo bench --manifest-path research/indent-pack/Cargo.toml --bench layout --locked -- --noplot
python3 research/indent-pack/summarize.py
```

生成したKDLサンプルは `artifacts/` に置く（git管理外）。
Cargo.lockは実験の依存バージョンを固定するため保存する。
元の状態を完全に再現する場合は `b5d1aef` のcheckoutで実行する。
生成元writerはbaseline commitに固定しているが、依存するclub-kdl本体はcheckoutに従う。
異なるcheckoutで再測定した場合は新しい結果として扱う。
