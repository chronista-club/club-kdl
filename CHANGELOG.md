# Changelog

このプロジェクトの主要な変更を記録します。

フォーマットは [Keep a Changelog](https://keepachangelog.com/ja/1.0.0/) に基づいており、
このプロジェクトは [セマンティックバージョニング](https://semver.org/lang/ja/) に準拠しています。

## [Unreleased]

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

[Unreleased]: https://github.com/chronista-club/club-kdl/compare/v0.5.1...HEAD
[0.5.1]: https://github.com/chronista-club/club-kdl/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/chronista-club/club-kdl/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/chronista-club/club-kdl/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/chronista-club/club-kdl/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/chronista-club/club-kdl/releases/tag/v0.2.0
