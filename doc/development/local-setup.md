# ローカル開発環境セットアップ

## 前提条件

- macOS (Apple Silicon または Intel)
- Rancher Desktop インストール済み
- kubectl インストール済み

## 環境確認

### 1. Rancher Desktop の起動

Rancher Desktopアプリケーションを起動し、Kubernetesが有効になっていることを確認します。

### 2. kubectl context の確認

```bash
# 現在のcontextを確認
kubectl config current-context
# 出力: rancher-desktop

# クラスター情報を確認
kubectl cluster-info
# 出力例:
# Kubernetes control plane is running at https://127.0.0.1:6443
# CoreDNS is running at https://127.0.0.1:6443/api/v1/namespaces/kube-system/services/kube-dns:dns/proxy
```

### 3. ノード情報の確認

```bash
kubectl get nodes
# 出力例:
# NAME                   STATUS   ROLES                  AGE   VERSION
# lima-rancher-desktop   Ready    control-plane,master   1h    v1.33.2+k3s1
```

## UNISON_FLEET 環境変数の設定

このリポジトリでは、`.mise.local.toml`により自動的に`UNISON_FLEET=local`が設定されます。

```bash
# ディレクトリに移動すると自動的に環境変数が設定される
cd /path/to/unison-fleets
echo $UNISON_FLEET
# 出力: local
```

## アプリケーションのデプロイ

### 1. 名前空間の作成

```bash
kubectl apply -f local/k8s/manifests/namespace.yaml
```

### 2. デモアプリケーションのデプロイ

```bash
# デモアプリケーションをデプロイ
kubectl apply -f local/k8s/manifests/unison-demo.yaml

# デプロイメントの確認
kubectl get all -n unison-system
```

### 3. アプリケーションへのアクセス

NodePort経由でアプリケーションにアクセスできます：

```bash
# ブラウザで開く
open http://localhost:30080

# または curl でテスト
curl http://localhost:30080
```

## Kustomizationを使用したデプロイ

より高度な設定管理には、Kustomizationを使用します：

```bash
# Local Fleet用の設定でデプロイ
kubectl apply -k fleets-config/local

# デプロイされたリソースを確認
kubectl get all -n unison-system
```

## トラブルシューティング

### Podが起動しない場合

```bash
# Pod の詳細情報を確認
kubectl describe pod <pod-name> -n unison-system

# ログを確認
kubectl logs <pod-name> -n unison-system
```

### NodePortにアクセスできない場合

1. Rancher Desktopの設定で「Port Forwarding」が有効になっているか確認
2. ファイアウォールの設定を確認
3. サービスが正しく作成されているか確認：

```bash
kubectl get svc -n unison-system
```

## 開発のヒント

- `kubectl port-forward`を使用して、NodePort以外でもアクセス可能：
  ```bash
  kubectl port-forward -n unison-system svc/unison-demo 8080:80
  ```

- リソースの変更を監視：
  ```bash
  kubectl get pods -n unison-system -w
  ```

- すべてのリソースを削除する場合：
  ```bash
  kubectl delete namespace unison-system
  ```