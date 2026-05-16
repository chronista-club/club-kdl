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

### Phase 1 — Schema Codegen 連携 foundation

club-kdl を `kdl-schema` crate (`unison monorepo` 内、 creo-memories の type SSOT) の基盤にする。

- public API audit: schema-friendly な trait 拡張 (例: `kdl_schema()` で型情報を runtime 取得)
- TypeScript / Zod / Rust / SurrealQL の多言語コード生成基盤に乗せる
- 関連: [unison monorepo の `kdl-schema` 設計](https://linear.app/chronista/issue/CREO-10)

### Phase 2 — Ecosystem dogfood

下流 caller での実戦投入で API friction を炙り出す。

- **midistage-keystage**: KDL serde 採択候補 (knus or kdl-rs から swap)
- **creo-memories Story KDL**: シリアライズ基盤の swap 候補
- **fleetflow** `flow.kdl`: 既存 KDL ユーザーの migration 検討

各 caller での pain point を Issue / Discussion で collect。

### Phase 3 — API stabilization

v1.0 freeze に向けた breaking changes を一括投入。

- public API audit: 削除候補 / 改名候補の整理
- `cargo-semver-checks` baseline 確立
- breaking changes 全部投入 → **v0.6.0** → **v1.0.0-beta.1**

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
| `v0.6.0`        | API audit + breaking changes 全部投入         | 未着手            |
| `v1.0.0-beta.1` | API freeze + beta soak (1 month)             | 未着手            |
| `v1.0.0`        | stability commit (v2.0 まで breaking なし)    | 未着手            |

## 依存・連携

| 周辺プロジェクト | 関係性 |
|---------------|-------|
| **club-unison** v1.0 sprint | polyglot client base、 並走 (memory: `mem_1Cb4kaKYtarGUmxMXyEDZG`) |
| **kdl-schema** crate (unison) | club-kdl を foundation として使用 |
| **midistage-keystage**        | dogfood caller 候補 |
| **creo-memories Story KDL**   | dogfood caller 候補 |

## Notes

- creo-memories 側にも同期済み: memory `mem_1Cb5fKiwL38fsD14Msab68` (category `todo` / status `active` / Atlas `chronista-club`)
- Atlas は現状 `chronista-club`、 将来単独 `club-kdl` Atlas 化を検討
