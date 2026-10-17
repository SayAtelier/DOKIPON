# Universal GemArc 5D Design Atlas v5.4
## 以动态工程设计替代静态文档表达盲区的完整执行提示词

> 交付目标：**直接生成可离线从 `index.html` 打开的 1D–5D 交互式工程设计网站及机器可读设计契约**。本任务的交付物是完整的**动态设计系统**，不是替用户先行开发整个商业 App。工程实现由附属的 AI 全栈执行提示词另行驱动。
>
> GemArc 的 D 是信息表达与因果分析层级，不是物理空间维数；0D 是不占用可视化页面的存在性校验。**所有正式设计文档、配置、Case 和交付报告只写当前有效的最终状态**。

## 0. 顶部输入区——优先填写

```yaml
project:
  name: auto
  local_source_path: ""        # 源码存在时优先本地；为空则从下方来源取得
  source_repository: ""        # Git URL，可选
  source_ref: auto              # 记录实际分支与 commit，不凭空填写
  public_reference_url: "https://feliture.shop/"
  existing_design_path: ""    # 已有架构 JSON、IPA 设计、UML、ER、工作流等
  subject: "Feliture 未来的 Flutter 跨端电商系统"
  output_path: "./gemarc-5d-design-atlas"
  design_site_entry: "index.html"
  default_language: "zh-CN"
  document_language: "ja-JP"
  supported_languages: ["zh-CN", "ja-JP", "en-US"]
  mode: "BUILD_DESIGN_ATLAS_AND_VERIFY"
  target_platforms: ["flutter-web", "android", "ios", "line-mini-app"]
  target_backend: "Rust/Axum microservices (Designed; unless repository proves otherwise)"
  static_offline_entry: true
  evidence_policy: "strict"
  design_scope: ["requirements", "basic_design", "detailed_design", "test_design", "1D", "2D", "3D", "4D", "5D"]
  quality:
    logic_branch_coverage_target: 90
    browser_e2e: true
    traceability_completeness_target: 100
    mobile_viewports: [390, 768, 1024, 1440]
    wcag_target: "2.2-AA"
```

## 1. 身份、权限与最终目的

你是具备真实文件访问、执行、工程逆向、Flutter 设计、Rust API/数据契约、UML/ER、图论、Three.js/WebGL、状态机、因果模拟、IPA 风格设计审查及 UI/UX 能力的自主软件设计 Agent。**先读取真实来源，复用已有数据和组件，然后编写、测试、交付可浏览的动态设计网站。**不得只写 Markdown 提案、静态 Mermaid、截图、动画视频、Flutter 空工程，或给出等待开发的清单。未经授权不得推送远程、修改生产网站或发出真实支付。

动态设计需要回答传统设计书难以直观回答的问题：**关系从何处进入并跨越哪些系统边界；多级依赖为何发生波及；事件如何在时间中排队、并发、竞争与退避；Guard 改变后首次差异出现在哪里；重试、熔断、补偿在三维结构中如何传播和恢复；AI 依据哪条完整证据链生成代码。**不能把页面数量、宝石数量或动画数量作为成功标准。

交付两个清楚分开的对象：① `GemArc Design Atlas`（动态设计阅读器；本次实际开发），② `Target System Specification`（Feliture Flutter/App/LINE/Backend 微服务的设计内容；只表达未来目标）。不得把阅读器代码误当成目标电商系统已经实现的业务架构。

## 2. 维度的不可变定义

| 层级 | 精确定义 | 可见性与交付物 | 严格禁止 |
|---|---|---|---|
| **0D 存在性** | 对某个事实/对象/功能的“存在／不存在／未知”判定和身份校验 | **内部校验门禁**。可在详情中展示证据等级，但不制作独立 0D 图、导航页或装饰动画 | 将 0D 当作“证据图表”、或让 Unknown 偷渡成存在 |
| **1D 提示词** | 以自然语言为主体、附带结构化约束的 AI 执行意图，说明做什么、为什么、依据什么、如何验收 | 可选范围、版本、任务类型的**提示词工作台**；可预览、复制、导出和追踪 | 将 1D 误称为图数据库、单纯字段详情或概念解释 |
| **2D 工程图** | 在平面中保真表达结构与关系 | 规范 UML、ER、BPMN/工作流、DFD、依赖、部署图及可独立点击的 Node–Edge | 只有截图、无箭头语义、节点与 Edge 不可区分 |
| **3D 语义空间** | 2D 的同一个 Canonical Graph 在具有工程意义的三轴、楼层和房间中的**真三维模型** | 左手坐标、三维宝石节点、可拾取 Edge、语义布局/聚焦/剖切 | 伪 3D CSS 效果冒充 GPU 真三维；为装饰添加无关房间 |
| **4D 动态 3D** | **同一个 3D 场景**叠加逻辑时间、状态、事件与实际传播轨迹 | Play/Pause/Seek/Step、流向、高亮、等待/竞争/超时/恢复 | 将时间线画在别的独立二维面板并冒充 4D 本体 |
| **5D 因果 4D** | **同一个 4D 的规则模型**支持干预、平行世界、首次差异、传播与结果归因 | 基线/干预、复合异常、反事实、A/B 同步 3D、差异解释 | 预制彩色分叉动画；无证据就宣称真实因果 |

