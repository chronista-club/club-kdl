# unison-fleets

unison プロジェクトのインフラストラクチャ管理とデプロイメントツール群

## 概要

unison-fleetsは、unisonアプリケーションをローカル環境およびGCP環境にデプロイするためのInfrastructure as Code (IaC) ツールセットです。

## 対応環境

- **ローカル開発環境**: Docker Compose, Kind
- **GCP**: GKE, Cloud Run, Compute Engine

## プロジェクト構造

```
unison-fleets/
├── local/          # ローカル環境用設定
├── gcp/            # GCP環境用設定
├── modules/        # 再利用可能なモジュール
├── scripts/        # ユーティリティスクリプト
└── doc/            # ドキュメント
    ├── architecture/   # アーキテクチャ設計
    ├── development/    # 開発ガイド
    └── operations/     # 運用ガイド
```

## クイックスタート

（準備中）

## ドキュメント

- [アーキテクチャ](./doc/architecture/)
- [開発ガイド](./doc/development/)
- [運用ガイド](./doc/operations/)

## ライセンス

Proprietary - All Rights Reserved