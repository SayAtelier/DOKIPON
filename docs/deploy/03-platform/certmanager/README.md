# RGS 内部证书管理

本目录定义 cert-manager 管理的 RGS 内部 CA 和六个域服务证书。适用于 cert-manager `cert-manager.io/v1` API；按任务给定环境，集群已有 cert-manager v1.18.2 及所需 CRD。本次只新增清单和说明，不执行集群操作。

## 替换范围

计划逐步替换以下静态资源和手工生成流程：

- `docs/deploy/01-k8s-manifests/50-secret-ca.yaml`
- `docs/deploy/01-k8s-manifests/50-secret-player-tls.yaml`
- `docs/deploy/01-k8s-manifests/50-secret-economy-tls.yaml`
- `docs/deploy/01-k8s-manifests/50-secret-match-tls.yaml`
- `docs/deploy/01-k8s-manifests/50-secret-social-tls.yaml`
- `docs/deploy/01-k8s-manifests/50-secret-admin-tls.yaml`
- `docs/deploy/01-k8s-manifests/50-secret-cluster-ops-tls.yaml`
- `tools/rgs-web/setup-certs.sh`
- 手工证书目录 `certs/`

当前检出的 `certs/` 仅跟踪 `MANIFEST.example.toml`，没有跟踪中的 PEM 文件；Git 历史曾包含 `certs/admin-tls.yaml`、`certs/economy-tls.yaml`、`certs/match-tls.yaml`、`certs/player-tls.yaml` 和 `certs/social-tls.yaml`，详见 [SECURITY-NOTES.md](SECURITY-NOTES.md)。因此，删除当前文件不能清除历史中的密钥。

`setup-certs.sh` 还会签发 `rgs-web-client` 客户端证书并复制到 `tools/rgs-web/`。本目录的六张域服务证书不替代该客户端证书；在为 `rgs-web` 单独定义 cert-manager `Certificate` 并切换消费者之前，不要删除或停用该脚本。

## 证书与 Secret 契约

六个服务 SAN 按 `setup-certs.sh` 调用的 `rgs-certgen` 默认域名及其 SAN 生成代码确认：

| 域 | DNS SAN | Secret |
|---|---|---|
| player | `player.service` | `rgs-secret-player-tls` |
| economy | `economy.service` | `rgs-secret-economy-tls` |
| match | `match.service` | `rgs-secret-match-tls` |
| social | `social.service` | `rgs-secret-social-tls` |
| admin | `admin.service` | `rgs-secret-admin-tls` |
| cluster-ops | `cluster-ops.service` | `rgs-secret-cluster-ops-tls` |

旧脚本向 `rgs-certgen` 传入 `--validity-days 730`。六个叶证书继续使用 730 天有效期，并在到期前 30 天续期（`duration: 17520h`、`renewBefore: 720h`）。内部 CA 有效期设为 5 年，也提前 30 天续期（`duration: 43800h`、`renewBefore: 720h`）。cert-manager 会自动申请续期；叶证书续期时生成新私钥。CA 证书续期保留 CA 公钥，CA 私钥轮换需要单独安排信任切换。

叶证书 Secret 名称及 `tls.crt` / `tls.key` 键保持不变，原服务证书投影可继续映射到 `/etc/rgs/certs/server.pem` 和 `server.key`。叶证书同时声明 `server auth`、`client auth` 用途，以兼容现有双向 TLS 探针。

**CA 投影必须在后续切换时调整一个键名。** cert-manager 的 `Certificate` 固定把目标 Secret 的证书与私钥写到 `tls.crt` / `tls.key`，不支持把证书输出键改名为 `ca.pem`；CA `Issuer` 也通过 `secretName` 读取该 TLS Secret，不接受自定义证书键。因此本配置将 CA Secret 名称保留为 `rgs-secret-ca`，CA 证书由 `tls.crt` 提供。消费者仍可在容器内使用相同文件 `/etc/rgs/certs/ca.pem`，但投影源需从：

```yaml
name: rgs-secret-ca
items:
  - key: ca.pem
    path: ca.pem
```

改为：

```yaml
name: rgs-secret-ca
items:
  - key: tls.crt
    path: ca.pem
```

此任务受限于新增文件，不修改 `01-k8s-manifests/` 中的消费者清单。切换前必须同步此投影变更。不要把 CA 私钥 `tls.key` 投影到工作负载。叶 Secret 中 cert-manager 可能额外写入 `ca.crt`；不要把该键当作长期信任包，CA 信任仍由单独的 `rgs-secret-ca` 提供。

## 安全修复与切换

当前分支的六个 `50-secret-*-tls.yaml` 含有可解码为 PEM 的叶证书和私钥。它们必须在切换完成后从活动部署清单和 Git 中删除，六个叶私钥必须重新生成。因为已签发的旧叶证书在旧 CA 仍受信任时仍可验证，且当前没有配置证书吊销机制，安全轮换还需要更换 CA 并按双信任阶段迁移消费者。单纯删除文件或只重签叶证书不能让泄露的旧证书立即失效。完整优先级和仓库历史处理见 [SECURITY-NOTES.md](SECURITY-NOTES.md)。

仓库文件清理命令仅在消费者切换完成后执行：

