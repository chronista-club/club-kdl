## 概要

<!-- 何を変更したか / なぜこの変更が必要か -->

## 変更内容

<!-- 主要な変更ポイントを箇条書きで -->

-
-

## 関連 issue

<!-- Closes #123 / Refs #456 など -->

## チェックリスト

- [ ] `cargo fmt --all -- --check` が通る
- [ ] `cargo clippy --all-targets --workspace -- -D warnings` が通る
- [ ] `cargo test --workspace --all-targets` が通る
- [ ] `cargo doc --no-deps --workspace` が warning なしで通る
- [ ] public API を変更した場合、 `CHANGELOG.md` を更新した
- [ ] 破壊的変更がある場合、 `CHANGELOG.md` に明記した

## 補足

<!-- レビュアーに見てほしい点 / 設計上の悩み / 動作確認方法など -->
