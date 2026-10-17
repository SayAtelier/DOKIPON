# GemArc Commerce — 设计到实现映射说明

> 本文件由 `docs/GemArc-5D-Design-Atlas-v5.4` 改造而来。原图谱集（Design Atlas）描述的是「GemArc 5D Flutter Workbench」——一个架构设计/推演工作台。
> 本仓库把它**收敛为电商交易系统**：保留 0D–5D 设计方法论、IPA 需求/基本/详细设计、状态机与 Saga、幂等/Outbox、安全威胁模型，
> 去掉纯工作台专属的 3D 楼层投影/因果实验室 UI，把它落成可运行的 BFF + 领域服务 + Flutter 三端（App / Web / LINE MINI App）。

## 1. 改造对照表

| Atlas 原件 | 本仓库落点 | 处理 |
|---|---|---|
| `01_要件定義書.md` FR-CHECKOUT/CATALOG/LINE/SECURITY | `services/order`、`services/catalog`、`apps/line_web`、`services/identity` | **保留**，其余 12 个工作台 FR（GRAPH/FOCUS/SPATIAL/CAUSAL/WORKFLOW-\*/PERSISTENCE/INTEROP/EVIDENCE/RELEASE/OFFLINE/NOTIFICATION）改为后端横切实现约束 |
| `02_基本設計書.md` C4 逻辑图 | `docs/03-基本设计/README.md` + 真实服务拓扑 | **重画**为电商 C4 |
| `03_詳細設計書.md` 因果算法/3D 投影/Workflow 画布 | `docs/04-详细设计/`：Order FSM、Saga、Outbox、幂等 | **重写**（删 3D/画布，保留纯函数状态机思想） |
| `AI_IMPLEMENTATION_BLUEPRINT.md` §B 七服务 | `services/bff, identity, catalog, cart, inventory, order, payment-adapter` | **落地**为 Rust/Axum workspace |
| `AI_IMPLEMENTATION_BLUEPRINT.md` §F Dart packages | `packages/*` + `apps/shop_app` | **落地**为 Flutter monorepo |
| `LIFF_連携・受入設計.md` LIFF-001~012 | `apps/line_web` + `services/identity` LINE 端点 + `docs/05-渠道/line-mini-app.md` | **保留**并接入真实代码路径 |
| `05_非機能要件定義書.md` | `docs/06-非功能需求/README.md` | **保留阈值、删除未测项** |
| `08_セキュリティ設計.md` 威胁表 | `docs/07-安全设计/README.md` + 服务端强制实现 | **升级为可执行**（不再是"未実装"） |
| `reference-models/commerce_openapi.yaml` | `contracts/openapi.yaml` | **补全**为完整可生成契约 |
| 2D/3D/4D/5D 可视化 | 不进入电商产品 | **裁剪**，方法论保留在 `docs/00-方法论/` |

## 2. 证据等级（沿用 0D 门禁）

| 等级 | 含义 | 在本仓库的用法 |
|---|---|---|
| `Observed` | 有可核验运行证据 | 已跑通的 HTTP 接口、migration、测试输出 |
| `Designed` | 有设计文档、无运行证据 | 目标架构、阈值目标 |
| `Hypothetical` | 业务假设 | PSP 行为、LINE 审核结论、物流条件 |

**未运行的验证不得标注 `Observed`。** 每份实现文档都带 `[证据等级: …]` 前缀。

## 3. 0D–5D 方法论保留方式

不作为 UI 存在，而是作为**工程约束**：

- **0D 证据门禁** → 所有设计条目必须带证据等级与来源。
- **1D Prompt** → `docs/00-方法论/prompts/` 下按需求生成实现意图的提示词模板。
- **2D 结构** → `contracts/`（OpenAPI/AsyncAPI/ER）是 2D–5D 的共享底座。
- **3D 空间** → 部署拓扑：`docs/08-部署/`（三层 zone 映射服务）。
- **4D 时间** → Order/Checkout 状态机 + 事件时间线 = 可重放的时序。
- **5D 因果** → Saga 补偿 + 对账（reconciliation）= 正常/异常/恢复路径 + 反事实。

## 4. 官方参考

- IPA 共通フレーム2013：https://www.ipa.go.jp/publish/secbooks20130304.html
- IPA 非機能要求グレード2018：https://www.ipa.go.jp/archive/digital/iot-en-ci/jyouryuu/hikinou/ent03-b.html
- WCAG 2.2：https://www.w3.org/TR/WCAG22/
- LINE MINI App：https://developers.line.biz/en/docs/line-mini-app/discover/introduction/
- RFC 9457（Problem Details）：https://www.rfc-editor.org/rfc/rfc9457.html
- OpenAPI 3.1：https://spec.openapis.org/oas/v3.1.0
