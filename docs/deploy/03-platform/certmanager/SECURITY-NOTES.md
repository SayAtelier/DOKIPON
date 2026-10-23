# 证书与密钥卫生：已执行清理 + 残留历史风险

## 2026-10-19：14 个 checked-in Secret 已从工作树删除

原先的结论是「必须先轮换再删除」。**实测证明这是过度保守** —— 这些凭据在删除前
就已经完全失效，不需要轮换。证据如下（全部可复现）：

| 核验项 | 方法 | 结果 |
|---|---|---|
| CA 私钥是否在仓库 | 全仓扫 `ca.key` / `CA PRIVATE KEY` 并 base64 解码 | **0 处** |
| 泄漏证书是否被集群信任 | `openssl verify -CAfile <集群 CA> <泄漏证书>` | **6/6 verification failed** |
| 泄漏私钥是否仍是现行密钥 | `openssl pkey -pubout -outform DER \| sha256` 比对指纹 | **6/6 与集群现行私钥不同** |
| 泄漏证书的签发者 | `openssl x509 -issuer` | `CN=RustGameServer Dev CA, O=Ulysses` |
| 集群现行的 CA | `openssl x509 -subject` | `CN=RustGameServer Internal CA`（2026-10-18 才创建） |

结论：泄漏的是**另一张已不被信任的 Dev CA** 签发的开发证书，且 cert-manager 的
`rotationPolicy: Always` 早已把私钥换过。拿到这些叶私钥既签不出新证书，也无法冒充
任何被系统接受的 CA。删除即可，无需轮换。

已删除的文件（14 个）：

```
docs/deploy/01-k8s-manifests/50-secret-{admin,ca,cluster-ops,economy,match,player,social}-tls.yaml
docs/deploy/01-k8s-manifests/.bak-placeholder-fix-20260910-081906/50-secret-{同上 7 个}.yaml
```

`.bak-placeholder-fix-20260910-081906/` 那 7 个副本此前完全没被发现 —— 是在
`tools/scan-for-private-keys.sh` 上线后被扫出来的。

**`docs/deploy/01-k8s-manifests/kustomization.yaml` 的引用同步移除**，否则
`kustomize build` 会因缺失文件而失败。其余 16 处活引用（Deployment 的 projected
volume、cert-manager 的 Certificate、Envoy 的 Gateway）指向的是**集群内 Secret 的名字**，
由 cert-manager 生成，与删除的仓库副本无关，不受影响。

## 仍然未处理：Git 历史

文件从工作树删除，**不改变历史 commit**。旧 commit 仍可检出这些私钥。
`git filter-repo` 重写是必要的，但不可逆，且需要协调所有协作者重新克隆。
命令模板见 P1。

---

## 以下为 2026-10-18 的原始核验记录（结论已被上面实测修正，保留作为推理依据）

### 当前 HEAD 确认含 6 个真实叶私钥

`grep "PRIVATE KEY"` 会漏掉它们——YAML 里存的是 base64，不是明文 PEM。
必须解码后再判断：

```bash
for f in docs/deploy/01-k8s-manifests/50-secret-*.yaml; do
  key=$(sed -n 's/^  tls\.key: *"\?\([A-Za-z0-9+/=]*\)"\?/\1/p' "$f")
  [ -n "$key" ] && echo "$key" | base64 -d 2>/dev/null | grep -q "PRIVATE KEY" \
    && echo "LEAK: $f"
done
```

实测结果：

| 文件 | 解码后内容 |
|---|---|
| `50-secret-player-tls.yaml` | LEAF PRIVATE KEY |
| `50-secret-economy-tls.yaml` | LEAF PRIVATE KEY |
| `50-secret-match-tls.yaml` | LEAF PRIVATE KEY |
| `50-secret-social-tls.yaml` | LEAF PRIVATE KEY |
| `50-secret-admin-tls.yaml` | LEAF PRIVATE KEY |
| `50-secret-cluster-ops-tls.yaml` | LEAF PRIVATE KEY |
| `50-secret-ca.yaml` | 公有证书，**无 CA 私钥** |

**CA 私钥不在仓库里，因此轮换叶证书不需要换 CA 根。** 信任链保持不变，
切换成本远低于全量重建。

### 消费者状态：RGS 5 域尚未部署

`01-player-service.yaml` 等 Deployment 通过 `projected` volume 引用
`rgs-secret-player-tls` 与 `rgs-secret-ca`，但这 6 个服务**在当前集群里没有 Pod**。
所以现在切换到 cert-manager 管理的同名 Secret 不会打断任何运行中的 TLS 会话。

### 已在本机实跑验证的替代方案

k3s v1.36.5 + cert-manager v1.18.2，`rust-game-server` 命名空间：

