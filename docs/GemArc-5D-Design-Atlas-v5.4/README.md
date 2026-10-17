# GemArc 5D Design Atlas v5.4 — 动态工程设计说明书

**交付对象：** Feliture 未来 Flutter Web / Android / iOS / LINE MINI App + 后端微服务的**1D–5D 动态设计体系**。本包不是 Flutter App 的源码工程，也不是已经验证的生产后台架构。

## 启动

直接解压并双击根目录 `index.html`。页面内嵌 CSS、JS、设计数据、提示词生成器、41 份设计文档；无需 CDN、Flutter SDK 或 localhost。开启 JavaScript；如果浏览器不支持 WebGL，阅读器会使用**明确标记的三维投影降级模式**，不能把它当 GPU 真三维测试通过。

## 维度的精确含义

- **0D**：仅做存在 / 不存在 / 未知的后台证据门禁，**没有单独的可视化页面**。
- **1D**：提示词（Prompt）。选需求、设计阶段、因果流程和证据严格度，自动生成可复制/导出的工程执行意图，附稳定 FR/BD/DD/Case ID。
- **2D**：可交互 Graph Node/Edge 与标准 UML/ER/时序图。图数据库是 2D–5D 的共享底座，不是 1D。
- **3D**：有语义的 Building/Floor/Room、真实 Graph ID、可拾取 Edge 和统一左手坐标 +X Right / +Y Up / +Z Forward。
- **4D**：同一 3D Scene 的逻辑时间、状态机事件、传播、暂停和重放。
- **5D**：有条件干预与基线反事实比较的 4D，可解释正常/异常/恢复、首次差异与传播。

## 核心内容

| 内容 | 当前设计数据 |
|---|---:|
| 功能需求 | 13 条 |
| Canonical Graph Node / Edge | 19 / 26 |
| 状态机/Flow | 16 |
| Guarded Model Case | 144 |
| 全栈 E2E 设计 Case | 42（**仅设计，未对微服务执行**） |
| 内嵌设计文档 | 41 |
| Excel | IPA 跟踪表 + 0D–5D 定义 + UI UX 审查页 |

## 重点交付

- `index.html` — 唯一离线启动入口：1D 提示词、2D 规范工程图、3D 空间、4D 时间、5D 因果、IPA 需求/基本/详细设计和跟踪矩阵。
- `UNIVERSAL_5D_DESIGN_PROMPT.md` — 以用户本次指定维度重新写就的**完整版通用提示词**，供以后换仓库、换产品复用。
- `DESIGN_INTENT_REVIEW.md` — 对附件 v3.0 的目的、错位、易混淆概念和视觉风险进行审查。
- `UIUX_DESIGN_SPEC.md` — 高精度视觉 Token、认知负荷、可访问性和交互验收要求。
- `AI_ENGINEERING_EXECUTION_PROMPT.md` — 独立阶段：AI 根据设计实施 Flutter + Rust/Axum 后端/微服务；不是当前 Atlas 的完成证明。
- `IPA_要件・設計・Case追跡.xlsx` — 原追踪表及新增 Dimension/UX 页。
- `reference-models/` — 有稳定 ID 的架构、Workflow、Causal Flow、Case、OpenAPI、LHC Scene Contract。
- `tests/` 与 `QA_REPORT.md` — 已运行测试及尚未验收的事项。

## 事实边界

Feliture 网站可公开观察的体验不等于源码或生产微服务结构。Flutter、LINE、Rust、PostgreSQL、PSP、库存 Saga 等为目标设计（Designed）或带依据的条件模拟（Hypothetical）。LINE MINI App 是经 LIFF 接入的 Web 应用，商品和内容必须满足其当前政策；本包不声明通过 LINE 平台审核。

## 官方参考

- IPA 共通フレーム2013：https://www.ipa.go.jp/publish/secbooks20130304.html
- IPA 非機能要求グレード2018：https://www.ipa.go.jp/archive/digital/iot-en-ci/jyouryuu/hikinou/ent03-b.html
- WCAG 2.2：https://www.w3.org/TR/wcag/
- LINE MINI App：https://developers.line.biz/en/docs/line-mini-app/discover/introduction/

**正式设计、清单、配置、追踪表只表达当前最终有效状态。**
