# 開発ドキュメント

## 概要

unison-fleetsの開発に関するガイドライン

## ドキュメント一覧

- [ローカル開発環境セットアップ](./local-setup.md) - Rancher Desktop を使用したローカル環境構築
- Fleet アーキテクチャ（準備中）
- IaCコーディング規約（準備中）
- モジュール開発ガイド（準備中）
- テスト作成ガイド（準備中）

## 開発の流れ

1. **ローカル環境でのテスト** (`UNISON_FLEET=local`)
   - Rancher Desktop上でアプリケーションを実行
   - NodePort経由でアクセステスト

2. **Fleetへのデプロイ**
   - 各Fleet用のKustomizationオーバーレイを適用
   - Fleet固有の設定（リソース制限、レプリカ数等）を調整

3. **監視とログ**
   - kubectl logsでアプリケーションログを確認
   - メトリクスエンドポイント（:9090/metrics）で監視

## 環境変数

主要な環境変数：

- `UNISON_FLEET`: 現在のFleet識別子（local/edge/cloud/enterprise）
- `RUST_LOG`: ログレベル設定
- `NODE_ENV`: Node.js環境設定（該当する場合）

---

最終更新: 2025年1月5日