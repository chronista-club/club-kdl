# club-kdl Roadmap — to v1.0

> v0.5.0 で `unison-kdl` → `club-kdl` rename + crates.io publish が一段落。 ここから API stabilization + ecosystem dogfood で v1.0 を目指す。

## Phases

### Phase 0 — Release quality baseline (= v0.5.1) — ✅ **完了 (2026-05-16)**

リリース品質の整備。 public API は変更しない (semver patch)。

- 致命: fmt + clippy + MSRV (`1.94`) + dual license (MIT OR Apache-2.0) + derive crate metadata + `[package.metadata.docs.rs]`
- CI 強化: `fmt --check` / `clippy -D warnings` / `doc -D warnings` / multi-OS (ubuntu+macOS+windows) × multi-toolchain (stable+beta) / MSRV / `cargo-deny` / `cargo-semver-checks`
- Release workflow: tag push (`v*.*.*`) → derive → main の順で `cargo publish`、 `workflow_dispatch` で dry-run support
- OSS hygiene: `CONTRIBUTING.md` / `SECURITY.md` / `CODE_OF_CONDUCT.md` / `dependabot.yml` / Issue+PR templates / `deny.toml`
- Polish: README badges (CI/MSRV/Downloads) / kdl-rs+knuffel+knus 比較セクション / benchmark 数値 (read 115x slower than JSON, write 5x slower) / doctest 5 件復活 / CHANGELOG compare links

**成果**: PR #7 merge → `v0.5.1` tag → crates.io 公開 (`club-kdl` 0.5.1 + `club-kdl-derive` 0.5.1) → [GitHub Release v0.5.1](https://github.com/chronista-club/club-kdl/releases/tag/v0.5.1)。

### Phase 1 — Schema Codegen foundation (`club-kdl-codegen`)

> 設計方針はヒアリング (2026-05-16) で確定。 詳細 memory: `mem_1Cb5kDohizi4iT1PZJhALw`。

**前提発見**: club-unison (`crates/unison-protocol`) は既に KDL-first の codegen を実装済み
(`schemas/*.kdl` → `src/parser/schema.rs` → `src/codegen/{rust,typescript}.rs`、 既に `club-kdl` 依存)。
Phase 1 は「ゼロから作る」 ではなく **club-unison の codegen を抽出・汎用化する**。

- **配置**: `club-kdl-codegen` を club-kdl repo の 3 つ目 workspace member として新設
  (club-unison は build-dep として利用)
- **SSOT**: KDL-first — KDL schema ファイルが型の正
- **KDL schema の 2 方言**: data 方言 (`struct`/`enum`/`field` = 型定義、 core) と
  protocol 方言 (`protocol`/`channel`/`request` = RPC IDL、 club-unison が拡張)
- **codegen ターゲット**: Rust / TypeScript は club-unison から移植、 **Zod / SurrealQL を並行で新規**
- **club-kdl 本体 (lib)**: codegen の都合で必要なら breaking 変更も許容 (→ v0.6.0 前倒しの可能性)
- **ゴール**: club-unison を `club-kdl-codegen` ベースに載せ替え (出力 diff でリグレッション検知) +
  4 ターゲット出力可 + 各ターゲットに example/snapshot test
- **派生論点**: SurrealQL の出力仕様 (enum → `ASSERT IN [...]` / nested → `object` or `record<table>` /
  relation 表現) は設計段階で詰める
- creo-memories の型同期 dogfood は Phase 2

### Phase 2 — Ecosystem dogfood

下流 caller での実戦投入で API friction を炙り出す。

- **midistage-keystage**: KDL serde 採択候補 (knus or kdl-rs から swap)
- **creo-memories Story KDL**: シリアライズ基盤の swap 候補
- **fleetflow** `flow.kdl`: 既存 KDL ユーザーの migration 検討

各 caller での pain point を Issue / Discussion で collect。

### Phase 3 — API stabilization

v1.0 freeze に向けた breaking changes を整理・確定。

- public API audit: 削除候補 / 改名候補の整理
- `cargo-semver-checks` baseline 確立
- breaking changes を確定 → **v1.0.0-beta.1**

> 注: Phase 1 で codegen の都合により本体 breaking が入った場合、 **v0.6.0 は Phase 1 中に切られる**
> 可能性がある (当初の「breaking は Phase 3 で一括」 方針を緩和)。

### Phase 4 — Documentation polish

- 全 `#[kdl(...)]` 属性の完全な doc 書き起こし
- `examples/` にユースケース別 example を充実 (config / DSL / schema 等)
- docs.rs での見た目最終調整 (rustdoc-args / feature flag visibility)

### Phase 5 — v1.0 release

- **v1.0.0-beta.1** → 1 month soak (dogfood + early adopter feedback)
- **v1.0.0** = API stability commit (v2.0 まで public API breaking なし)

## Release milestones

| Tag             | テーマ                                       | 状態              |
|-----------------|---------------------------------------------|-------------------|
| `v0.5.1`        | 品質整備 (= Phase 0)                          | ✅ 2026-05-16 公開 |
| `v0.6.0`        | `club-kdl-codegen` 新設 + 本体 breaking (Phase 1 中、 必要時) | 未着手 |
| `v1.0.0-beta.1` | API freeze + beta soak (1 month)             | 未着手            |
| `v1.0.0`        | stability commit (v2.0 まで breaking なし)    | 未着手            |

## 依存・連携

| 周辺プロジェクト | 関係性 |
|---------------|-------|
| **club-unison** (`crates/unison-protocol`) | 既に KDL-first codegen を実装済み・`club-kdl` 依存。 Phase 1 の抽出元かつ最初の dogfood |
| **midistage-keystage**        | dogfood caller 候補 (Phase 2) |
| **creo-memories Story KDL**   | dogfood caller 候補 (Phase 2、 型同期の最大の消費者) |
| **fleetflow** `flow.kdl`      | 既存 KDL ユーザー、 migration 検討 (Phase 2) |

## Notes

- roadmap memory: `mem_1Cb5fKiwL38fsD14Msab68` / Phase 1 設計方針: `mem_1Cb5kDohizi4iT1PZJhALw` (どちらも Atlas `chronista-club`)
- Atlas は現状 `chronista-club`、 将来単独 `club-kdl` Atlas 化を検討