进阶关系是 **0D Gate → 1D Intent → 2D Semantic Graph → 3D Semantic Scene → 4D Dynamic Scene → 5D Causal Dynamic Scene**。任意阶段可以选择架构 View，但不能凭切换维度篡改业务事实。Dimension 与 View 正交；Logical/Deployment/Security/Data 等是工程关注角度，不是 2D/3D 的同义词。

## 3. 真实数据和可信度

先按源码、本地文件、仓库、公开网站、设计文档、执行轨迹分别检查。记录项目身份、分支、commit、提取时间、文件位置、行号、数据契约版本。多来源存在差异时生成冲突项，禁止直接融合不同项目。

每个 Node、Edge、Guard、Case、服务与接口都有 `id`, `source/evidenceIds`, `verification` 和 `graphRevision`。等级：Implemented（源码存在）、Observed（真正观测）、Designed（目标设计）、Inferred（依据不完整）、Hypothetical（模拟假设）、Unknown（无充分证据）。**Implemented ≠ Observed；Designed ≠ Implemented。**对不能访问的 Feliture 私有微服务、PSP、库存系统，默认只能作为 Designed/Hypothetical，不得使用营销页面反推出数据库表或架构事实。

保持需求 → BD → DD → Node/Edge/Port → API/Event/Data → Guard → Scenario → UT/IT/ST/UAT 的双向稳定索引；跨版重命名使用 ID 映射表，不得每次随机生成新的实体 ID。没有可用证据的 4D/5D 应明确标注模拟，不要伪造真实运行数据。

## 4. 1D 提示词契约——以要求驱动全部维度

每一条提示词必须包含 `promptId`, `scopeIds`, `sourceRevision`, `objective`, `constraints`, `allowedAssumptions`, `requiredArtifacts`, `diagramTargets`, `simulationTargets`, `acceptanceTests`, `unresolvedIssues`, `outputLanguage`。至少提供 **要件定义、基本设计、详细设计、因果 Case、工程实现、设计审查**六类可选意图，绑定真实需求 ID 和 Case ID；修改选择即重新生成文本。保留人工编辑版本和机器生成版本的差异、作者、来源与审查状态（非编译器级执行语义）；复制/导出独立完成。

提示词必须给 AI 明确角色、输入、不可越界的事实、目标契约、输出格式、检查方式、停止条件及未确认事项。1D 工作台不是传统文档替代品，而是生成和核查 2D–5D 的机器可审计**控制入口**；Graph Store 从属于 2D–5D 的统一底座，而不再被归类为 1D。

## 5. 2D：规范图与图论精确交互

以 Canonical Graph 的 Node/Edge 为一等对象，支持 directed/undirected/multigraph/self-loop/conditional/typed/port edge。Graphology（或在离线便携版提供语义等价、经测试的轻量实现）管理拓扑，Dagre 做有向布局；UML/ER/BPMN 符号和基数必须保真。点击 Node/Edge 独立查看 ID、类型、出入关系、端口、守卫、Evidence、设计文档、异常 Case 与影响链。

提供 BFS/DFS、Neighborhood、Ancestors/Descendants、Shortest/All Paths、Cycle/SCC、Impact/Dependency 查询，并设置深度、数量与时间预算。所有结果在当前画布定位，不能只输出 JSON；大量节点先聚合/过滤，再逐级展开。切换图种必须保持 stable ID、选择与来源。

## 6. 3D：真正有语义的图论世界

一张完整三维图须保留 `Graph Node ID`、`Graph Edge ID`、`Room/Floor/Building` 的语义来源。楼宇、楼层、房间可作为虚拟阅读容器，但须标注不是物理部署事实。三轴代表**有工程依据且相互补充**的分类；为空时折叠，不得随机投射人工制造意义。

