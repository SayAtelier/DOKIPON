# GemArc 5D Design Atlas — DOKIPON Commerce

**工程样机（Scoped implementation）**：提供真实可交互的 1D–5D 纵向链路，不宣称满足输入 v8.2.0 提示词的所有生产级质量门禁。

## 打开方式

双击根目录 `index.html`。它内嵌样式、模型和逻辑，不依赖 CDN 或网络请求。静态部署版本是 `dist/index.html`。不同浏览器的 `file://` 限制可能导致环境差异；离线功能以本地代码和数据为基础。

Python HTTP 静态服务（可选）：

```sh
python -m http.server 8080
```

打开 `http://localhost:8080/index.html`。

## 核心操作

| 维度 | 用户操作 | 计算/展示 |
|---|---|---|
| 0D | 无独立页面 | 事实存在性在模型中管理 |
| 1D | 左侧选择需求，编辑、复制或导出 Prompt | 需求和 Node ID 与源文件关联 |
| 2D | 缩放/拖动画布，点击 Node/Edge，打开工程图 | 12 个节点和 19 条关系；专业图可独立审查 |
| 3D | 拖动旋转、滚轮缩放，切换 View，点击聚焦 | 语义楼宇、楼层和房间；不相关内容退避 |
| 4D | 切换 Case、播放/暂停/定位时间 | 确定性事件序列驱动模型状态和连线 |
| 5D | 禁用模块或 Edge，插入 Validator/Cache/Proxy | 图差分、Port 校验、A/B 重算、首次差异 |

`4D` / `5D` 不访问实际生产 API。模型动态是可重复计算的**设计实验**，不是已采集的业务调用轨迹。

## 全维度语义名称与多视角适配

- **Node 与 Edge 都有完整语义名称和稳定 ID**。2D 画布全部 19 条 Edge 具有独立名称、可点击的关系标签；3D 场景在 GPU 和 CPU 降级渲染上方使用同一套屏幕空间命名图层。
- **三种命名模式**：`智能避让`（默认，优先显示焦点对象并消除标签冲突）、`全部名称`（查看视野内所有可见 Node / Edge 名称，可能拥挤）、`仅相关名称`（不显示与当前选中对象无关的对象名）。
- **多角度适配**：标签使用相机投影定位，不随宝石、楼层、关系线扭曲；缩放和 Orbit 时保持屏幕正向；标签紧随 Node、Edge、Building、Floor 的实际锚点。语义楼层仅是展示分组，不是伪造的正式 Graph Node。
- **完整名称可查**：标签支持点击、Tooltip；左侧单独列出并可搜索所有 Node / Edge，检查栏显示完整名称、ID、Source/Target 与证据。
- **4D/5D 联动**：当前事件关系名称高亮，因果差异标签具有独立状态；实验节点及被重写的 A/B Edge 直接生成可区别的标注，清空干预即清除实验标签。
- **自审边界**：为了可读性，智能模式可能临时隐藏拥挤或视野外标签；`全部名称` 可查看在视野内的节点与关系名称。不可用场景的 GPU 深度遮挡判定、完整自动避障和所有 WCAG 条目尚未通过严格验收。

## 来源与范围

- 目标仓库：<https://github.com/SayAtelier/DOKIPON>，固定读取版本 `1756173c04ddb11306a357927d05b1cc4ceac872`。
- 参考仓库：<https://github.com/UlyssesLeoLee/GemArc>，仅参考设计理念。
- 参考网站：<https://feliture.shop/>，仅作为公开电商体验参照，尚无证据证明与 DOKIPON 是同一个生产系统。
- 可确认：Flutter 应用入口、Rust Commerce 工作区/服务入口、OpenAPI 文件存在。
- 待确认：生产依赖拓扑、真实数据库 Schema、支付回调时序、微服务运行状态。

## 文件结构

- `index.html` / `dist/index.html`：离线便携页面和静态入口。
- `source/template.html`、`style.css`、`viewer.js`、`engine.js`、`labels.js`：可维护页面、语义标签布局、确定性模拟引擎。
- `models/`：Canonical Graph、来源与证据、Workflow、Case、交互模板和追踪矩阵。
- `docs/ja-JP/`：日语版要件定义、基本设计、详细设计和测试设计。
- `docs/AI_IMPLEMENTATION_HANDOFF.md`：后续实现工程契约。
- `QA_FINAL.md`：真实执行结果与未完成项。
- `MASTER_PROMPT_v8_2_adaptive_labels.yaml`：完整可粘贴提示词，包含顶端 GitHub 参考字段及跨维度命名与多角度适配的强制契约。

## 测试

```sh
node --test tests/*.test.cjs
python tests/label_browser_test.py
python tests/browser_test.py
python tests/release_checks.py
```

此环境的 Chromium 对本地 `file://` 和 `http://127.0.0.1` 页面导航施加管理限制；浏览器交互测试使用直接注入完整 HTML 的方式验证，其结果不能替代 `file://` 真实端到端验收。详见质量报告。

## 范围外 / 不可宣称

本版未集成 Graphology、Dagre、Three.js、Dexie.js；空间实现采用无依赖 WebGL 渲染器和 Canvas 降级投影，语义图采用稳定 ID JSON。ER/UML 当前为受限审查图模板，不是从源码全面自动解析的规范图。不能将此样机当作已完成日本 IPA 认证或正式 WCAG 2.2 AA 审计的产品。
