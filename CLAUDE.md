# club-kdl

KDL (KDL Document Language) の serialization / deserialization ライブラリ。
derive macro (`#[derive(KdlDeserialize, KdlSerialize)]`) で KDL ↔ Rust struct を往復する。

- crate: `club-kdl` (lib) / `club-kdl-derive` (proc-macro)
- repo: https://github.com/chronista-club/club-kdl
- ライセンス: MIT OR Apache-2.0 / MSRV: Rust 1.95 / edition 2024

詳細な行動指針はグローバル `~/.claude/CLAUDE.md` (Chronista Style) を参照。
このファイルは club-kdl 固有の事項のみ記載する。

## creo-memories Atlas

このプロジェクトのデフォルト Atlas は **`Kdl Club`** (`atl_1Cb5n12s7T1RUEB1XEfELn`)。

- club-kdl 関連の `search` / `remember` / `create_todo` は **必ず `atlasId: atl_1Cb5n12s7T1RUEB1XEfELn`** を指定する
- ecosystem 横断の決定 (命名規則など) のみ `chronista-club` Atlas に置く
- SessionStart hook の Atlas 候補マッチング (ディレクトリ名 `club-kdl`) は Atlas 名 `Kdl Club` と
  exact match しないため、 **この id 指定が SSOT**

### 主要 memory

| 種別 | memory id |
|------|-----------|
| v1.0 roadmap | `mem_1Cb5fKiwL38fsD14Msab68` |
| Phase 1 ヒアリング結果 | `mem_1Cb5kDohizi4iT1PZJhALw` |
| Phase 1 spec (REQ-KDLGEN) | `mem_1Cb5mSTRoyRpZVmeZMs6vH` |
| Phase 1 design | `mem_1Cb5mWnMTdzXfJVoNGFwup` |
| design idea: flat schema fast path (spark) | `mem_1Cb5m77r4KN9EPn2L42VD6` |

## ロードマップ

v1.0 までの Phase 計画は `ROADMAP.md` 参照。 現在地: Phase 0 完了 (v0.5.1 公開済)、
Phase 1 (`club-kdl-codegen`) は spec/design 確定・実装着手前。

## 開発の基本

- workspace 構成: `club-kdl` (`.`、 feature `compose` / `compact` / `kdl-rs`、 すべて既定 off) + `club-kdl-derive` (`derive/`) + `club-kdl-codegen` (`codegen/`)。 `club-kdl-compose` は 0.14 で `club-kdl` の `compose` feature に統合した
- リリースは tag push (`v*.*.*`) で `.github/workflows/release.yml` が動く。 `verify` ジョブ (版とタグの照合、全テスト) のあと、
  `publish` ジョブは environment `crates-io` で **mako の承認待ち** になり、 承認されてから derive → club-kdl → codegen の順に publish する
- **エージェントの決まり**: PR は作って CI 結果まで報告し、 **main へのマージは mako の OK を待つ**。 environment の承認 (Review deployments) は
  **mako が画面で行い、 エージェントは `gh` などで代わりに承認しない**
- main はルールセットで保護している (強制 push と削除の禁止、 PR 必須、 CI 7 ジョブ必須。 semver-checks は破壊的変更の PR で赤になるのが正常なので必須にしない)
- PR は CI 8 job (fmt/clippy/doc + 3OS×2toolchain test + MSRV + deny + semver-checks) を通すこと
- **TS ツールの実行は `bun` / `bunx` を使う** (`npm` / `npx` は使わない)。 chronista-style ecosystem が bun ベース。 codegen の TS/Zod ターゲット出力も bun 消費を前提とする