左手坐标约定 `+X=Right, +Y=Up, +Z=Forward`，camera 初始从负 Z 看向正 Z。统一 World、Camera、Ray Picking、Drag、Orbit、Zoom、父子变换与边线路由的左右手转换策略；Three.js 原生右手系需在独立适配层转换并验证法线、绕序、碰撞/拾取，禁止双重镜像。

Node 使用统一 GemGeometryFactory：形状编码功能角色、基础颜色编码领域、外环/符号编码状态、尺寸只表达有出处的量。宝石不得用在按钮、导航、普通卡片、时间轴或背景。Edge 保留独立几何、箭头、类型、条件、端口、跨房间/楼层连接和足够的屏幕拾取容差；平行边、自环不得合并。聚焦时按**图关联/事件关联**计算相关度，非相关对象透明退避/折叠，快速切换可取消并可完整恢复。

3D 视图包括 Overview、Exploded Floors、Cutaway Room、Node/Edge Focus、跨层 Path Highlight、Trust Boundary 和 Deployment Context。只为多维信息解释增益而使用 3D；图示需要准确基数与文字密度时优先提供 2D 正式图。

## 7. 4D：状态机决定的动态三维

独立逻辑引擎维护 `logicalTime`, `sequence`, `queue`, `currentStates`, `snapshots`, `events`；同一三维 Node/Edge 接收事件帧并显示状态环/传播方向/延迟/重试/阻塞/资源竞争。所有动画通过 Trace 投射到三维场景，FPS、窗口大小、颜色和相机不允许改变事件结果。

明确同步调用/异步消息/并行/等待/计时器/超时/重试/死信/恢复/断网；事件重放支持 Step、Seek、Snapshot、Reverse Seek（从最近快照确定性重算）。相互独立的并发事件需明确排序策略与随机种子；无事件不播放装饰性流光。记录每个 Event ↔ Graph Node/Edge ↔ Rule ↔ Evidence 的绑定；无法绑定则显示 Unbound，不得自造连接。

## 8. 5D：可复验的多因素因果四维

为 Case 定义 `initialState`, `modelRevision`, `seed`, `interventions`, `guards`, `branchQueue`, `expectedTerminal`, `invariants`, `evidence`。A/B 从相同初始快照分支，按干预独立重算，差异从实际 Trace 推导。至少覆盖：成功、输入无效、边界上下限、网络延迟、超时、取消、重复请求、消息重投、乱序、服务宕机、竞态、锁冲突、资源耗尽、重试上限、熔断、部分成功、补偿失败、人工干预、最终一致性、恢复、无差异干预、多因素组合。

最关键的 UI 操作序列：选 baseline → 选 t* → 调整参数或禁用规则 → 生成 B 世界 → 启动计算 → 同步三维重放 → 标记首次**实际**分歧 → 展示传播边、被阻断边和受影响组件 → 比较终态/时延/成本/资源 → 点击任意分歧解释条件和证据。可切换 Overlay / Split 3D / Branch Timeline；时间轴只是三维世界的辅助控制，不得代替其表现。

因果证据强度分为 Dependency、Temporal、Correlation、Model-defined Causation、Empirically Validated Causation。相同终态不表示没有过程差异；没有差异必须显示“当前干预未引起可观测差异”。对无限循环设预算并解释终止原因。Case 数量由**有效 Guard 覆盖与状态空间**决定，禁止机械复制换名凑数量。

## 9. 应比传统设计文档更清楚的六类瞬间

1. **关系穿透**：一个请求如何跨 UI、BFF、业务服务、数据库和外部 PSP，哪些是真实或仅 Proposed。
2. **竞争与争用**：两个并发订单争抢同一库存版本，前后顺序和失败位置在三维图中的移动轨迹。
3. **一致性窗口**：支付结果未知、订单尚未落库时，哪些组件在等待，消息重投为何不重复扣款。
4. **故障边界**：断网、服务熔断或 LINE 登录取消，影响停在哪条 Edge，哪些节点被隔离。
5. **补偿逆流**：正向交易和 Saga 补偿按不同线路运行，失败的补偿具备悬而未决状态。
6. **反事实解释**：同一基线中只改一个条件，前一段共享路径与第一处差异在同一 3D 场景直接可见。

每类必须有读者可以触发的真实交互、可追踪数据和退出/恢复操作。不能只配一张“精美”静态图或一段文字描述。

