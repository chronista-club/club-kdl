# unison Fleet設定

## 概要

`UNISON_FLEET`環境変数を使用して、異なるfleetでの設定を管理します。

## Fleet識別子

- `local`: ローカル開発環境（Rancher Desktop）
- `edge`: エッジ環境（オンプレミス、エッジコンピューティング）
- `cloud`: クラウド環境（AWS、GCP等）
- `enterprise`: エンタープライズ環境（大規模デプロイメント）

## 使用方法

```bash
# 環境変数を設定
export UNISON_FLEET=local

# または、コマンド実行時に指定
UNISON_FLEET=local kubectl apply -k fleets-config/local
```

## ディレクトリ構造

```
fleets-config/
├── base/           # 共通設定
├── local/          # ローカルfleet用オーバーレイ
├── edge/           # エッジfleet用オーバーレイ
├── cloud/          # クラウドfleet用オーバーレイ
└── enterprise/     # エンタープライズfleet用オーバーレイ
```

## Fleet別の特徴

### local fleet
- 開発者のローカル環境（Rancher Desktop、Kind等）
- NodePortでのアクセス
- 最小限のリソース設定

### edge fleet
- オンプレミス環境やエッジロケーション
- 低遅延、高可用性重視
- ローカルストレージ使用

### cloud fleet
- パブリッククラウド環境（AWS、GCP、Azure）
- オートスケーリング対応
- マネージドサービス統合

### enterprise fleet
- 大規模エンタープライズ環境
- 高度なセキュリティ要件
- マルチテナント対応