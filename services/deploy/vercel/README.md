# Vercel 版 —— Serverless 函数拓扑

这是 DOKIPON commerce 的**函数版**部署：每个 commerce 服务是一个独立的 Vercel
Function，前面一个网关 Function 按路径前缀分发。与 `services/deploy/k3s/`（集群版，
每个服务一个 Deployment + Envoy）跑的是同一份服务代码、同一个契约。

部署到 Vercel 时用这一版。

## 拓扑

```
浏览器 / Flutter Web
        │
        ▼
  /api/gateway          网关 Function：按路径前缀转发到归属服务
        │
        ├─ /v1/products…        → /api/svc/catalog
        ├─ /v1/carts…           → /api/svc/cart
        ├─ /v1/checkouts…       → /api/svc/order
        ├─ /v1/orders…          → /api/svc/order
        ├─ /v1/sessions…        → /api/svc/identity
        ├─ /v1/inventory…       → /api/svc/inventory
        ├─ /v1/payments…        → /api/svc/payment
        ├─ /v1/content…         → /api/svc/content
        └─ /admin/v1/…          → /api/svc/admin
```

与集群版一一对应：

| 集群版 | 函数版 | 归属服务 |
|---|---|---|
| `commerce-bff` Deployment | `api/svc/…` 里的网关 | 路径前缀分发 |
| `commerce-catalog` Deployment | `/api/svc/catalog` | `/v1/products` |
| `commerce-cart` Deployment | `/api/svc/cart` | `/v1/carts` |
| `commerce-order` Deployment | `/api/svc/order` | `/v1/checkouts`、`/v1/orders` |
| `commerce-identity` Deployment | `/api/svc/identity` | `/v1/sessions` |
| `commerce-inventory` Deployment | `/api/svc/inventory` | `/v1/inventory` |
| `commerce-payment-adapter` Deployment | `/api/svc/payment` | `/v1/payments` |
| `commerce-content` Deployment | `/api/svc/content` | `/v1/content` |
| `commerce-admin` Deployment | `/api/svc/admin` | `/admin/v1` |
| Envoy Gateway | `/api/gateway` | 前缀转发 |

## 实现放在哪

Vercel 要求**函数入口在部署根的 `api/` 下**，而部署根必须是仓库根（函数要编译
`services/crates/**` 这个 Rust workspace）。所以仓库根保留薄适配层：

```
api/<name>.rs        # 几行入口，实现体在 services/deploy/vercel/src/ 下
vercel.json          # Vercel 强制要求在部署根
```

其余全部在本目录：

```
services/deploy/vercel/
├── README.md         # 本文件
├── env.example       # 环境变量模板
├── build-vercel.sh   # 构建脚本（Flutter 静态包 + Rust 函数）
└── src/              # 网关与每个 Function 的实现
```

## 环境变量

见 `env.example`。要点：

- 每个服务一个 base URL（`CATALOG_URL`、`CART_URL`……）。网关按这些地址转发，
  admin 门面也用同一组变量找上游。
- **默认值必须是本地可用的地址**，不是 Docker 服务名。集群版里
  `http://commerce-catalog:8082` 这种值在 Vercel 上解析不了 —— 那是仓库里
  admin 门面 503 的根因。
- `ADMIN_USER` / `ADMIN_PASSWORD` 决定运营后台的凭据；`ADMIN_SESSION_ID` 是 admin
  代客户读订单时呈现的会话 id（订单按 buyer 隔离，没有它每个读都会 401）。

### 凭据是必填的

`ADMIN_PASSWORD`、`LINE_ID_TOKEN_KEY`、`LINE_CHANNEL_ID`、`PSP_WEBHOOK_SECRET`
这四项**没有可用的默认值**，部署必须自己给：

| 变量 | 决定什么 | 不给会怎样 |
|---|---|---|
| `ADMIN_PASSWORD` | 谁能登录运营后台 | 后台对整个互联网开放 |
| `LINE_ID_TOKEN_KEY` | 谁签发的 LINE id 令牌算数 | 任何人都能伪造任意 LINE 用户登录 |
| `LINE_CHANNEL_ID` | id 令牌的受众 | 同上，且令牌不绑定渠道 |
| `PSP_WEBHOOK_SECRET` | 哪个 PSP 的回调算数 | 任何人都能伪造支付成功回调 |
| `SERVICE_TOKEN` | 服务之间怎么证明「这是内部调用」 | 后台读不到跨会话的订单 |

原因很直接：这几个开发默认值（`dev-only-admin` / `dev-only-key` /
`dev-only-secret`）就写在仓库里。一个没设这些变量的部署**不会报错**——它会照常
提供服务，只是任何读过这份源码的人都能进来。

所以服务在启动前会检查这些值：没设、空、或者还是开发默认值，**服务直接拒绝启动**，
而不是带着开发默认值对外服务。

本地跑（要接受开发默认值）另外设 `DOKIPON_ALLOW_DEV_SECRETS=1`；
部署**永远不要**设它。

## 状态

服务把状态放在**内存**里。函数实例之间不共享——一次加购如果落到另一个实例，那个实例
看不到，购物车会“凭空清空”。集群版的多个 Pod 也是同样的情况（内存态、跨 Pod 不共享）。

这不是这一版的缺陷，而是 commerce 服务当前的状态模型；**两版都受它约束**。
要变成生产可用的持久化，需要把可变 store（cart / order / inventory）搬进共享数据库——
改的是 store，不是拓扑。

## 构建

`build-vercel.sh` 在 Vercel 的 builder 上跑：

1. 装 Flutter SDK（校验 sha256）
2. `flutter build web` 出门店与运营后台两个包，合并进 `public/`
3. Rust 函数由 Vercel 的 Rust builder 编译

## 验证

```bash
curl -s https://<host>/healthz
curl -s https://<host>/v1/products | head -c 200
```

完整的前端实测（逐控件点击 + 契约断言）见 `tools/commerce-e2e/`。