## 10. Flutter／LINE／后端微服务作为设计对象

画出 Flutter UI/Domain/Data/Platform Adapter 共享层、Web/Android/iOS 宿主差异、LINE MINI App 的 Flutter Web + LIFF 桥接约束及安全许可边界；不把 LINE 称为原生 Flutter 二进制。后端设计可选择 BFF + Identity/Catalog/Cart/Inventory/Order/Payment Adapter 等，但**仅在业务边界成立时才拆微服务**，不要为“微服务数量”而拆分；逐个记录 Owner、数据主权、API、Event、存储一致性和熔断条件。

明确 OpenAPI 3.1、事件 Schema、PostgreSQL migration/ER、idempotency、outbox/inbox、支付未知终态对账、Saga、超时预算、安全日志脱敏、LINE Server-side token validation、权限模型、监控/回滚。前后端及 App 的 AI 实现提示词必须从相同追踪模型导出，但本 Atlas **只交付这些设计与实现约束**，不宣称产品工程已上线。

## 11. 日本 IPA 设计深度

按参考结构维护 `要件定義書`, `基本設計書`, `詳細設計書`, `非機能要件定義書`, `UI/UX設計書`, `インターフェース設計書`, `データ設計書`, `テスト設計書`, `セキュリティ設計書`, `運用設計書`, `要求追跡マトリクス`。以 IPA《共通フレーム2013》组织阶段与职责，用《非機能要求グレード2018》梳理可用性、性能、运维、迁移、安全等非功能要求。不得称为 IPA 官方“认证图”或强制固定目录。

要求每条需求关联 BD/DD、2D Diagram、Graph Node/Edge、3D Space、4D Event、5D Guard/Branch、UT/IT/ST/UAT，并明确任何缺口。日本市场文案遵循本地化、相邻信息不重复、状态提示符合语境；所有设计/配置/清单只写最终有效状态。

## 12. 视觉、色彩心理学与认知负荷——强制验收

设计语言为**冷静的工程编辑台**，非炫光科幻宣传站：背景低饱和海军蓝/石墨、阅读卡片略提亮；强调色青绿仅用于交互及聚焦，蓝色用于中性信息与选中关系；琥珀提示注意，珊瑚红用于故障/危险，灰色表示不可用/未知。颜色还须配图标、文字、线型，**不能单靠颜色编码**。角色宝石的 Domain Color 与运行状态的外环颜色分离，以免一个视觉通道承载冲突意义。

提供 Color Token、Typography Token、Spacing Scale（建议 4/8 为基础）、Radius、Elevation、Motion Token、Dark/Light 可访问配色；文字默认 `14–16px`，主内容正文行高 1.5–1.75。WCAG 2.2 AA：正文目标对比度≥4.5:1，重要非文本 UI 与大字号相关目标≥3:1；控制焦点、键盘可达、200% 文字缩放、Reduced Motion。避免 `8px` 的微字承载关键信息。动画强调“状态改变”而非装饰，用户可以暂停、调速、逐帧、关闭；可预测过渡约 120–240ms，超过阈值时停用无意义动效。

默认视图优先显示高价值的约 6–18 个节点，并提供全图/聚合/分层检索；长期阅读时侧边导航稳定、中央内容为唯一视觉焦点、右侧按需展开语义详情，底部时间轴仅在 4D/5D 出现。所有面板必须处理长中文/日文/英文、字体放大、屏幕缩放、Windows 滚动条、移动端单栏和输入法；禁止超框、控件遮挡、标签漂浮、密集无意义图例、过量 Bloom/透明材质。浏览画面同一时刻只允许**一个主要视觉焦点**。

色彩心理学只作为认知一致性设计假设，不得声称特定颜色能够普遍产生确定情绪反应。使用真实任务测试“找到故障路径需要几步、Edge 是否易选、读懂状态需要多久”，根据可访问性与可用性测量优化，而不是纯主观评比。

## 13. 技术分层与浏览器安全

底层：Evidence Registry + Canonical Graph + Diagram Semantics + 1D Intent Registry + Workflow Semantics；计算：Graph Query + Discrete Event Engine + Causal Branching + OutcomeDiff；投影：2D SVG/Canvas、3D WebGL/Three.js + LHC Adapter；持久化：版本化数据、JSON 导入导出、Dexie/IndexedDB（部署版）。阅读器 `index.html` 便携版必须离线自包含，若浏览器的 `file://` 安全策略阻止 IndexedDB/WebGL，必须明确提示并提供经过测试的非冒充降级表示；核心数据可只读内嵌。禁止 CDN 是唯一依赖、任意执行 YAML/Guard JS、跨来源运行不可信代码。

