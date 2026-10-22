# DOKIPON 运营后台 — HTML 高保真原型（电商口径）

`tools/gm-console/frontend`（React 18 + Vite + TS，9 个页面组件 2 526 行，前端共 14 个源文件 3 190 行）
与 `crates/gm-backend`（Actix-web，32 条路由） 的静态等价物。
**双击 `index.html` 即可打开**，无需构建、无需起服务、无网络请求。

> 这套原型原本服务于游戏服务器后台（GM Console）。项目改造为电商交易系统后，
> **界面文案与演示数据已全面电商化**；**接口字段名、状态枚举与端点路径一律保持原样**——
> 逐条对照见下文「原型界面电商化 vs 后端字段名沿用游戏域」与 `index.html` 的
> **「游戏服务器 → 电商 改造对照表」**。

## 页面

| 文件 | 屏 | 主接口 |
|---|---|---|
| `index.html` | 原型索引（路由覆盖矩阵 + 契约漂移矩阵 + 走查问题 + 电商化对照表） | — |
| `login.html` | 运营后台登录 / 角色映射 | `POST /gm/login` |
| `dashboard.html` | 经营总览 / 实时图表 / 公告下发 | `GET /gm/summary`、`/gm/metrics`、`/gm/events`(SSE) |
| `players.html` | 消费者筛选、排序、分页、详情 | `GET /gm/players` |
| `servers.html` | 服务实例状态与启停 | `GET /gm/servers`、`POST …/start|stop` |
| `items.html` | 补偿与兑换码发放及发放历史 | `POST /gm/items/grant`、`GET /gm/items/grants` |
| `canvas.html` | 营销素材下发（客户端叠加图层） | `POST /gm/canvas/send`、`GET /gm/canvas/anchors` |
| `mall.html` | 店铺商品增删改（行内编辑） | `GET/POST /gm/mall/items`、`PUT/DELETE …/{id}` |
| `reports.html` | 经营报表多维筛选与 JSON 明细 | `GET /gm/reports` |
| `support.html` | 售后工单建单与状态流转 | `GET/POST /gm/support`、`PATCH …/tickets/{id}` |

文件名镜像后端端点族（`/gm/players`、`/gm/mall/items` …），**不随界面改名**；
「文件 ↔ 屏 ↔ 后端端点」的对应关系见 `index.html` 的对照表。

## 保真约定

1. **字段名与状态语义不改写**。列名、取值、状态标签色值、`invalid_credentials` 等文案分支均取自源码；
   界面上的中文只是这些契约字段的「界面标签 + 字段名」并列（如 `Player ID（消费者 ID）`）。
2. **视觉取自 `App.css`**。浅色底 + 靛蓝强调 + 6 种状态标签，配色值与真实构建一致。
3. **校验顺序照搬**。例如 `补偿发放` 页 `Player ID / Item ID` 为空时走 warning 分支且不发请求；
   `经营报表` 页按 `type → category → reportType` 回退取类型。
4. **后端语义对齐的量值**：高消费客户阈值 `total_spent > 1000`（`players_handler.rs`）、
   `online_connections` = 各服务实例 `online_players` 之和（`servers_handler.rs::metrics`）、
   启动服务实例后承载会话数落在 50..500（`start_server`）、
   `/gm/metrics` body 只有 `online_connections` / `running_servers` / `total_servers` 三行，
   无 `# HELP` / `# TYPE`。
5. **数据是演示数据**。`assets/data.js` 全部内置，时间戳相对页面加载生成；任何写操作只改本页状态。

> ⚠️ **本原型的字段命名取自「前端期望」。**
> 2026-10-22 审核发现：后端用 snake_case 序列化、前端按 camelCase 读，两者之间没有 serde rename 规则，
> 共 13 处契约不一致——接线后 消费者 / 补偿发放 / 售后工单 / 经营报表 会整块空白，营销素材下发会必然 400。
> **这 13 处已全部按「后端对齐前端」修复**，逐条对照见 `index.html` 的**契约漂移矩阵**，
> 并由 `crates/gm-backend/tests/it_frontend_contract.rs`（CT1–CT9）锁定契约。
> 原型按前端期望渲染，是因为它是一份**设计原型**（供评审看屏），不是联调测试。

## 原型界面电商化 vs 后端字段名沿用游戏域

本轮改造（游戏服务器 → 电商）**只动了两类东西**：

| 动的东西 | 例子 |
|---|---|
| **界面文案**（标题、标签、说明、导航、横幅、空态） | `Players → 消费者`；`Guild → 会员分组`；`Online Players → 承载会话数`；`Ticket → 售后工单` |
| **演示数据值**（姓名、客群、补偿项 ID、商品名、公告与诉求内容） | 玩家名改为真实电商顾客（中日英混合）；`item_gem_600 → item_voucher_500`；`Starter Bundle → 咖啡豆礼盒` |

**没有动的东西（属已知遗留，不是缺陷）：**

