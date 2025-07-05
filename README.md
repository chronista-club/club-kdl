# unison-fleets

unison プロジェクトのインフラストラクチャ管理とデプロイメントツール群

## 概要

unison-fleetsは、unisonアプリケーションを様々な環境（Fleet）にデプロイするためのInfrastructure as Code (IaC) ツールセットです。`UNISON_FLEET`環境変数によって環境を識別し、各Fleet固有の設定を適用します。

## 対応環境（Fleet）

### Local Fleet（ローカル開発環境）
- **プラットフォーム**: macOS + Rancher Desktop
- **Kubernetes**: Rancher Desktop内蔵K3s (context: `rancher-desktop`)
- **アクセス方法**: NodePort (30080)
- **用途**: 開発・テスト

### Edge Fleet（エッジ環境）
- **プラットフォーム**: オンプレミスサーバー、エッジデバイス
- **用途**: 低遅延が求められる環境、ローカルデータ処理

### Cloud Fleet（クラウド環境）
- **プラットフォーム**: AWS EKS, GCP GKE, Azure AKS
- **用途**: スケーラブルな本番環境

### Enterprise Fleet（エンタープライズ環境）
- **プラットフォーム**: プライベートクラウド、大規模Kubernetesクラスター
- **用途**: 高セキュリティ・コンプライアンス要件

## プロジェクト構造

```
unison-fleets/
├── .mise.local.toml    # ローカル環境変数設定 (UNISON_FLEET=local)
├── fleets-config/      # Fleet別Kubernetes設定（Kustomization）
│   ├── base/          # 共通設定
│   ├── local/         # Local Fleet用オーバーレイ
│   ├── edge/          # Edge Fleet用オーバーレイ
│   ├── cloud/         # Cloud Fleet用オーバーレイ
│   └── enterprise/    # Enterprise Fleet用オーバーレイ
├── local/             # Local Fleet固有リソース
│   └── k8s/
│       └── manifests/
├── scripts/           # ユーティリティスクリプト
└── doc/              # ドキュメント
    ├── architecture/  # アーキテクチャ設計
    ├── development/   # 開発ガイド
    └── operations/    # 運用ガイド
```

## クイックスタート

（準備中）

## ドキュメント

- [アーキテクチャ](./doc/architecture/)
- [開発ガイド](./doc/development/)
- [運用ガイド](./doc/operations/)

## ライセンス

Proprietary - All Rights Reserved