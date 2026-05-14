# Changelog

このプロジェクトの主要な変更を記録します。

フォーマットは [Keep a Changelog](https://keepachangelog.com/ja/1.0.0/) に基づいており、
このプロジェクトは [セマンティックバージョニング](https://semver.org/lang/ja/) に準拠しています。

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

## [0.3.0] - 2026-04-XX

### 追加

- enum data variants 対応 (struct / newtype / unit バリアントの KDL シリアライズ・デシリアライズ)

## [0.2.0]

### 追加

- `kdl_node_name()` 自動解決
- `#[kdl(alias = "...")]` 属性
- `usize` 型対応
- 網羅テスト整備
