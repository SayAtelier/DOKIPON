# GemArc Unified 5D → Flutter + Full-Stack Microservices · AI 执行提示词

> 版本：5.4。此文件用于向 AI Agent 下达**研发实现任务**；`index.html` 本身只是可以离线运行的动态设计说明书。设计中的服务与数据结构不得被冒充成现有 Feliture 生产系统。

## 0. 项目输入（优先填写）

```yaml
product: Feliture Commerce / Flutter App + Web + LINE MINI App
architecture_design_root: ./
design_entry: ./index.html
required_contracts:
  - ./reference-models/architecture.json
  - ./reference-models/workflow.json
  - ./reference-models/causal_flows.json
  - ./reference-models/causal_cases.json
  - ./reference-models/unified_scene_contract.json
  - ./reference-models/causal_spatial_bindings.json
  - ./reference-models/commerce_openapi.yaml
  - ./reference-models/engineering_cases.json
source_repo_or_folder: "FILL_ME" # 非必填；不提供时只生成 Proposed 架构，不修改真实业务
source_git_ref: auto
implementation_output: ./implementation
implementation_mode: BUILD_TEST_REPORT
language: ja-JP # 设计文档日语，面向开发者的概要可保留中文
supported_locales: [zh-CN, ja-JP, en-US]
platforms: [web, android, ios, line-mini-app]
backend_target:
  language: Rust
  framework: Axum
  db: PostgreSQL
  data_access: SQLx
  broker: NATS_JetStream
  tracing: OpenTelemetry
  local_dev: Docker_Compose
quality_gates:
  logic_branch_coverage: 90
  unit: true
  integration: true
  contract: true
  e2e: true
  security: true
  mobile_device: true
  webview_compatibility: true
  no_live_payment_during_tests: true
```

## 1. 强制目标

作为具有真实文件和终端访问能力的软件工程 Agent，读取本目录中通过 0D 存在性门禁校验的 1D–5D 设计、IPA 风格需求/基本/详细设计、机器可读契约及所有 186 个 Case（144 条规则 Case + 42 条全栈工程设计 Case）。输出可构建、可测试、可审计的 **Flutter + Rust 微服务实现**，并保留所有 Graph Node/Edge、需求、API、Case、测试文件之间的稳定映射。

禁止只输出 Mermaid、UI mock、README、伪代码或框架空壳代替真实实现。代码、数据库迁移、契约测试、部署文件、Flutter Web/Android/iOS、LINE WebView Bridge 均应有实际源文件与验证结果。

## 2. 维度与提示词强制契约

- **0D 是隐式存在性门禁**：只有存在性或证据不足的判定；不制作单独页面，不把 Unknown 当作真实实体。
- **1D 是 Prompt/Intent**：使用站内提示词工作台按需求、BD/DD、Case、证据和阶段生成 AI 规范。Agent 必须读取 `UNIVERSAL_5D_DESIGN_PROMPT.md` 和追踪矩阵，然后生成设计与代码，不得将 1D 当作 Graph。
- **2D 是规范二维工程图**：使用统一 Node/Edge 和 UML/ER/状态/时序语义；不把普通文字卡片当图示。
- **3D/4D/5D 是同一个 Canonical Graph 的静态空间、动态空间及有因果分支的动态空间**；所有对象复用 stable ID 与 graphRevision。
- 正式设计文档只呈现最终状态。用户未确认的业务数据仍然是 `Designed/Hypothetical`；不得声称上线。

## 3. 模型纪律：3D → 4D → 5D

1. **Canonical Graph 唯一**。Node/Edge 必须有独立 ID、类型、方向、证据级别；2D、3D、4D、5D 使用同一 graphRevision，不复制不同来源的业务真相。
2. **3D = Graph + Semantic Spatial Containers**。Building/Floor/Room 是语义空间；Node 是真实模型对象，Edge 是可拾取三维连接对象。不同 Edge 不合并，同层、跨房间、跨层都保持 graphRef。
3. **4D = 同一 3D Scene + Time/Event/State**。事件帧携带 logicalTime、Graph ID、变更属性、状态差异和效果标签。暂停、逐步、回放、seek/undo 都确定性重算；动画 FPS 不影响状态机。
4. **5D = 4D + Causal/Intervention/Counterfactual**。统一 Guards/State Machine 产生正常、失败、边界、恢复和多因子故障分歧；反事实基线与故障路径共享相同场景和 graphRevision，具备结局差异、恢复补偿与未绑定事件警告。
5. **左手坐标系统**：`+X → Right, +Y → Up, +Z → Forward`。World/Camera/Picking/Drag/Pinch/Wheel 必须按约定。若使用 Three.js 右手系，单独配置 LHC→RHC 变换，测试三角面绕序、法线、射线命中、父子 Transform，不允许重复镜像变换。
6. 宝石只出现在工程架构 3D 模型的 Node 上，且应区分 Shape/Color/Size，普通表单/按钮不宝石化。