```powershell
git rm -- docs/deploy/01-k8s-manifests/50-secret-ca.yaml docs/deploy/01-k8s-manifests/50-secret-player-tls.yaml docs/deploy/01-k8s-manifests/50-secret-economy-tls.yaml docs/deploy/01-k8s-manifests/50-secret-match-tls.yaml docs/deploy/01-k8s-manifests/50-secret-social-tls.yaml docs/deploy/01-k8s-manifests/50-secret-admin-tls.yaml docs/deploy/01-k8s-manifests/50-secret-cluster-ops-tls.yaml
```

为保证证书管理器生成新叶私钥，受控维护窗口内需清除旧 Secret 后再创建新 Certificate。此操作会让现有服务暂时失去旧信任；本目录尚未提供双 CA 信任包，未完成信任切换或安排维护窗口前不得执行：

```powershell
kubectl -n rust-game-server delete secret rgs-secret-ca rgs-secret-player-tls rgs-secret-economy-tls rgs-secret-match-tls rgs-secret-social-tls rgs-secret-admin-tls rgs-secret-cluster-ops-tls --ignore-not-found
```

以下命令是**完成信任迁移和静态 Secret 清理后的部署/核验命令**。它们仅供后续切换使用；本次没有运行任何 `kubectl` 命令。

```powershell
# 从仓库根目录进入本目录，仅应用这里的 cert-manager 资源。
Set-Location docs/deploy/03-platform/certmanager
kubectl apply -k .

# 等待 CA、六个 Issuer 和七张 Certificate 就绪。
$issuerNames = @('rgs-selfsigned-bootstrap', 'rgs-player-ca-issuer', 'rgs-economy-ca-issuer', 'rgs-match-ca-issuer', 'rgs-social-ca-issuer', 'rgs-admin-ca-issuer', 'rgs-cluster-ops-ca-issuer')
foreach ($name in $issuerNames) {
    kubectl -n rust-game-server wait --for=condition=Ready "issuer/$name" --timeout=5m
    if ($LASTEXITCODE -ne 0) { throw "Issuer is not Ready: $name" }
}
$certificateNames = @('rgs-internal-ca', 'rgs-player-tls', 'rgs-economy-tls', 'rgs-match-tls', 'rgs-social-tls', 'rgs-admin-tls', 'rgs-cluster-ops-tls')
foreach ($name in $certificateNames) {
    kubectl -n rust-game-server wait --for=condition=Ready "certificate/$name" --timeout=5m
    if ($LASTEXITCODE -ne 0) { throw "Certificate is not Ready: $name" }
}

# 查看签发状态、SAN 与有效期；describe secret 只显示键名和字节数，不输出 Secret 值。
kubectl -n rust-game-server get issuers,certificates
kubectl -n rust-game-server describe certificate rgs-internal-ca
kubectl -n rust-game-server describe certificate rgs-player-tls
kubectl -n rust-game-server describe certificate rgs-economy-tls
kubectl -n rust-game-server describe certificate rgs-match-tls
kubectl -n rust-game-server describe certificate rgs-social-tls
kubectl -n rust-game-server describe certificate rgs-admin-tls
kubectl -n rust-game-server describe certificate rgs-cluster-ops-tls
kubectl -n rust-game-server describe secret rgs-secret-ca
kubectl -n rust-game-server describe secret rgs-secret-player-tls
kubectl -n rust-game-server describe secret rgs-secret-economy-tls
kubectl -n rust-game-server describe secret rgs-secret-match-tls
kubectl -n rust-game-server describe secret rgs-secret-social-tls
kubectl -n rust-game-server describe secret rgs-secret-admin-tls
kubectl -n rust-game-server describe secret rgs-secret-cluster-ops-tls
```

## gm-console 公网证书：Let's Encrypt

公网 gm-console 应使用 ACME/Let's Encrypt，不应由 RGS 内部 CA 签发。以下是使用 HTTP-01 的模板：替换邮箱、域名和公网 Ingress class，并确保公网 TCP/80 可达且挑战请求会路由到 cert-manager。若入口不是 Ingress 或不能满足 HTTP-01，应改用已经配置凭据的 DNS-01 solver。

```yaml
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    email: replace-me@example.com
    server: https://acme-v02.api.letsencrypt.org/directory
    privateKeySecretRef:
      name: letsencrypt-prod-account
    solvers:
      - http01:
          ingress:
            ingressClassName: replace-with-public-ingress-class
---
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: gm-console-public
  namespace: rust-game-server
spec:
  secretName: gm-console-public-tls
  dnsNames:
    - gm.example.com
  issuerRef:
    name: letsencrypt-prod
    kind: ClusterIssuer
    group: cert-manager.io
```

现有 `55-57-gm-console-envoy-*.yaml` 定义的是 ClusterIP 上的 HTTP/8080 Envoy listener，没有 TLS listener、证书 Secret 投影或公网 Ingress。签发 ACME Certificate 还不够：后续需在 Envoy listener 配置 TLS、在 Deployment 投影 `gm-console-public-tls` 的 `tls.crt` / `tls.key`，并由实际公网入口提供 ACME challenge 路由及 HTTPS 访问。不要把此公网证书接到内部 CA Issuer。

参考：[cert-manager 自签名 Issuer](https://cert-manager.io/docs/configuration/selfsigned/)、[CA Issuer](https://cert-manager.io/docs/configuration/ca/)、[Certificate 与 Secret 输出](https://cert-manager.io/docs/usage/certificate/)及 [ACME Issuer](https://cert-manager.io/docs/configuration/acme/)。
