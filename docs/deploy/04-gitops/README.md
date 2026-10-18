# RGS GitOps 晋级流水线

## 链路

GitHub 推送新版本与镜像后，Kargo 的 10 个 `Warehouse` 按服务 tag pattern 发现镜像和 Git revision，自动生成包含镜像 digest 的 Freight。Freight 还必须带有 OCI `source`、`revision` 和 `sbom` 注解，缺少任一项不会生成可晋级 Freight。Kargo 的 `dev` 阶段自动晋级并运行 10 分钟 Prometheus 分析；验证通过后自动晋级到 `staging`。`staging` 分析完成后，`prod` 不自动晋级，需人工批准并通过 Kargo CLI/UI 启动。每次晋级都把服务镜像更新为 Freight digest，提交到 `stage/dev`、`stage/staging` 或 `stage/prod` 分支，切换 `overlays/active`，并通过 `argocd-update` 让 Argo CD 同步唯一的 `rgs` 应用。

Argo Rollouts 对每个服务按 `10% → 50% → 100%` 推进，每个百分比暂停 5 分钟并运行 P99 与错误率分析。Rollout 沿用既有稳定 Service 名称和端口。当前没有配置服务网格或 Ingress 流量路由器，因此 `setWeight` 通过稳定 Service 后端 Pod 副本比例近似分配流量；副本数较小时，实际请求百分比会有取整误差。

## 文件与依赖顺序

| 路径 | 用途 |
|---|---|
| `kargo-project.yaml` | 声明 Kargo Project；保留 `rust-game-server` 命名空间 |
| `kargo/10-freight.yaml` | 每个服务一个 Warehouse，发现受限 tag、Git revision 和 SBOM 注解 |
| `kargo/30-analysis.yaml` | Stage 验证模板，检查 10 分钟 P99 与错误率 |
| `kargo/05-promotion-task.yaml` | 将 Freight digest 写入环境 overlay，并更新活动环境分支 |
| `kargo/00-stage.yaml` | 声明 dev、staging、prod 请求关系、promotion 和验证 |
| `kargo/20-promotionpolicy.yaml` | Kargo 1.12 的 `ProjectConfig.spec.promotionPolicies`；该版本不使用独立 `PromotionPolicy` CR |
| `argocd/00-project.yaml` | 限制 Argo CD 来源仓库和目标 namespace |
| `argocd/10-application.yaml` | 活动 RGS 应用，开启自动同步、自愈、prune 与 allow-empty |
| `argocd/20-overlay-dev.yaml` | 手动同步的 dev overlay 预览应用 |
| `rollouts/` | 10 个 Rollout、共享 AnalysisTemplate，以及补入 Kustomize 基础目录未引用的 Service |
| `overlays/{dev,staging,prod}/` | 环境副本数、安全参数和镜像 digest 输入 |
| `overlays/active/` | Argo CD 固定监听的单一入口，由 Kargo 在晋级时切换 |
| `overlays/common/prometheus.yml` | Prometheus 服务抓取配置，直接访问各服务 9464 指标端口 |

## 从手工 apply 切换

先为 Kargo 和 Argo CD 配置仓库读取凭据；Kargo 还需要向 `stage/*` 分支写入的 Git 凭据。凭据应由现有 Secret 管理流程提供，不要提交到 Git。GHCR 镜像应带上以下 OCI 注解，并让 `sbom` 注解指向 CycloneDX 或 SPDX SBOM/attestation：

- `org.opencontainers.image.source`
- `org.opencontainers.image.revision`
- `org.opencontainers.image.sbom`

引导命令：

```powershell
kubectl apply -k docs/deploy/04-gitops/
```

该操作创建 Kargo Project、Kargo stages、AppProject 和 Application。Argo CD 的 `rgs` 应用从 `overlays/active` 同步完整清单；第一次同步会将旧 Deployment 副本设为 0，由对应 Rollout 接管，Service 名称保持不变。旧的 `docs/deploy/01-k8s-manifests/` 文件不会删除。

生产晋级需先在 Kargo UI 选择已在 staging 验证的 Freight，或运行：

```powershell
kargo promote --project rust-game-server --stage prod --freight <FREIGHT_NAME>
```

`kargo promote` 与 Kargo UI 的 Promote 操作等价；Argo CD UI 中的 `rgs` 应用显示同步与 Rollout 健康状态。日常晋级由 Kargo 调整 Application 的目标 revision，避免人工直接修改 Deployment 镜像。

## 验证命令

以下命令用于部署后检查；本次作者流程没有运行 `kubectl`，也没有访问集群。

```powershell
python -c "import pathlib,yaml; files=list(pathlib.Path('docs/deploy/04-gitops').rglob('*.yaml'))+list(pathlib.Path('docs/deploy/04-gitops').rglob('*.yml')); [list(yaml.safe_load_all(p.read_text(encoding='utf-8'))) for p in files]; print(f'{len(files)} YAML files parsed')"
kubectl get projects.kargo.akuity.io rust-game-server
kubectl get stages.kargo.akuity.io -n rust-game-server
kubectl get warehouses.kargo.akuity.io -n rust-game-server
kargo get stage --project rust-game-server
kargo get warehouse --project rust-game-server
argocd app get rgs
kubectl get rollouts.argoproj.io,analysisruns.argoproj.io -n rust-game-server
kubectl get services -n rust-game-server
```

## Prometheus 指标

P99 延迟 PromQL：

```promql
histogram_quantile(0.99, sum by (le) (rate(rgs_http_request_duration_seconds_bucket{service=~"{{args.service-name}}"}[10m])))
```

错误率 PromQL：

```promql
sum(rate(rgs_http_requests_total{service=~"{{args.service-name}}",status=~"(?i)(5..|5xx|error|failed|failure)"}[10m])) / clamp_min(sum(rate(rgs_http_requests_total{service=~"{{args.service-name}}"}[10m])), 1e-9)
```

成功条件分别为 P99 `< 3` 秒、错误率 `< 0.01`；越界即失败，无样本会标记为 inconclusive。Prometheus overlay 抓取服务 9464 端口；请确认各镜像继续暴露这些 exporter 指标。