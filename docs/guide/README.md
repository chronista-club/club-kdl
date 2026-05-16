# club-kdl ガイド

club-kdl の使い方を深掘りするガイド集。 トップの `README.md` が「概観」 なのに対し、
ここは「特定タスクの手順・判断基準」 を扱う。

| ガイド | 内容 |
|--------|------|
| [カスタム型ガイド](./custom-types.md) | `FromKdlValue` / `ToKdlValue` を実装して独自型 (chrono 型・newtype・外部 crate の型) を KDL 値にマッピングする |
| [KDL 設計ベストプラクティス](./best-practices.md) | argument / property / children の使い分け、 idiomatic な KDL スキーマ設計とアンチパターン |
| [トラブルシュート](./troubleshooting.md) | よくあるエラー (ライフタイム・子ノード解決・型不一致など) の原因と対処 |
