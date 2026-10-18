# 08 — デプロイと運用（Multipass）

> **STATUS (2026-10-18)**: 自動デプロイ未実行。`deploy/deploy.sh` は
> コミット済み・VM 上で手動実行可能。CI からは叩いていない。
> Multipass の Hyper-V バックエンドで VHDX が C: ドライブに
> 置かれるため、C: ドライブの空き容量が 5 GB を下回ると
> `cargo build --release` が `EDQUOT` で失敗する既知の問題あり。

## 1. 前提

| 要件 | 状態 |
|---|---|
| Multipass 1.16+ (Hyper-V バックエンド) | 必須 |
| `informed-impala` インスタンス (Ubuntu 26.04) | 1 vCPU / 1 GB / 5 GB 既定 |
| VM 内に rustup で stable インストール | `curl https://sh.rustup.rs \| sh -s -- -y --profile minimal` |
| VM 内に C/C++ ツールチェイン | `apt install -y build-essential pkg-config libssl-dev` |
| ソース | `/tmp/dokipon`（`git clone https://github.com/SayAtelier/DOKIPON.git`） |

## 2. ワンショットインストール

VM 内で：

```bash
git clone https://github.com/SayAtelier/DOKIPON.git /tmp/dokipon
cd /tmp/dokipon
git checkout main
cd /tmp/dokipon/services
cargo build --release           # 7 binaries
sudo bash ../deploy/deploy.sh install
```

deploy.sh がやること：
- ユーザー `commerce` を作成（system、nologin）
- バイナリを `/opt/commerce/bin/{commerce-bff,commerce-identity,...}` へ
- `/etc/systemd/system/commerce-*.service` を 7 本書き出す
  - `commerce-bff.service` は `IDENTITY_URL` / `CATALOG_URL` /
    `CART_URL` / `INVENTORY_URL` / `ORDER_URL` /
    `PAYMENT_ADAPTER_URL` の 6 つの環境変数を持つ
- `systemctl enable --now` ですべて起動
- ログは `/var/log/commerce/commerce-*.log` へ

## 3. 状態確認

```bash
sudo bash deploy/deploy.sh status
# commerce-bff                 ACTIVE
# commerce-identity            ACTIVE
# ...
sudo bash deploy/deploy.sh logs commerce-bff | tail -50
```

## 4. アップデート

```bash
cd /tmp/dokipon
git pull --ff-only
cd services
cargo build --release
sudo bash ../deploy/deploy.sh restart
```

`restart` は 7 サービスすべてを再起動する。
サービス個別にやるなら `systemctl restart commerce-order.service` で十分。

## 5. アンインストール

```bash
sudo bash deploy/deploy.sh uninstall
# サービス停止 → unit ファイル削除 → /opt/commerce 削除
```

## 6. ポート割当

| サービス | ポート |
|---|---|
| commerce-bff | 8080 |
| commerce-identity | 8081 |
| commerce-catalog | 8082 |
| commerce-cart | 8083 |
| commerce-inventory | 8084 |
| commerce-order | 8085 |
| commerce-payment-adapter | 8086 |

`OpenAPI.yaml` と同じ値。クライアントは **BFF（8080）** を叩く。
BFF が各サービスにプロキシする。

## 7. 既知の制限

- `commerce-mocks` は workspace から外している。
  `cargo test --workspace` の対象外。VM 上では個別実行：
  ```bash
  cd /tmp/dokipon/services
  cargo test -p commerce-mocks   # 個別なら通る（axum を立てない単体テストのみ）
  ```
- TLS / secret manager / PostgreSQL 実接続 / 実 PSP / LIFF 実機審査は
  未実装（[9 条 Release Blocker](../04-detailed/セキュリティ設計.md#5-release-blocker) 参照）。
- 本書は **手動運用想定**。CD パイプライン化は v8.4 以降の作業。
