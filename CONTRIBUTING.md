# Contributing to club-kdl

開発に参加いただきありがとうございます。 このドキュメントは club-kdl への貢献手順をまとめたものです。

## はじめに

- バグ報告 / 機能要望は [Issues](https://github.com/chronista-club/club-kdl/issues) へ
- セキュリティ問題は **Issue を立てず**、 [SECURITY.md](./SECURITY.md) の手順に従ってください

## 開発環境

| ツール | バージョン |
|--------|----------|
| Rust toolchain | `Cargo.toml` の `rust-version` 以上 (現在 1.94) |
| edition | 2024 |

`rustup` または [`mise`](https://mise.jdx.dev/) で toolchain を管理してください。

```sh
# mise を使う場合
mise use rust@1.94
```

## ローカル開発フロー

```sh
# fork & clone
git clone https://github.com/<your-username>/club-kdl.git
cd club-kdl

# branch を切る
git checkout -b feat/<short-description>

# 変更 → 確認
cargo fmt --all
cargo clippy --all-targets --workspace -- -D warnings
cargo test --workspace --all-targets
cargo doc --no-deps --workspace

# commit
git add .
git commit -m "feat: short description"
```

## CI が通る条件

PR は以下を全てパスする必要があります:

- `cargo fmt --all -- --check` — 整形
- `cargo clippy --all-targets --workspace -- -D warnings` — lint
- `cargo test --workspace --all-targets` — テスト (ubuntu / macOS / windows × stable + beta)
- `cargo doc --no-deps --workspace` — ドキュメント (warning なし)
- MSRV (`rust-version` 指定の Rust) で `cargo check` が通る
- `cargo-deny` — license / advisories
- `cargo-semver-checks` — 既存 public API への破壊的変更検出

ローカルで上記を全て通してから push してください。

## コミット規約

| prefix | 用途 |
|--------|------|
| `feat:` | 新機能 |
| `fix:` | バグ修正 |
| `docs:` | ドキュメント更新 |
| `test:` | テスト追加・修正 |
| `chore:` | 雑多な変更 (依存更新・version bump 等) |
| `refactor:` | 挙動を変えないコード整理 |
| `perf:` | パフォーマンス改善 |
| `ci:` | CI workflow 変更 |

メッセージは **日本語または英語** どちらでも構いません。

## バージョニング

[Semantic Versioning](https://semver.org/lang/ja/) に準拠します。

- `MAJOR` — public API の破壊的変更
- `MINOR` — 後方互換のある機能追加
- `PATCH` — 後方互換のあるバグ修正

derive crate (`club-kdl-derive`) は本体と **同じバージョン** をリリースします。

## リリース手順 (maintainer 向け)

1. `CHANGELOG.md` を更新
2. `Cargo.toml` の `[workspace.package].version` を bump
3. PR を作成 → merge
4. `git tag v0.X.Y` を main で打つ → push
5. `release.yml` workflow が自動で crates.io に publish

dry-run 確認は GitHub Actions の `Release` workflow を `workflow_dispatch` で `dry_run=true` 起動。

## 行動規範

このプロジェクトは [Contributor Covenant](./CODE_OF_CONDUCT.md) を採用しています。