- **接口字段名**：`playerId` / `vipLevel` / `guild` / `mall` / `totalSpend` / `itemId` / `onlinePlayers` 等
  全部保持原样——它们是 `crates/gm-backend` 的真实契约，前端按 camelCase 读、后端按 camelCase 序列化。
  界面上的中文只是标签，详情网格里仍显示字段名（如 `Player ID（消费者 ID）`）。
- **状态枚举**：`online` / `offline` / `banned`、`running` / `stopped`、`open` / `pending` / `resolved`。
  这些值既是 API 取值也是 CSS 类名来源（`status-online` 等），因此标签原样显示，
  界面语义由筛选下拉、统计 chip 与说明文字给出。
- **端点路径与文件名**：`/gm/players`、`/gm/mall/items`、`players.html` … 一律不动。
- **原型数据结构**：`assets/data.js` 的键名（`players` / `vipLevel` / `guild` / `totalSpend` …）、
  `playerStats()` 统计逻辑、`metricsText()` 的三行格式全部未动。

因此：**契约漂移本身已修复**（13 处全部 `已修复`），但**词汇层面的遗留仍在**——
后端与前端都还叫 `playerId` / `guild` / `mall`。把这批字段名改成电商词汇属于后端改造，
不在原型范围内；改名前原型会与真实契约脱节，故保留原样并在此标注。

## 原型新增的部分（真实构建中没有）

- 二次确认弹窗（停服务实例 / 发补偿项 / 下架商品）
- 营销素材的位置与锚点可视化预览
- 状态标签前的小圆点（`status-tag--dot`）、`running` / `stopped` 两个状态色类
- 文档出处标注（`chip--doc`）与差距标注（`chip--gap`）
- 未登录时的演示模式横幅（真实构建中 `App.tsx` 对未登录用户一律渲染 Login 屏）

## 走查清单（`index.html` 有完整表格）

- **契约漂移 13 处 —— 已全部修复**（后端对齐前端，`it_frontend_contract.rs` 锁定），见契约漂移矩阵。
- 7 条运营端点（`/gm/*`）后端已实现但前端无任何入口：封禁、补偿、维护模式、审计查询、管理员管理、健康视图、公告历史。
- 导航「营销下发」（文件仍叫 `canvas.html`）实为客户端图层广播，与 RGS-REQ-024 / RGS-BAS-021 的拓扑无限画布无关，仓库中无拓扑渲染代码。
- 角色只有 `superadmin` / `admin` 两档，写操作无 per-endpoint 角色校验。
- 停服务实例 / 发补偿项 / 下架商品在真实构建中为单击直发，无二次确认。
- 金额以 JSON number 传输，与仓库 README 的「整数最小货币单位」约定不一致（原型界面按电商口径以整数元显示，字段类型未改）。

## 验证方式

原型分两层验证过，两层都实跑，不是静态推断：

1. **渲染层** —— 在真实浏览器里逐页打开，确认运行期产出：经营总览的 6 张指标卡
   （活跃消费者 28 / 运行中服务实例 6 / 待处理售后 4 / 在售商品 8）、在线会话折线图的 SVG（4 条网格线 +
   采样点 + 末值）、服务实例快照按承载会话数降序的 6 行、消费者页的 `Page 1 of 3` 与
   `lastLogin` 倒序、报表按 `generatedAt` 倒序。9 页控制台均无 error / warning。

2. **交互层** —— `../tools/verify-interactions.mjs` 用 jsdom 载入真实页面脚本、
   派发真实 DOM 事件并断言页面状态，**95 项断言全部通过**：筛选、排序、分页、详情卡、
   二次确认弹窗、工单状态流转、表单校验的每一条 warning 分支、SSE 前置去重、
   九宫格锚点双向同步、JSON 高亮、空态文案区分（"无记录" vs "无匹配"）。

```bash
npm install jsdom
node docs/prototype/tools/verify-interactions.mjs          # 全部
node docs/prototype/tools/verify-interactions.mjs canvas   # 只跑某屏
```

ESM 不认 `NODE_PATH`：若在别处运行该脚本，用 `GM_PROTO_ROOT` 指向本目录。

`../tools/prototype-server.js` 是可选的本地静态服务器（走查时用），不是原型的运行前提。

## 结构

```
docs/prototype/
├── admin-console/          ← 本原型
│   ├── index.html          原型索引
│   ├── login.html … support.html   9 屏
│   ├── README.md
│   └── assets/
│       ├── prototype.css   设计令牌与组件样式（源自 App.css）
│       ├── prototype.js    导航 / 会话 / 弹窗 / SVG 图表 / JSON 高亮
│       └── data.js         演示数据集（电商口径的「值」+ 原样的字段名）
└── tools/
    ├── verify-interactions.mjs    交互回归（jsdom）
    └── prototype-server.js        可选静态服务器
```

脚本均为经典 `<script>`（非 ES module），因此 `file://` 直接可用。
`localStorage` 访问全部包在 `try/catch` 内并带内存回退，行为对齐 `App.tsx` 的 `readStorage`。