**拒收条件**：4D 或 5D 只有独立的二维列表、静态渐变或颜色变换；Edge 无法独立选择；动画路径与状态机结果不一致；未绑定的因果规则被隐式连接到随机节点。

## 4. 先契约，后服务，最后 UI

1. 根据 `AI_IMPLEMENTATION_BLUEPRINT.md` 和 API 契约确定 BFF、Identity、Catalog、Cart、Inventory、Order、Payment Adapter；如仓库已有相同能力，按责任边界重用而不重写。
2. 先建立 OpenAPI 3.1、Event Schema、ER、Rust DTO/State Machine、SQL migration/seed、API client codegen 的 CI 门禁。
3. 金额使用 minor units / ISO 4217；订单写请求带 Idempotency-Key；任何支付超时先进入 PaymentUnknown/Reconcile，绝不盲目二次扣款。
4. 数据库按服务明确 owner；交易 Outbox 与订单状态同本地事务；Broker 至少一次投递 + Inbox 去重；事件本身不能构成 exactly-once 保证。
5. 安全：身份令牌由服务端核验，签名、issuer、audience、expiry、nonce、RBAC/ABAC、Webhook 防重放、Secret Manager、审计脱敏。预置假 PSP/LINE adapter，所有高风险操作默认禁止连接生产。
6. Flutter 分为 UI/Domain/Data/Platform Adapter 与 Graph/Scene/Timeline/Causal/Workflow；支持 `zh-CN/ja-JP/en-US`，屏幕与状态文案分离，符合日本语境和认知负荷规则。
7. LINE MINI App 是 WebView + LIFF 的 Web 项目，不是 Android/iOS 包。首次接入、登录取消、外部浏览器、禁用 API、内容审核分别作为 Gate。

## 5. 日本 IPA 文档和验收制品

必须维护：`要件定義書`、`基本設計書`、`詳細設計書`、`非機能要件定義書`、`テスト設計書`、`セキュリティ設計`、`運用設計`、`要求・設計・Case追跡表`。每项 FR/NFR/SEC 通过唯一 ID 关联 Class/UML/ER/Service/API/DB/Graph Node/Edge/Case/UT/IT/ST/UAT。所有文档只写**最终有效状态**，不用迭代过程、临时方案或中间草稿污染交付。

设计目标不等于观察事实。定义的 `Implemented` 必须有代码位置，`Observed` 必须有执行证据，`Designed` 是未实现目标，`Hypothetical` 是模型假设。未获得业务答案的支付规则、库存策略、物流、地域、用户数据删除/保留时间等，必须标记待确认并设置测试 Mock；不能声称可以生产发布。

## 6. 测试与完工判定

- 全部 144 规则 Case 要一致性检查并由真实引擎重新执行；42 个 E2E 设计 Case 必须逐项建立测试代码，所有“未执行”应明确记录。
- 包含 Left-handed orientation/projection/picking、2D↔3D 相同 ID、4D seek、5D Guard 分歧、补偿失败、并发库存、重复消息、支付未知状态、LINE 外部浏览器/取消登录、移动端恢复、观测日志/Trace、备份恢复、回滚。
- 对关键域逻辑、Graph、状态机、契约逻辑统计 Statement 与 Branch Coverage ≥90%，并报告工具与范围；不能将设计数据校验冒充覆盖率。
- 各端编译／测试、容器启动与健康检查、数据库迁移、契约回归、真实浏览器 E2E、Android/iOS 真机、LINE WebView 与平台规则分别生成 Evidence。无工具环境时列为 `NOT_VERIFIED`，不得假报 PASS。
- 必须输出 `README.md`、`QUALITY_REPORT.md`、`ARCHITECTURE_MAP.json`、`RELEASE_GATES.md`，附重现实测的命令和 commit SHA。

## 7. 完成条件

只有在可构建代码、可启动服务、UML/ER/状态与代码一致、自动化测试、跨端/安全/异常验证及部署回滚证据齐全后，才宣布完整工程已实现。在此之前交付状态应为 `Designed` / `Partially Implemented` / `Unverified`，不得用模拟 Case 代替业务验收。
