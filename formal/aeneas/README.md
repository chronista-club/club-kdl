# 実 Rust からの抽出証明（Charon / Aeneas / Lean 4）

`src/ast.rs` の 8 メソッドを **実際の Cargo crate から抽出**し、期待仕様との一致を証明します。Rust を写した別 crate や手書きのメソッド代替実装は使いません。

## 対象

| Rust のメソッド | 期待仕様 | 証明する動作 |
| --- | --- | --- |
| `Value::as_integer` | `expectedAsInteger` | Integer のときだけ元の i128 を Some で返す |
| `Value::as_float` | `expectedAsFloat` | Float のときだけ元の 64 bit 値を Some で返す |
| `Value::as_bool` | `expectedAsBool` | Bool のときだけ元の bool を Some で返す |
| `Value::is_null` | `expectedIsNull` | Null のときだけ true |
| `Entry::arg` | `expectedEntryArg` | name / ty が None、value が入力値の entry を作る |
| `Entry::value` | `expectedEntryValue` | entry の値をそのまま返す |
| `Document::nodes` | `expectedDocumentNodes` | root.children の全要素を順序・重複を保って返す |
| `Document::as_node` | `expectedDocumentAsNode` | root ノードをそのまま返す |

各 `*_matches_spec` は任意の入力について `Result.ok (expected... input)` との一致を証明します。したがって抽出モデル上では、値の一致だけでなく正常終了も含みます。`ClubKdlProof.lean` の各定理直後に `#print axioms` を置いています。

## 固定ツール

- Lean **4.36.0**（`lean-toolchain`）
- Charon **0.1.322**, commit `9e193f0317dacc822cc864c1798b117c70dcf7df`
- Charon が使う Rust **nightly-2026-09-17**
- Aeneas **nightly-2026.11.21-2e3d448**, commit `2e3d4488280bb8a95411db81019a7854a784861f`
- Bun（ローカル検証環境: 1.4.5）

Aeneas の Lean ライブラリも同じ revision に固定し、Mathlib を含む推移依存は `lake-manifest.json` に固定しています。`check.ts` は Aeneas・Charon・Rust toolchain の版、実効 Lean 版、manifest と実際の Lean 依存 checkout の Aeneas revision を検査します。