所有 View、Scene、动画和插件接收同一 Graph Revision。几何/颜色/相机/画布坐标只影响显示，**不得参与业务 Guard**。给出所有公开 Schema 的验证、版本、迁移、反序列化防护、图查询预算和无限画布增量保存策略。

## 14. AI Native 工作流与格式互操作

独立 Workflow Canonical Schema：`workflowId`, `nodes`, `ports`, `edges`, `guards`, `resources`, `timeouts`, `retries`, `compensations`, `provenance`, `presentation`。语义与无限画布坐标分离，明确子流程、并行 Fork/Join、异常 Edge、端口类型、安全作用域；变化由图验证器重新核验。提供 JSON 导入/导出、映射报告、BPMN 等选定格式的可支持子集，并逐项报告不可逆语义损失。AI 可按 Schema 精确检查和建议修改；没有执行权限时仅产生可审阅 Patch，不直接调用生产系统。

## 15. 可审查 Case 体系

不要依赖“144 / 186”作为通用目标。为每个目标服务及关键 Edge 推导如下矩阵：正常路径 × 单点异常 × 边界上下限 × 并发 × 顺序变化 × 恢复 × 补偿失败 × 安全失败 × 跨端能力差异 × 组合异常。每个 Case 必须有 ID、初态、输入、Guard 命中、Trace、节点/边绑定、可观测状态、预期终态、假设、测量与关联测试。每个分支对应唯一规则，不得用预写故事板假装模型输出。以 Guard/Transition 的实际覆盖报告补足数量。

## 16. 实施顺序、质量门禁与最终交付

**实际工作顺序：** 来源核验与 0D Gate → 1D 提示词注册 → Canonical Graph → 2D 规范语义/布局 → 3D 世界与可拾取 Edge → 4D 离散事件 → 5D 反事实分歧 → Design Atlas 整合 → 日本 IPA 文档索引/RTM → E2E/UI/视觉/性能/可访问性测试 → 离线打包。

验收应覆盖：
- 0D 不出现独立页面；1D 生成的提示词确实随需求/阶段改变且引用稳定 ID；2D 支持 Node 和 Edge 的独立点击、UML/ER 语义保真。
- 3D 的 Node/Edge 对应统一 Graph ID，左手坐标轴、相机和拾取正确；楼层切换与 Focus Retreat 可逆；4D/5D **复用同一场景**，不复制架构事实。
- 4D 逻辑时间、并发顺序、seek/重放和动画路径与 Trace 一致，FPS 独立；5D baseline 与干预可重复，含异常、补偿、无差异及多故障组合；其首次差异由比较器计算。
- 导航、搜索、键盘、屏幕阅读、移动端宽度、长日文、文字缩放、色彩对比和 `prefers-reduced-motion`；WebGL 不支持时不能伪称真三维通过。
- 设计追踪完整、证据等级正确、Case 覆盖有效、不会访问生产业务、所有自动化测试及浏览器截图来源可复现；关键逻辑分支覆盖率以**实际测试**测量，目标≥90%。

交付 `index.html`、可维护源文件、架构/图/事件/因果/工作流 JSON、1D 提示词集合、Case 库、日文 IPA 风格文档、要求追踪矩阵、使用说明与最终验收报告。**最终报告必须将“已实际验证”与“设计目标 / 尚未验证”分开；不得将旧版测试或模拟场景算作 Flutter/后端工程测试。**

## 17. 自检拒收条件

如果发现以下任一问题，不能宣布完整：0D 仍占据独立图示页面；1D 仍被定义为图数据库；2D 工程关系语义不完整；3D 只有房间装饰或没有独立拾取的 Edge；4D/5D 使用不同虚构图；5D 的 A/B 仅改变颜色却没有重新计算；没有异常/恢复或无差异 Case；缺失稳定 ID、接口/业务规则追踪；成人类别 LINE 业务未经合规确认即被标记可发布；UI 文字难以阅读；离线 index 因依赖 CDN 空白；测试未执行而写通过。

**执行命令：立即读取给定来源并构建、测试、交付这一套动态设计网站。设计的最大价值是解释复杂系统的交互与因果，不能用宏大展示页掩盖具体的工程细节。**
