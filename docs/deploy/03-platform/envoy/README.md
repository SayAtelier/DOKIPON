# Envoy Gateway — 电商入口

把商城的两个 Flutter 前台和两个后端 API 收进一个 Envoy Gateway，按 Host 头分流：

| 域名 | 内容 | 来源 |
|---|---|---|
| `shop.local` | 消费者商城（Flutter Web） | `shop-web` Deployment |
| `shop.local/v1/*` | BFF：storefront / guest session / checkout | `commerce-bff` |
| `shop.local/v1/products` | 商品目录 | `commerce-catalog` |
| `admin.local` | 后台控制台（Flutter Web） | `admin-web` Deployment |
| `admin.local/admin/v1/*` | 后台 API | `commerce-admin` |

## 文件

| 文件 | 内容 |
|---|---|
| `05-envoyproxy.yaml` | 数据面规格：DaemonSet、ClusterIP→NodePort、资源、Service patch |
| `10-commerce-gateway.yaml` | GatewayClass / Gateway / 两条 HTTPRoute |
| `20-commerce-tls.yaml` | cert-manager Issuer + Certificate（**不含** CA Secret，见下） |

## 部署顺序

```bash
# 1. 控制器（一次性）
#    官方 install.yaml，镜像换成 daocloud 加速源，imagePullPolicy 改 Never
kubectl apply -f eg-install-local.yaml

# 2. EnvoyProxy CRD —— 必须用 create，不能用 apply
#    apply 会注入 last-applied 注解，而这个 CRD 的 schema 有 1.1 MB，
#    超过 metadata 262144 字节的上限，直接被 API server 拒绝
kubectl create -f eg-crd.yaml

# 3. 数据面 + 网关
kubectl apply -f docs/deploy/03-platform/envoy/05-envoyproxy.yaml
kubectl apply -f docs/deploy/03-platform/envoy/10-commerce-gateway.yaml

# 4. HTTPS 证书（见下，必须在第 5 步之前）
bash tools/setup-commerce-tls.sh

# 5. 确认两个 listener 都编程成功
kubectl get gateway -n commerce
#   https  Programmed=True Accepted=True ResolvedRefs=True
#   http   Programmed=True Accepted=True ResolvedRefs=True
```

### 为什么第 4 步不能省

HTTPS listener 引用 `commerce-wildcard-tls` 这个 Secret。证书不存在时，
Gateway 的 listener 会停在 `ResolvedRefs=False / InvalidCertificateRef`，
**整个 Gateway 都不会被编程**（`Programmed=False`），连 HTTP listener 也一起挂。

`tools/setup-commerce-tls.sh` 做三件事：
把 `rust-game-server/rgs-secret-ca` 复制到 commerce 命名空间
（cert-manager 的 Issuer 只能签发同命名空间的 Certificate）、
创建引用它的 Issuer、签发 Certificate。

CA 私钥**不进仓库**，脚本在集群内读出再写回。Secret 用 `kubectl create` 而非
`apply` 创建，避免 kubectl 把私钥塞进 last-applied 注解。

**`20-commerce-tls.yaml` 里没有 commerce-ca 这个 Secret，这是故意的。**
如果把它写进 manifest 并 apply，apply 会用 manifest 的内容覆盖集群里的 Secret ——
而 manifest 没有 data 字段（私钥不能提交），于是 ca.crt / tls.crt / tls.key 全被清空，
Issuer 立刻失去 CA，签发和续签一起失败。

## 访问

```bash
# VM 上（NodePort 31080 / 31443）
curl -H "Host: shop.local"  http://172.31.155.166:30080/
curl -H "Host: admin.local" http://172.31.155.166:30080/healthz

# HTTPS：内部 CA 是自签的，浏览器会警告，属预期
curl --cacert ca.pem --resolve shop.local:31443:127.0.0.1 https://shop.local:30443/
```

Windows 宿主直接访问 `http://<VM_IP>:31080`，需要在 hosts 里加
`127.0.0.1 shop.local admin.local`（或用 `--resolve`）。

## 测试

Playwright 的 Chromium 只装在 VM 上（Windows 侧下载会被网络挡掉），
依赖记录在 `tools/e2e/package.json`：

```bash
# 1. 把测试脚本送进 VM
#    注意：multipass transfer 不支持目录递归
#    （"omitting local directory ... recursive mode not specified"），
#    必须先打包再传，然后解包
tar czf e2e.tgz -C tools e2e
multipass transfer e2e.tgz informed-impala:/home/ubuntu/
multipass exec informed-impala -- bash -lc \
  'cd /home/ubuntu && tar xzf e2e.tgz && rm e2e.tgz'

# 2. 装依赖（首次，约 2 分钟）
multipass exec informed-impala -- bash -lc \
  'cd /home/ubuntu/e2e && npm install && npx playwright install --with-deps chromium'

# 3. 跑检测器自检 —— 它证明"空白页检测"不是摆设
multipass exec informed-impala -- bash -lc 'cd /home/ubuntu/e2e && node selfcheck.js'

# 4. 跑主测试
multipass exec informed-impala -- bash -lc \
  'cd /home/ubuntu/e2e && PW=$(sudo /usr/local/bin/k3s kubectl get secret commerce-admin-credentials -n commerce -o jsonpath="{.data.OPERATOR_PASSWORD}" | base64 -d) &&
   ENVOY=http://127.0.0.1:30080 ADMIN_USER=operator ADMIN_PASSWORD="$PW" node e2e_envoy.js'
```

`npm install` 是必须的：测试依赖 `pngjs` 解析截图，`tools/e2e/package.json`
把它和 `playwright` 一起声明了。干净克隆不装依赖会直接 `MODULE_NOT_FOUND`。

`tools/e2e/selfcheck.js` 是检测器的自检：它用合成的 PNG 证明
"单色页面被判为空白、稀疏但真实绘制的页面不被误判"。
没有它，一个永远通过的检测和一个坏掉的检测没有区别。

## 已知的坑（都写在 manifest 注释里）

- **Service 必须 ClusterIP**：k3s 以 `disable: servicelb` 运行，LoadBalancer
  Service 永远拿不到地址，Gateway 停在 `AddressNotAssigned`。
- **EnvoyProxy 改动不重建数据面**：旧配置会留在 Deployment 的 args 里。
  改完必须 `kubectl delete deploy/ds commerce-envoy`，强制重新翻译。
- **DaemonSet 的资源路径**是 `envoyDaemonSet.container.resources`，
  没有 Deployment 形态的那层 `pod`。
- **`parametersRef` 是本地引用**：没有 `namespace` 字段，EnvoyProxy 必须和
  Gateway 同命名空间。
- **`logging.level.gateway` 不是合法 component**，配上数据面启动即退。
- **NodePort 30080/30443 沿用原 commerce 服务的端口号**，那三个号原本是 commerce 服务的，
  留着是为了以后能安全地重新启用。