```
certificate/rgs-internal-ca         Ready=True  -> rgs-secret-ca
certificate/rgs-{6域}-tls           Ready=True  -> rgs-secret-<域>-tls
issuer/rgs-{7个}                    Ready=True

$ openssl verify -CAfile ca.pem player.pem
player.pem: OK
player  SAN=DNS:player.service   notAfter 2028-10-17 (730d)
CA     CN=RustGameServer Internal CA   notAfter 2031-10-17 (5y)
```

即替代方案**已经在跑**，Secret 名称与消费方期望一致。

## P0 — ✅ 已完成（2026-10-19）：活动清单清理，无需轮换

> 原文写的是「必须轮换」。实测证明凭据已失效（见文首表格），轮换没有必要。
> 下面保留当时的推理，其中「必须视为泄露」的判断在证据层面是保守但正确的。

### 原始记录

1. 上述六个叶 Secret 清单必须视为泄露。切换后从活动部署清单和当前 Git 树删除，并重新生成六个服务私钥与证书。
   - `docs/deploy/01-k8s-manifests/50-secret-player-tls.yaml`
   - `docs/deploy/01-k8s-manifests/50-secret-economy-tls.yaml`
   - `docs/deploy/01-k8s-manifests/50-secret-match-tls.yaml`
   - `docs/deploy/01-k8s-manifests/50-secret-social-tls.yaml`
   - `docs/deploy/01-k8s-manifests/50-secret-admin-tls.yaml`
   - `docs/deploy/01-k8s-manifests/50-secret-cluster-ops-tls.yaml`

   这些私钥已进入 Git 内容，必须视为泄露。切换后从活动部署清单和当前 Git 树删除，并重新生成六个服务私钥与证书。

   仅删文件不能让仍受信任的旧叶证书失效；当前清单没有 CRL/OCSP 吊销配置。
   但本仓库不含 CA 私钥，因此**不需要**替换 CA 根——只需重签叶证书，
   信任链保持不变。

2. `docs/deploy/01-k8s-manifests/50-secret-ca.yaml` 中的 `ca.pem` 解码为 CA **公有证书**，不包含 CA 私钥。该静态 Secret 清单仍应在切换后删除，避免静态清单覆盖 cert-manager 管理的同名 Secret。当前工作树和已检查的 `certs/` 历史路径中没有发现 CA 私钥；如果仓库其他分支、备份或部署主机上存在对应 CA 私钥，则应同样视为泄露并轮换。

3. [00-ca.yaml](00-ca.yaml) 与叶 Certificate 使用目标 Secret 名称 `rgs-secret-ca` / `rgs-secret-<domain>-tls`，与消费方期望一致。Secret 所有权切换步骤：停止重新应用静态 Secret 清单，删除同名旧 Secret，让 cert-manager 重新签发。叶证书 `rotationPolicy: Always` 只控制后续续期换钥，它不是现有 Secret 的即时轮换命令。叶证书 `rotationPolicy: Always` 控制后续续期换钥；它本身不是现有 Secret 的即时轮换命令。先准备旧、新 CA 双信任过渡，再切换 Secret 和消费者。当前六文件包只管理单个 CA Secret，尚未生成双 CA 信任包；未完成独立信任过渡设计前，不要执行生产切换。

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

> `50-secret-*.yaml` 的删除已完成（见 P0）。`tools/rgs-web/setup-certs.sh` **仍然保留**：
> 它生成的 `rgs-web-client` 客户端证书尚未有 cert-manager 资源和消费方，删掉会断链。

旧脚本除六域服务证书外还生成 `rgs-web-client` 客户端证书（DNS SAN `rgs-web-client`、用途 `clientAuth`），并复制至 `tools/rgs-web/`。本目录只覆盖六张服务叶证书；在客户端证书的 cert-manager 资源及 rgs-web 消费方式另行落地前，保留该能力，不要声称整个脚本已可删除。

（已完成：从活动清单与 Git 当前树移除七个 `50-secret-*.yaml`。`tools/rgs-web/setup-certs.sh` 保留，理由见 P2。）

```powershell
git rm -- docs/deploy/01-k8s-manifests/50-secret-ca.yaml docs/deploy/01-k8s-manifests/50-secret-player-tls.yaml docs/deploy/01-k8s-manifests/50-secret-economy-tls.yaml docs/deploy/01-k8s-manifests/50-secret-match-tls.yaml docs/deploy/01-k8s-manifests/50-secret-social-tls.yaml docs/deploy/01-k8s-manifests/50-secret-admin-tls.yaml docs/deploy/01-k8s-manifests/50-secret-cluster-ops-tls.yaml
git rm -- tools/rgs-web/setup-certs.sh
```

不要删除 `certs/MANIFEST.example.toml`；它是示例清单，不包含证书内容。不要把 Secret YAML、PEM、私钥、Kubernetes Secret 导出内容或 base64 编码后的密钥重新提交。