実行ファイルの既定配置は `${XDG_DATA_HOME:-$HOME/.local/share}/nostos-formal/aeneas-2e3d448/{charon,aeneas}` です。別の場所なら、そのディレクトリを `AENEAS_HOME` に指定してください。依存のインストールは検証スクリプトから行いません。Charon/Aeneas の構築は [Aeneas upstream](https://github.com/AeneasVerif/aeneas/tree/2e3d4488280bb8a95411db81019a7854a784861f) に従います。

## 初回準備と実行

elan / Lean / Lake、Bun、上記の Charon / Aeneas と Charon 用 Rust toolchain を用意します。Lake 設定と manifest はこのディレクトリに置いてあり、他 repo の設定を参照しません。

```sh
cd formal/aeneas
lake build ClubKdl
cd ../..
bun formal/aeneas/check.ts
bun formal/aeneas/check.ts --mutation-test
```

初回の Lake 依存取得・ビルドにはネットワークと時間が必要です。同じ lockfile / Lean 版で構築済みの `.lake/packages` がある場合は、依存パッケージのキャッシュだけを再利用できます。生成対象の `.lake/build` は共有しません。

`check.ts` は Bun で実行する TypeScript ですが、使う API は標準の `node:*` モジュールと `process` だけです。npm 依存は追加しません。

通常の検証は次の順に進みます。

1. ツール版を検査。
2. `charon cargo --preset=aeneas --sysroot default --start-from crate::ast::... -- ... --manifest-path Cargo.toml --lib` で、8 対象を実 crate から抽出。古い LLBC は事前に消し、新しい出力が無ければ失敗。
3. Aeneas の `-namespace ClubKdlGenerated -impl-namespace` で Lean を生成。生出力を `.lake/ClubKdl.raw.lean` に保存し、互換用 import のみを機械的に付加。
4. `lake build ClubKdl` で生成定義を検査。
5. `lake env lean -DwarningAsError=true ClubKdlProof.lean` で証明を毎回検査。
6. 8 件の公理出力が各 1 件あることを検査。許可する公理は `propext`, `Quot.sound` のみ。`sorryAx`、`Classical.choice`、独自の公理は拒否。

`ClubKdl.lean` は生成物としてコミットします。手で編集せず、変更は Rust または検証工程をレビューしてから再生成してください。

## Mutation test

`--mutation-test` は正常なソースを証明した後、一時ディレクトリに `Cargo.toml` / `Cargo.lock` / `src/` / `derive/` と workspace member の `codegen/`、manifest が明示する `benches/` をコピーします。元の workspace の manifest は変更しません。このライブラリ repo は `Cargo.lock` を追跡しないため、初回は Cargo が lockfile を作成します。既存 lockfile がある抽出とすべての mutation 試行は `--locked` で実行します。

8 試行それぞれで正しい元ソースに戻してから、一つだけ壊します。

| 対象 | 一時コピーへの変更 |
| --- | --- |
| `as_integer`, `as_float`, `as_bool` | 対応 variant の値を Some で返す代わりに None を返す |
| `is_null` | Null の代わりに Bool を true にする |
| `Entry::arg` | 入力値を捨てて Null を格納する |
| `Entry::value` | 元の値の代わりに Null を返す |
| `Document::nodes` | 全ノードの代わりに空の範囲 `[0..0]` を返す |
| `Document::as_node` | root の代わりに最初の子を返す（空なら panic） |

各試行では Rust を再抽出し、生成 Lean が正常時と異なることを確認してから再ビルド・再証明します。成功条件は **対象定理だけが `sorryAx` を伴う Lean の証明失敗となり、残り 7 定理の公理検査は通ること**です。Rust 抽出失敗、Lean 定義のビルド失敗、ツール欠落は mutation 成功として扱いません。

一時ディレクトリは `finally` で削除し、エラー終了時も元の `src/ast.rs` と `ClubKdl.lean` が開始時のバイト列と同じか照合します。証拠は `.lake/proof.log` と `.lake/mutation-<method>.log` に残ります。`.lake/` は git 管理外です。

## 保証の範囲と信頼境界

- 実 Rust の対象 8 メソッドと、それらが扱う AST 型を Charon → Aeneas → Lean で検査します。parser、`Node::arg` / `prop` の iterator 実装、derive の展開全体、serializer / writer 全体は抽出対象に含みません。それらの構造法則の手書きモデルは [../lean](../lean/README.md) を参照してください。
- 固定 Aeneas 2e3d448 は Rust `f64` を `F64` と出力しますが、対応する Lean 型をライブラリに定義していません。`ExtractionCompat.lean` で `F64 := BitVec 64` という **64 bit 値キャリアだけ**を補完します。IEEE 754 浮動小数点算術、丸め、比較、NaN の演算則は扱いません。variant 識別と値の受け渡しのみを証明します。NaN・無限大・負のゼロもビット列として保持します。
- `Value::Bool` コンストラクタと Lean の `Bool` 型も名前が衝突するため、`-impl-namespace` でメソッドを `.impl` に置き、互換ファイルで `abbrev ClubKdlGenerated.ast.Value.impl.Bool := _root_.Bool` と定義します。名前解決だけの型別名であり、新しい公理や振る舞いは追加しません。
- 上流の `Aeneas.Std.alloc.vec.Vec.deref` は格納 Slice を List 経由で再構築し、その長さの補題が `Classical.choice` に依存します。許可公理を広げず、互換ファイルに `ClubKdlGenerated.alloc.vec.Vec.deref v := v.slice` を定義して、抽出コードの標準ライブラリ呼び出しをこの構成的なモデルに名前解決します。格納済み Slice の全データと境界証明をそのまま返すモデルです。**この標準 primitive のモデル選択は信頼境界の一部**であり、Rust 標準ライブラリの `deref` 自体を抽出・証明したものではありません。対象の `Document::nodes` の Rust 本体と生成本体は無変更です。
- 生成された型・関数の本体は一切書き換えません。補助 import と生出力は区別でき、同じ入力からの再生成で差分が出ないことを検査できます。
- Lean カーネル、Charon/Aeneas の翻訳、Rust 標準ライブラリの Aeneas モデル、ツールチェーンと依存コードを信頼します。証明は実 CPU の実行・ABI・コンパイラの正しさを保証しません。参照を返す API は抽出モデルの値として扱い、実アドレスの同一性は主張しません。
- mutation test は検証の感度を確かめる 8 個の意図的な反例です。あらゆる Rust バグを検出する完全性や全 API の正しさを主張するものではありません。
