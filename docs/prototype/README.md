# DOKIPON 原型集 / Prototype suites

本目录是 DOKIPON（GemArc Commerce）的 **HTML 静态原型**。两套,对应仓库的两条产品线:

| 套件 | 目录 | 对应代码 | 领域 |
|---|---|---|---|
| **消费者端 storefront** | `storefront/` | `packages/design_system` + `packages/domain_models` + `packages/storefront_state` + `packages/api_client` + `services/crates/commerce-*` | **电商交易系统**（当前产品线：App / Web / LINE MINI App 三端共用） |
| **运营后台 admin console** | `admin-console/` | `crates/gm-backend`（Actix-web）+ `tools/gm-console/frontend`（React 18 + Vite + TS） | 游戏服务器遗留线。界面已电商化，但**底层字段名仍沿用游戏域词汇**（`playerId` / `vipLevel` / `guild` / `mall`），见 `admin-console/index.html` 的「游戏服务器 → 电商 改造对照表」 |

> 本仓库由**游戏服务器**改造为**电商交易系统**（`docs/ecommerce/README.md` 有完整的
> Atlas → 电商收敛对照表）。两套原型并列放在这里，正是为了把「哪些是现役、哪些是遗留」
> 摆在明面上，而不是混成一套看不出年代感的屏。

## 打开方式

**双击 `storefront/index.html` 或 `admin-console/index.html` 即可。** 无构建、无起服务、无网络请求、无外部资源。
每屏右侧（或下方）的说明面板标注了它对应的 API、源码位置与可复现的操作。

## 消费者端 storefront（8 屏）

| 文件 | 屏 | 依据 |
|---|---|---|
| `storefront/index.html` | 套件索引 + **真实实现现状** | openapi / commerce-* / Dart 客户端三方对照 |
| `storefront/catalog.html` | 商品列表 | `CatalogScreen` — 渠道过滤、cursor 分页、游客降级 |
| `storefront/product.html` | 商品详情 | `ProductDetail` — 权威价格、价格版本、渠道资格 |
| `storefront/cart.html` | 购物车 | `Cart` — 快照小计、乐观并发 409 |
| `storefront/checkout.html` | 结算 | `CheckoutStatus` 全 10 态；`payment_unknown` 只轮询不重提 |
| `storefront/order.html` | 订单 | `Order` — 事件轨迹 / Outbox / 对账 |
| `storefront/session.html` | 会话与降级 | `SessionController` — LIFF-002 ~ 006 |
| `storefront/problem.html` | 错误显示 | `ApiProblem` — problem+json 与 code 分支 |

视觉令牌逐值取自 `packages/design_system/lib/src/tokens.dart`；组件行为取自
`product_card.dart` / `availability_badge.dart` / `price_label.dart` / `status_banner.dart`。

## 运营后台 admin console（9 屏）

`index.html` / `login.html` / `dashboard.html` / `players.html` / `servers.html` /
`items.html` / `canvas.html` / `mall.html` / `reports.html` / `support.html`

文件**保持原名**：它们镜像后端端点族（`/gm/players`、`/gm/mall/items` …），
对照关系见 `admin-console/index.html`。

## 验证

```bash
npm install jsdom
node docs/prototype/tools/verify-interactions.mjs
```

当前：**149 项断言全部通过**（运营后台 95 + 消费者端 54）。
脚本用 jsdom 载入**真实页面脚本**、派发**真实 DOM 事件**、断言**页面状态**——
不是静态检查，因此"渲染出来了但处理器是死的"这种情况会被抓出来。

- 运营后台套件：筛选、排序、分页、详情卡、二次确认弹窗、工单状态流转、
  每一条表单校验分支、路由覆盖矩阵与漂移矩阵的行数/标记一致性
- 消费者端套件：渠道过滤（LINE/Web 差异）、cursor 分页、游客横幅、
  购物车版本冲突 409、结算状态机与 `payment_unknown` 的"禁止重提"文案、
  会话五种降级、problem+json 的分支命中

ESM 不认 `NODE_PATH`：若在别处运行脚本，用 `GM_PROTO_ROOT` 指向 `admin-console`，
`storefront` 会自动取同级目录。

## 诚实边界

原型是**设计评审物**，不是联调测试：

1. 页面上的数据全是内置演示值，不代表任何一次真实请求。
2. 消费者端按**客户端期望**的形状渲染；服务端实际返回的形状与之有 6 处已知差异，
   逐条列在 `storefront/index.html` 的「真实实现现状」里 —— 那些是**真实代码里待修的接线缺口**，
   原型无法替它们补上。
3. 运营后台的界面已电商化，但**后端字段名仍是游戏域词汇**；`admin-console/index.html`
   的改造对照表逐屏列明了对应关系。
