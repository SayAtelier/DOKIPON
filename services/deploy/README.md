# 部署目标 / Deployment targets

DOKIPON 的 commerce 产品线有**两套**部署目标。它们跑的是同一份服务代码、同一个契约
（`contracts/openapi.yaml`），差别只在**拓扑**：一个是集群里的多个 Deployment，
一个是 Vercel 上的多个 Function。

| | 目录 | 拓扑 | 入口 |
|---|---|---|---|
| **集群版** | `services/deploy/k3s/` | 每个服务一个 Deployment，前面一个 Envoy Gateway | k3s 集群 + Envoy |
| **函数版** | `services/deploy/vercel/` | 每个服务一个 Vercel Function，前面一个网关 Function | Vercel |

## 怎么选

| | k3s 版 | Vercel 版 |
|---|---|---|
| 适合 | 自有机器 / 专有网络、需要 mTLS 与固定 IP | 演示、预览、对外分享一个链接 |
| 服务之间怎么通信 | Envoy 按路径前缀转发到对应 Pod | 网关 Function 按路径前缀转发到对应 Function |
| 状态 | 各 Pod 内存态（**跨 Pod 不共享**） | 各 Function 内存态（**跨实例不共享**） |
| 契约 | 23 条路径，两版完全一致 | 同左 |
| 证书 | cert-manager + Envoy TLS 终止 | Vercel 托管证书 |
| 启动成本 | 集群就绪后秒级 | 每个 Function 冷启动 |

两版的**状态语义是一样的**：服务把状态放在内存里，所以一次加购如果落到另一个实例，
那个实例看不到。这不是某一版的缺陷，而是 commerce 服务当前的状态模型；两版都受它约束。
要变成生产可用的持久化，需要把可变 store（cart / order / inventory）搬进共享数据库。

## 一处无法回避的例外

Vercel 要求**函数入口在部署根的 `api/` 下**，而部署根必须是**仓库根**——因为函数要编译
`services/crates/**` 这个 Rust workspace，函数包在子目录里就够不到那些 crate。

所以仓库根保留了三个**薄适配层**：

```
vercel.json                       # Vercel 强制要求在部署根
api/*.rs                          # 每个文件是几行入口, 实现体在 services/deploy/vercel/src/
scripts/                          # 仓库其他工具脚本
```

适配层只做转发，不含实现。其余全部按部署目标分目录：

```
services/deploy/
├── README.md                     # 本文件
├── k3s/                          # 集群版
│   ├── README.md
│   ├── 00-namespace-configmap.yaml
│   ├── 10-services.yaml          # 每个服务一个 Deployment
│   └── build-images.sh
└── vercel/                       # 函数版
    ├── README.md
    ├── env.example               # 两版用到的全部环境变量
    ├── build-vercel.sh           # 构建脚本 (在 Vercel builder 上跑)
    └── src/                      # 网关与每个 Function 的实现
```

## 契约是唯一事实来源

`contracts/openapi.yaml` 定义 23 条路径与全部字段名。**两版都必须按它实现**，
任何一版都不能自己发明字段名。这个仓库被这类漂移咬过：

- 服务端发 `session_id`，契约与 Dart 客户端要的是 `sessionId` —— 游客会话 id 变空串，
  加购按钮点了没反应，而响应是 200、字段数量也对。
- 服务端发 `amount_minor`，契约要的是 `price.amountMinor` —— 商品列表里每个价格显示成 0。

两版都由同一套集成测试守着：

```
cd services && cargo test --test '*'
```

- `services/crates/commerce-edge/tests/contract_coverage.rs` —— 每条契约路径都被接住
- `services/crates/commerce-edge/tests/contract_fields.rs` —— 字段名逐条断言

## 验证一条部署是否健康

```bash
curl -s https://<host>/healthz                      # 200
curl -s https://<host>/v1/storefront | head -c 200  # 200，且字段是 camelCase
curl -s https://<host>/v1/products   | head -c 200  # 200，items[] 带 productId
```

完整的前端实测（逐控件点击、契约断言）见 `tools/commerce-e2e/`。
