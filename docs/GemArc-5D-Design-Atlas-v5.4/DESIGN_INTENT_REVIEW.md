# 附件 v3.0 提示词的意图分析与最终设计判断

## 1. 核心目标

附件强调五件事：**源码证据是真相、正式工程图必须语义保真、统一 3D 宝石是节点视觉语法、4D/5D 必须来自计算、最终交付必须实际可交互且经验证**。最关键的隐含诉求不是增加图数量，而是让用户观察“原本埋在接口、状态机和分散的详细设计书里的互动”。因此本项目应当优先表达传统纸面文档不易表达的结构穿透、状态变化、交错执行、异常传播与因果比较。

## 2. 六项必须修正的概念或矛盾

1. **0D 定义错位。** 附件把“存在性”写成实体清单与范围展示，上一版又把它混入图谱导航；但 0D 只有“是否存在／是否确认”，属于事实门禁，不应强行占据一维可视化页面。
2. **1D 定义错位。** 附件 v3.0 把 1D 说成“语义信息”，上一版实现中甚至用 1D 指代 Node–Edge 图。按现在的明确要求，1D 必须是 **Prompt/Intent**，且内容可根据需求 ID、阶段和验收自动组织，不能只是一页说明文字。
3. **4D/5D 表达链易被切断。** 虽然原文承诺 4D/5D 有规则模型，但渲染分层图和界面组织仍可能诱导 Agent 独立制作时间线和因果面板。应强制 3D 为唯一空间舞台，4D 是这个空间的时间演化，5D 是多个可比较的 4D 运行分支。
4. **真正需要的不是页面数量。** 原文罗列大量 View 和图种，很容易生成覆盖面广但实际浅的静态画廊。应将验收改成特定设计问题可通过交互回答，例如支付响应丢失引发的补偿链和库存并发。
5. **“开发产品”与“开发设计阅读器”被混淆。** 附件命令直接写 HTML 架构工作台，而历史需求还要求 Flutter 未来完整 App。必须区分本次实际构建的动态设计站与未来 App/微服务的目标设计，避免向用户交付第二个工程框架代替设计内容。
6. **视觉设计规范缺少可量测约束。** 附件已有“极简、未来科技”和宝石切型，却没有充足的视觉 Token、层级权重、色彩对比、动效预算、焦点聚焦与多语言溢出验收，因此容易变成高反光、高噪声的技术演示。

## 3. 精修的体系结论

- `0D`: invisible existence/identity gate; implemented / observed / designed / hypothetical labels are graph provenance, **not another diagram**.
- `1D`: prompted, scoped and traceable AI instruction, which names the requirement, constraints, evidence, acceptance, outputs.
- `2D`: rigorously semantic diagram and property graph; node and edge independently selectable.
- `3D`: LHC spatial graph with meaningful building/floor/room grouping; gems only for diagram nodes.
- `4D`: the very same 3D objects animated according to deterministic events, states and logical time.
- `5D`: branchable 4D worlds with guarded interventions, counterfactual diff, first divergence and recoveries.

**维度增加的不是炫光数量，而是可回答的问题深度。** 每一层都必须说明它相比上一层带来哪个以前无法精确回答的设计问题，不能为了展示 5D 而添加无关空间轴或因果故事。

## 4. UI 方向与易用性

本设计采用克制的石墨/深蓝背景与低饱和卡片；青绿色只代表交互与聚焦，蓝色表示信息和关系，琥珀/珊瑚分别表示警告/故障，灰色表示未知与背景。状态必须同时有文字、符号或线型，不以颜色为唯一载体。移除主工作台的大面积装饰宝石、强发光、持续旋转；只在真正的三维工程节点使用宝石几何。

默认显露任务路径而非工具清单：**提示词 → 工程图 → 语义空间 → 时间回放 → 因果实验**。读者点击某个 Edge 后，其它信息退避；按 Esc/返回恢复。屏幕窄时折叠侧边栏、保留设计主画布、让编辑与检查按需展开。中文、日文、英文文本应分别做真实长度和 200% 字号检查。

## 5. 可运行与不可断言的边界

现有 Atlas 是单文件离线 **设计阅读器**，不能将其模型 Case 验证等同于 Flutter、Rust、PSP、LINE 的生产执行结果。真实后端源码缺失时，只能提供追踪充分的 Designed/Hypothetical。WebGL 不可用时，应明确回退到交互式三维**投影/二维示意**，不声称 GPU 路径已实测。审查报告只报告已运行的检查和实际发现。

最终设计与执行提示词见 `UNIVERSAL_5D_DESIGN_PROMPT.md`。已有 `AI_ENGINEERING_EXECUTION_PROMPT.md` 是**将这些已审核设计转成代码**的另一个阶段，不能和当前 Design Atlas 的产物范围混为一谈。
