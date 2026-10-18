# Deployment → Rollout 迁移对照

10 个 Rollout 与原 Deployment 使用相同的资源名、selector 和完整 PodTemplate。容器命令、镜像初始引用、端口、探针、资源、安全上下文、env、卷和挂载从 Deployment 原样复制。差异集中在控制器与发布策略：`apps/v1 Deployment.spec.strategy` 替换为 `argoproj.io/v1alpha1 Rollout.spec.strategy.canary.steps`；副本数由环境 overlay 控制；旧 Deployment 在 overlay 中固定为 0 副本。

| 服务 | 旧 Deployment | 新 Rollout | 稳定 Service | Service 端口 |
|---|---|---|---|---|
| player | `player-service` | `player-service` | `player-service` | 50051/TCP, 7000/UDP, 9464/TCP |
| economy | `economy-service` | `economy-service` | `economy-service` | 50052/TCP, 9464/TCP |
| match | `match-service` | `match-service` | `match-service` | 50053/TCP, 7000/UDP, 9464/TCP |
| social | `social-service` | `social-service` | `social-service` | 50054/TCP, 9464/TCP |
| admin | `admin-service` | `admin-service` | `admin-service` | 50055/TCP, 8080/TCP, 9464/TCP |
| cluster-ops | `cluster-ops` | `cluster-ops` | `cluster-ops` | 50056/TCP, 9090/TCP, 9464/TCP |
| gm-backend | `gm-backend` | `gm-backend` | `gm-backend` | 8443/TCP, 8081/TCP, 9464/TCP |
| scene-service | `scene-service` | `scene-service` | `scene-service` | 50057/TCP, 9464/TCP |
| battle-service | `battle-service` | `battle-service` | `battle-service` | 50058/TCP, 9464/TCP |
| network-gateway | `network-gateway` | `network-gateway` | `network-gateway` | 7001/TCP, 50090/TCP, 8000/TCP, 4369/TCP, 9464/TCP |

稳定 Service 的名称、selector、端口和 targetPort 保持不变。`scene-service`、`battle-service` 与 `network-gateway` 的 Service 在原目录中未被基础 Kustomization 纳入，rollouts 子目录提供去除集群生成字段后的声明式 Service；其余稳定 Service 继续由基础清单提供。

每个 Rollout 依次执行 `setWeight: 10`、`50`、`100`，每步暂停 5 分钟并运行 `rgs-canary-analysis`。没有外部流量路由器时，Rollout 使用 stable Service 后端副本比例近似分流；10% 在低副本数下会取整，不能视为精确请求百分比。故障条件为 P99 ≥ 3 秒或 10 分钟错误率 ≥ 1%，Rollout 自动中止该次发布；无指标数据时分析为 inconclusive，Rollout 暂停等待处理。