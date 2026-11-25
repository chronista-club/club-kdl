# Lean 検証記録 — 2026-11-25

対象ソースの基点: `144bee97a975fa4cad654cd12b2ac4c089518547`（club-kdl 0.17.0）。この記録はローカルで実行した検証の証拠であり、将来のソース変更後の成功を保証しません。

## 実測結果

| 検証 | 結果 |
| --- | --- |
| `cd formal/lean && lean -DwarningAsError=true KdlCore.lean` | exit 0。17 構造定理 + 7 補助定理 = 24 件すべて空公理 |
| `bun formal/aeneas/check.ts --mutation-test` | exit 0。実 crate の 8 仕様一致証明と公理検査に成功 |
| 抽出証明の公理 | 8 件すべて `propext`, `Quot.sound` のみ |
| Rust mutation | 8/8 拒否。各試行で対象の 1 定理だけが `sorryAx` を持ち、他の 7 定理は許可公理のみ |
| mutation の原本保護 | `src/ast.rs` と `ClubKdl.lean` は開始時とバイト一致 |
| 生成定義の無改変 | `ClubKdl.lean` から機械付加した import 1 行を除くと、Aeneas 生出力とバイト一致 |
| 再生成の再現性 | 通常の `check.ts` を再実行して exit 0。stage 済み `ClubKdl.lean` に対する `git diff --exit-code` も exit 0 |
| ソース範囲 | Rust 本体・derive・Cargo 設定の差分なし。変更は `formal/lean/` と `formal/aeneas/` 内のみ |

mutation 対象: `as_integer`, `as_float`, `as_bool`, `is_null`, `entry_arg`, `entry_value`, `document_nodes`, `document_as_node`。

ローカルの生ログは `formal/aeneas/.lake/core-proof.log`, `proof.log`, `check.log`, `mutation-<method>.log`。キャッシュとログは git 管理外です。上記コマンドで再検証できます（`core-proof.log` / `check.log` は手動リダイレクト、`proof.log` / mutation ログはスクリプトが生成）。

SHA-256:

```text
55fb29440ff0f236e23a80882b9d6af20319cccfc9f2db80ef60015bf174e3dd  src/ast.rs
75adc979aa346a3e40ae5ce3f4e18d251005111c4a109aa9f788a0e36710c9ba  formal/aeneas/ClubKdl.lean
29db1fc2a6147cbd18cfee113279658da2eac6c9c1b657b0822815d2a3eab1d0  formal/lean/KdlCore.lean
```

## 段 B セルフレビュー

- AST の property は実実装と同じ右端優先。任意位置への挿入を含み、位置引数と property の検索を混同していない。
- Option の往復は `Some null` を除く前提を明記。None の位置を保持する正しい経路と、省略で添字がずれる反例を別々に証明した。
- flatten は独立した builder / writer の再帰定義を比較している。保証は列の等価性であり、文字列エスケープや実 formatter 全体の検証とは区別した。
- newtype の payload は変換済みとして扱い、外側の variant 分岐と内側の名前検査の有無を検証した。
- 抽出対象は実 Cargo crate の 8 メソッド。生成本文は手修正せず、毎回抽出する。失敗した翻訳を古い LLBC で補わない。
- 固定 Aeneas の制約に対する互換層は lead 承認済み。`F64` のビットキャリア、`Bool` の型別名、`Vec.deref` の格納 Slice 射影という 3 点に限定し、[README の信頼境界](README.md#保証の範囲と信頼境界)に明記した。
- 公理検査は当初 `Document::nodes` の `Classical.choice` を実際に拒否した。許可集合は拡大していない。最終的な定理の全出力を監査した。
- mutation は対象ごとに正しいソースから開始し、抽出・生成定義のビルド・証明の失敗を区別する。一時コピーの workspace member / bench path も保持し、元 manifest を変更しない。
- `.lake/` は除外。既存の `.mcp.json` の変更は保持し、この作業のコミットに含めない。

GitHub の既存 Rust CI と、ここに記録したローカル形式検証は別の検証です。形式検証の CI 組み込みは今回の変更範囲に含みません。main へのマージ・リリースは行いません。
