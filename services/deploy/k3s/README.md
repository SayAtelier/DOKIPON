# k3s 版 —— 微服务拓扑

DOKIPON commerce 的**集群版**部署：每个 commerce 服务一个 Deployment，前面一个
Envoy Gateway。与 `services/deploy/vercel/`（函数版，每个服务一个 Vercel Function）
跑的是同一份服务代码、同一个契约。

## 拓扑

| Deployment | 端口 | 归属路径 |
|---|---|---|
| `commerce-bff` | | 路径前缀分发 |
| `commerce-catalog` | 8082 | `/v1/products` |
| `commerce-cart` | 8083 | `/v1/carts` |
| `commerce-order` | 8085 | `/v1/checkouts`、`/v1/orders` |
| `commerce-identity` | 8081 | `/v1/sessions` |
| `commerce-inventory` | 8084 | `/v1/inventory` |
| `commerce-payment-adapter` | 8086 | `/v1/payments` |
| `commerce-content` | 8096 | `/v1/content` |
| `commerce-admin` | 8090 | `/admin/v1` |

Envoy 终止 TLS 并按路径前缀转发到各 Pod —— 它就是集群版的网关。

## 文件

| 文件 | 作用 |
|---|---|
| `00-namespace-configmap.yaml` | 命名空间与共享配置 |
| `10-services.yaml` | 每个服务一个 Deployment + Service |
| `build-images.sh` | 构建并推送容器镜像 |

## 与函数版的差别

只有拓扑不同：**服务代码、契约、字段名、状态语义全部一样**。
两版都把可变状态放在内存里，所以跨实例/跨 Pod 的购物车都看不到。

集群版额外有 mTLS、cert-manager 托管证书、固定出网 IP；函数版换来的是
一个可分享的链接和零运维。选型见 `services/deploy/README.md`。

## 验证

```bash
curl -s https://<host>/healthz
curl -s https://<host>/v1/products | head -c 200
```

契约覆盖测试（两版共用）：

```bash
cd services && cargo test --test '*'
```
