# 证书与密钥卫生：待执行清理项

本目录只新增证书管理资源和说明。以下是已从当前工作树与 Git 路径核验到的发现；任何清理、轮换和历史重写均未在本次执行。

## P0 — 立即安排轮换和活动清单清理

1. 当前受跟踪的六个叶 Secret 清单包含可解码为 PEM 私钥的 `data.tls.key`，同时包含对应 PEM 证书：
   - `docs/deploy/01-k8s-manifests/50-secret-player-tls.yaml`
   - `docs/deploy/01-k8s-manifests/50-secret-economy-tls.yaml`
   - `docs/deploy/01-k8s-manifests/50-secret-match-tls.yaml`
   - `docs/deploy/01-k8s-manifests/50-secret-social-tls.yaml`
   - `docs/deploy/01-k8s-manifests/50-secret-admin-tls.yaml`
   - `docs/deploy/01-k8s-manifests/50-secret-cluster-ops-tls.yaml`

   这些私钥已进入 Git 内容，必须视为泄露。切换后从活动部署清单和当前 Git 树删除，并重新生成六个服务私钥与证书。仅删文件不能让仍受信任的旧叶证书失效；当前清单没有 CRL/OCSP 吊销配置。需要轮换内部 CA、分阶段更新消费者信任、重新签发叶证书，再移除旧 CA 信任。

2. `docs/deploy/01-k8s-manifests/50-secret-ca.yaml` 中的 `ca.pem` 解码为 CA **公有证书**，不包含 CA 私钥。该静态 Secret 清单仍应在切换后删除，避免静态清单覆盖 cert-manager 管理的同名 Secret。当前工作树和已检查的 `certs/` 历史路径中没有发现 CA 私钥；如果仓库其他分支、备份或部署主机上存在对应 CA 私钥，则应同样视为泄露并轮换。

3. [00-ca.yaml](00-ca.yaml) 与叶 Certificate 使用目标 Secret 名称 `rgs-secret-ca` / `rgs-secret-<domain>-tls`。在应用前必须计划 Secret 所有权切换：停止重新应用静态 Secret 清单，并确保现有叶 Secret 中的私钥不会被 cert-manager 复用。叶证书 `rotationPolicy: Always` 控制后续续期换钥；它本身不是现有 Secret 的即时轮换命令。先准备旧、新 CA 双信任过渡，再切换 Secret 和消费者。当前六文件包只管理单个 CA Secret，尚未生成双 CA 信任包；未完成独立信任过渡设计前，不要执行生产切换。

## P1 — 清理 Git 历史与本地 PEM

1. 当前 `certs/` 仅跟踪 `certs/MANIFEST.example.toml`，没有当前 PEM 文件；但 Git 历史里曾提交以下五个 Secret YAML，每个都含有 `tls.key` 私钥：
   - `certs/player-tls.yaml`
   - `certs/economy-tls.yaml`
   - `certs/match-tls.yaml`
   - `certs/social-tls.yaml`
   - `certs/admin-tls.yaml`

   这些路径当前已不在工作树，但密钥内容仍可从历史 commit 读取。删除当前文件并不清除历史。仓库管理员应在协调维护窗口对所有分支与标签执行经批准的历史重写，并通知贡献者重新克隆；如果远端曾可被未授权者读取，应按已泄露凭据处理。

   历史重写命令模板（在仓库镜像副本中审查结果后，由仓库管理员执行）：

   ```powershell
   git filter-repo --invert-paths --path certs/player-tls.yaml --path certs/economy-tls.yaml --path certs/match-tls.yaml --path certs/social-tls.yaml --path certs/admin-tls.yaml --path docs/deploy/01-k8s-manifests/50-secret-player-tls.yaml --path docs/deploy/01-k8s-manifests/50-secret-economy-tls.yaml --path docs/deploy/01-k8s-manifests/50-secret-match-tls.yaml --path docs/deploy/01-k8s-manifests/50-secret-social-tls.yaml --path docs/deploy/01-k8s-manifests/50-secret-admin-tls.yaml --path docs/deploy/01-k8s-manifests/50-secret-cluster-ops-tls.yaml --path docs/deploy/01-k8s-manifests/50-secret-ca.yaml
   ```

   历史重写后还需由仓库维护者协调更新远端引用；不要在未协调共享分支的情况下强推。

2. `rgs-certgen` 会在输出目录生成 `ca.key.pem`、六个 `<domain>.key.pem` 和对应证书。当前 `certs/` 忽略规则排除这些文件，必须继续保持 PEM、私钥和隐藏证书文件不入 Git。当前目录没有待删除 PEM；若本地存在未跟踪副本，先核对路径再安全清理。`tools/rgs-web/setup-certs.sh` 还会写出 `tools/rgs-web/rgs-client.key.pem`、`rgs-client.crt.pem` 和 `rgs-ca.pem`；这些不是本证书包管理的文件，也不得提交。

## P2 — 移除旧生成入口并补齐客户端证书

旧脚本除六域服务证书外还生成 `rgs-web-client` 客户端证书（DNS SAN `rgs-web-client`、用途 `clientAuth`），并复制至 `tools/rgs-web/`。本目录只覆盖六张服务叶证书；在客户端证书的 cert-manager 资源及 rgs-web 消费方式另行落地前，保留该能力，不要声称整个脚本已可删除。

完成所有消费者切换后，再从活动清单与 Git 当前树移除七个 `50-secret-*.yaml` 和 `tools/rgs-web/setup-certs.sh`。命令仅供切换完成后使用：

```powershell
git rm -- docs/deploy/01-k8s-manifests/50-secret-ca.yaml docs/deploy/01-k8s-manifests/50-secret-player-tls.yaml docs/deploy/01-k8s-manifests/50-secret-economy-tls.yaml docs/deploy/01-k8s-manifests/50-secret-match-tls.yaml docs/deploy/01-k8s-manifests/50-secret-social-tls.yaml docs/deploy/01-k8s-manifests/50-secret-admin-tls.yaml docs/deploy/01-k8s-manifests/50-secret-cluster-ops-tls.yaml
git rm -- tools/rgs-web/setup-certs.sh
```

不要删除 `certs/MANIFEST.example.toml`；它是示例清单，不包含证书内容。不要把 Secret YAML、PEM、私钥、Kubernetes Secret 导出内容或 base64 编码后的密钥重新提交。
