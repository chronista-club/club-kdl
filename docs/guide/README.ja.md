# club-kdl ガイド

**[English](./README.md)** | 日本語

club-kdl を使うための詳しいガイド。トップページの `README.md` が全体像を紹介するのに対し、ここでは用途ごとの手順と判断基準を説明する。

| ガイド | 内容 |
|---|---|
| [ルールベースの自動エイリアス](./01-compact-names.ja.md) | 明示的に選択するコンパクト形式、名前対応表の再利用、読みやすい設定との使い分け |
| [原文を保持した値の編集](./02-preserve-value-edits.ja.md) | コメントや周囲のテキストを書き換えず、既存の引数・プロパティを更新する |
| [カスタム型ガイド](./custom-types.ja.md) | `FromKdlValue` / `ToKdlValue` を実装し、chrono の型、newtype、外部クレートの型を KDL の値として扱う |
| [KDL 設計のベストプラクティス](./best-practices.ja.md) | 引数・プロパティ・子ノードの使い分けと、避けたい設計例 |
| [トラブルシューティング](./troubleshooting.ja.md) | ライフタイム、子ノードの名前解決、型の不一致など、よくあるエラーの原因と対処 |
