# Lean 検証記録 — 2026-11-25（feature 拡張）

Task: `mem_1ChPHc9zUwMyPFSSRfUSKc`。基点: `8e0915f`（PR #54）。以下はこの変更での実測です。将来のソース変更後の成功を保証しません。

## ローカル実測

| 検証 | 結果 |
| --- | --- |
| `cd formal/lean && lean -DwarningAsError=true KdlCore.lean` | exit 0。既存 24 件を含む 27 構造定理 + 15 補助定理 = 42 件すべて空公理 |
| `bun formal/aeneas/check.ts --mutation-test` | exit 0。`compact,edit,compose` を有効にした実 crate の 15 仕様一致証明に成功 |
| 抽出証明の公理 | 15 件すべて `propext`, `Quot.sound` のみ |
| Rust mutation | 15/15 拒否。各試行で対象の 1 定理だけが `sorryAx` を持ち、残り 14 定理は許可公理のみ |
| mutation の原本保護 | `src/ast.rs`, `src/edit.rs`, `src/ser.rs`, `ClubKdl.lean` は開始時とバイト一致 |
| 再生成 | 通常の `check.ts` の再実行も exit 0。生成本文は Aeneas 生出力に互換 import 1 行を付加したもの |
| workflow | `actionlint .github/workflows/ci.yml` exit 0 |
| 配布範囲 | `cargo package -p club-kdl --list --allow-dirty` exit 0、`formal/` 配下のファイルなし |
| 差分 | `git diff --check` exit 0。Rust 本体・依存・`formal/quint/`・Lake lock は無変更 |

mutation 対象は [README の全 15 メソッド](README.md#対象)。ローカル生ログは `formal/aeneas/.lake/core-proof.log`, `proof.log`, `check.log`, `regenerate.log`, `mutation-<method>.log`, `package-list.log`。`.lake/` は git 管理外です。

SHA-256:

```text
55fb29440ff0f236e23a80882b9d6af20319cccfc9f2db80ef60015bf174e3dd  src/ast.rs
d29376bafad7d4d8a873286758c0f0e4a80ccab2079889e549d9a2625f3cc1c5  src/edit.rs
0ba6cbdb6880035ad5315a6d8b2a9d817b0e71380e6f60fc6bc9701ecb8053b6  src/ser.rs
21a2bc387be86e9a8031999ccdebda4965f322575024e146eb2b7acdcb05919c  formal/aeneas/ClubKdl.lean
31f4b2380be6b04d03cfea846dc70221aa62be73cf2e413ee99f98659fa518d0  formal/lean/KdlCore.lean
```

## 段 B セルフレビュー

- edit の DFS entry 列と count / locate は独立した再帰定義。任意深さの子と先行兄弟を含む加算を証明し、目的の entry が存在するという前提を明示した。Rust の byte span 生成・`usize` overflow の証明とは区別した。
- compact は成功した任意の node / entry 列の往復を証明。辞書の全域性を仮定せず、値・型注釈・順序・children を保存する。位置引数の不変と shadowing の一般的な反例も別定理にした。
- compose は `arg(0).and_then(as_string)` をモデル化し、後続の文字列を誤って拾わない。setter は Rust と同様に置換 entry の型注釈を消す。名前なしの無変更と、元名でのフィルタ・rename→prefix の順序、および先に rename すると壊れる条件を証明した。
- compose の名前は構成的な `List Char`。Lean の最適化された `String.append` / `BEq String` に由来する公理依存を確認し、構造モデルで使う文字列表現を変更した。UTF-8 実装との同値性は主張していない。
- Rust 実抽出は指定された 15 メソッド。feature を有効化したことと、feature 内の全関数を証明したことを混同しない。生成本文と既存 `ExtractionCompat.lean` は手修正していない。
- mutation は 3 ファイルを毎回元に戻してから 1 メソッドだけ変更する。対象外の 14 定理の成功と、原本のバイト不変を検査する。コンパイル不能な mutation は成功と数えず、`NodeBuilder::build` の mutation をコンパイル可能な entries 消去に直して全試行を再実行した。
- TDD は証明と負例の失敗・成功で検証した。Rust プロダクションコードの追加・修正はない。生成物と CI / Cargo 設定は tdd スキルの省略対象であり、再生成・構文・配布一覧で検証した。
- CI は固定 release の SHA-256 を検査。構造証明の空公理検査、15 mutation、生成物・lock の差分ゼロを別 step にし、Quint は既存スクリプトを別ジョブから呼ぶ。
- 既存 `.mcp.json` の変更は保持し、作業コミットから除外する。`formal/quint/` と Rust source に変更なし。

GitHub Actions の結果は PR の checks を参照してください。ローカル実測と GitHub 実行は別の証拠です。main へのマージ・リリースは行いません。
