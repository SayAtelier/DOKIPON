// Rewrite the "真实实现现状" section of the storefront prototype's index
// page so it describes the state of the code as it is now.
//
// The section used to enumerate six wiring gaps as open. They are closed: the
// commerce edge serves the whole contract from one process, and the services
// emit the contract's camelCase field names. A design document that lies
// about the implementation is worse than no document, so the text has to
// follow the code.
//
// Usage: node tools/prototype/refresh-reality.mjs <path-to-index.html>

import { readFileSync, writeFileSync } from 'node:fs';

const file = process.argv[2];
if (!file) {
  console.error('usage: node refresh-reality.mjs <index.html>');
  process.exit(2);
}

const html = readFileSync(file, 'utf8');

// The file is CRLF in this repository, so the markers are matched with a
// regex rather than with literal "\n" anchors.
const startRe = /^[ \t]*<section class="sec">\r?\n[ \t]*<h2>真实实现现状 \/ What the code actually does<\/h2>/m;
const endRe = /^[ \t]*<section class="sec">\r?\n[ \t]*<h2>设计系统真实组件 \/ The real components<\/h2>/m;

const startMatch = startRe.exec(html);
const endMatch = endRe.exec(html);
if (!startMatch || !endMatch || endMatch.index < startMatch.index) {
  console.error('markers not found');
  process.exit(1);
}
const start = startMatch.index;
const end = endMatch.index;

const replacement = `    <section class="sec">
      <h2>真实实现现状 / What the code actually does</h2>
      <p class="lead">
        原型照<b>客户端期望</b>渲染。这一节记录真实代码与该期望的差距。
        最初的版本列出 6 类接线缺口；<b>它们现在都已闭合</b>，下面是闭合方式与证据。
      </p>

      <h3>① 12 条路径由一个进程提供 —— commerce-edge</h3>
      <div class="verdict">
        <b>已闭合。</b>
        新增 <span class="k">services/crates/commerce-edge</span>：把 9 个 commerce 服务挂进
        同一个 axum <span class="k">Router</span>，按路径前缀 dispatch
        （<span class="k">route_for()</span>），价格权威走进程内而不会回调自己。
        未匹配的路径由 edge 兜底 404，而不是落到最后一个服务。
        <br><br>
        <span class="k">commerce-bff</span> 不再是「只有 4 条路由且 2 个是桩」的状态：
        <span class="k">/v1/sessions/guest</span> 与 <span class="k">POST /v1/checkouts</span>
        由真实实现（LIFF id_token 校验 + nonce 防重放、订单状态机 + 幂等记录）承担。
        客户端仍然只需要一个 <span class="k">baseUri</span>。
      </div>
      <table class="tbl">
        <tr><th>路径</th><th>实现位置</th></tr>
        <tr><td class="k">GET /v1/storefront</td><td>commerce-bff</td></tr>
        <tr><td class="k">GET /v1/products, /v1/products/{id}</td><td>commerce-catalog</td></tr>
        <tr><td class="k">GET /v1/carts/{id}, PATCH …/items/{sku}</td><td>commerce-cart</td></tr>
        <tr><td class="k">GET /v1/checkouts/{id}</td><td>commerce-order</td></tr>
        <tr><td class="k">GET /v1/orders, /v1/orders/{id}, POST …/reconcile</td><td>commerce-order</td></tr>
        <tr><td class="k">POST /v1/sessions/guest, /v1/sessions/line</td><td>commerce-identity</td></tr>
        <tr><td class="k">GET /v1/inventory, /v1/payments</td><td>commerce-inventory / commerce-payment-adapter</td></tr>
        <tr><td class="k">/admin/v1/**</td><td>commerce-admin</td></tr>
      </table>

      <h3>② snake_case ↔ camelCase 漂移 —— 已由服务端对齐</h3>
      <div class="verdict">
        <b>已闭合（服务端侧）。</b>
        契约 <span class="k">contracts/openapi.yaml</span> 是 camelCase，服务端过去发的是
        snake_case，Dart 客户端只读 camelCase —— 字段会<b>静默变空</b>（产品没标题、
        订单显示 ¥0、游客会话 id 为空）。
        现在各服务按契约逐字段对齐：
      </div>
      <table class="tbl">
        <tr><th>字段</th><th>现在</th><th>后果</th></tr>
        <tr><td class="k">productId</td><td>camelCase</td><td>正常</td></tr>
        <tr><td class="k">stockPolicy</td><td>camelCase</td><td>库存 Saga 判断正确</td></tr>
        <tr><td class="k">channelEligibility</td><td>camelCase</td><td>渠道政策排除生效</td></tr>
        <tr><td class="k">titleI18n</td><td>camelCase</td><td>标题不再为空</td></tr>
        <tr><td class="k">sessionId</td><td>camelCase</td><td>游客会话 id 不再为空</td></tr>
        <tr><td class="k">lines[].unitAmount / total.amountMinor</td><td>camelCase</td><td>订单不再显示 ¥0</td></tr>
      </table>
      <div class="verdict verdict--warn">
        <b>客户端仍保留双读兜底</b>（camel 优先、snake 兜底）。这不是 workaround，
        而是让客户端同时兼容对齐前后的服务端。
      </div>

      <h3>③ 分类过滤 —— 契约里的 categoryId 现在真的接上了</h3>
      <div class="verdict">
        <b>已闭合。</b>
        <span class="k">GET /v1/products</span> 一直声明了 <span class="k">categoryId</span>，
        服务端也一直按 <span class="k">category_id</span> 过滤，但客户端从没有这个概念 ——
        分类条上的按钮是 <span class="k">onTap: () {}</span>，点不动。
        现在：<span class="k">CatalogController.selectCategory()</span> 换分类是<b>替换</b>列表
        而不是追加，过滤器随 cursor 一起走，上一个分类还在飞的请求会被丢弃
        （query id 守卫），不会覆盖用户已经选中的分类。
        demo 目录也覆盖了导航条提供的全部分类 —— 一个大半入口通向空货架的 demo
        会让能用的过滤器看起来像坏的。
      </div>

      <h3>仍然存在的限制（如实记录）</h3>
      <ul class="muted" style="padding-left:18px;line-height:1.9">
        <li><b>inventory 没有列表路由。</b>
          inventory 服务只有三条预留写路由，没有读路由，因此
          <span class="k">GET /v1/inventory</span> 是 404，
          <span class="k">GET /admin/v1/inventory</span> 也会回
          <span class="k">admin.upstream_error</span>。契约里标着
          <span class="k">x-implemented: false</span> 与
          <span class="k">x-blocked-by: inventory-service-has-no-list-route</span>。</li>
        <li><b>状态在内存里。</b>
          购物车、订单、库存都是进程内状态，Vercel 可能同时跑多个函数实例，
          跨实例的购物车会「凭空清空」。目录、内容文档与运营后台是静态或幂等读，不受影响。
          要变成生产部署，需要把可变存储搬进共享数据库 —— 改的是 store，不是 edge。</li>
        <li><b>运营后台的字段名仍是游戏域词汇。</b>
          界面已电商化，后端返回的 <span class="k">playerId</span> /
          <span class="k">vipLevel</span> / <span class="k">guild</span> /
          <span class="k">mall</span> 没改 —— 逐屏对照表见运营后台索引页。</li>
      </ul>
    </section>

`;

const out = html.slice(0, start) + replacement + html.slice(end);
writeFileSync(file, out, 'utf8');
console.log(`rewrote ${file}: ${out.length} bytes`);
