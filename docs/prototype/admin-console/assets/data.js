/* ==========================================================================
   DOKIPON 运营后台 — prototype demo dataset (电商口径)
   --------------------------------------------------------------------------
   Field names and shapes are copied 1:1 from the real contracts so the
   prototype cannot drift from the implementation:

     players  ← crates/gm-backend/src/players_handler.rs + frontend Players.tsx
     servers  ← crates/gm-backend/src/servers_handler.rs
     mall     ← crates/gm-backend/src/mall_handler.rs
     grants   ← crates/gm-backend/src/items_handler.rs
     tickets  ← crates/gm-backend/src/support_handler.rs
     reports  ← crates/gm-backend/src/reports_handler.rs
     summary  ← crates/gm-backend/src/summary_handler.rs

   字段名（players / vipLevel / guild / totalSpend / onlinePlayers …）是后端
   真实契约，本文件只改「值」的领域语义：游戏侧的顾客名 / 公会名 / 道具 ID /
   商城商品已换成电商侧的顾客名 / 会员分组 / 补偿项 ID / 在售商品。
   状态枚举（online / offline / banned、running / stopped、open / pending /
   resolved）同样是契约值，一律不动。

   Every record is demo data. Nothing here is fetched; the file:// prototype
   has no backend, and every page labels its numbers as demo values.
   Timestamps are generated relative to page load so the screens never go stale.
   ========================================================================== */

window.GM_DATA = (function () {
  'use strict';

  var MIN = 60 * 1000;
  var HOUR = 60 * MIN;
  var DAY = 24 * HOUR;
  var now = Date.now();

  function ago(ms) {
    return new Date(now - ms).toISOString();
  }

  /* ------------------------------------------------------------- players -- */
  /* Player{id,name,level,vipLevel,status,lastLogin,totalSpend,region?,guild?}
     name = 真实电商顾客（中日英混排）；guild = 会员分组（遗留字段名 guild）。 */

  var RAW_PLAYERS = [
    ['Kaede Tsukimori', 87, 6, 'online', 4 * MIN, 4820.5, 'ap-northeast-1', '企业客户 / Enterprise'],
    ['Rin Asakura', 64, 4, 'online', 9 * MIN, 2130.0, 'ap-northeast-1', '企业客户 / Enterprise'],
    ['李佳宁', 72, 5, 'online', 12 * MIN, 3390.75, 'ap-northeast-1', null],
    ['Mira Halvorsen', 51, 3, 'online', 18 * MIN, 980.5, 'ap-southeast-1', 'VIP客群'],
    ['Tomo Aikawa', 43, 2, 'offline', 3 * HOUR, 640.25, 'ap-northeast-1', null],
    ['Olivia Bennett', 78, 5, 'offline', 26 * HOUR, 5210.0, 'ap-southeast-1', '普通客群 / Standard'],
    ['陈思远', 35, 1, 'offline', 2 * DAY, 210.0, 'ap-northeast-1', '新客群 / New member'],
    ['Emma Whitfield', 91, 7, 'online', 6 * MIN, 8840.0, 'ap-southeast-1', 'VIP客群'],
    ['王志远', 59, 3, 'banned', 5 * DAY, 1260.75, 'ap-northeast-1', '普通客群 / Standard'],
    ['Hana Mizutani', 66, 4, 'offline', 8 * HOUR, 2750.5, 'ap-northeast-1', null],
    ['张晓彤', 48, 2, 'offline', 30 * HOUR, 430.0, 'ap-southeast-1', null],
    ['Chloe Martin', 83, 6, 'online', 21 * MIN, 6120.25, 'ap-northeast-1', '企业客户 / Enterprise'],
    ['郑海燕', 55, 3, 'offline', 14 * HOUR, 1580.0, 'ap-southeast-1', null],
    ['Sofia Ricci', 39, 1, 'offline', 4 * DAY, 305.5, 'ap-northeast-1', '新客群 / New member'],
    ['Kaito Muroyama', 74, 5, 'online', 15 * MIN, 3940.0, 'ap-northeast-1', '企业客户 / Enterprise'],
    ['Sophie Laurent', 46, 2, 'offline', 19 * HOUR, 720.25, 'ap-southeast-1', null],
    ['Marcus Reed', 69, 4, 'offline', 6 * DAY, 2470.0, 'ap-northeast-1', null],
    ['Sanae Fujimoto', 61, 4, 'online', 7 * MIN, 3110.75, 'ap-northeast-1', 'VIP客群'],
    ['Oliver Grant', 33, 1, 'offline', 8 * DAY, 145.0, 'ap-southeast-1', '新客群 / New member'],
    ['刘雅琪', 76, 5, 'offline', 11 * HOUR, 4880.5, 'ap-northeast-1', '普通客群 / Standard'],
    ['Riku Nakagawa', 52, 3, 'offline', 40 * MIN, 1490.25, 'ap-northeast-1', null],
    ['Amelia Turner', 80, 6, 'online', 11 * MIN, 5440.0, 'ap-southeast-1', 'VIP客群'],
    ['黄子豪', 44, 2, 'banned', 12 * DAY, 890.0, 'ap-northeast-1', null],
    ['Aiko Shimomura', 58, 3, 'offline', 22 * HOUR, 1760.75, 'ap-northeast-1', '企业客户 / Enterprise'],
    ['王一鸣', 36, 1, 'offline', 5 * DAY, 260.25, 'ap-southeast-1', '新客群 / New member'],
    ['Freya Lindgren', 68, 4, 'online', 24 * MIN, 3670.5, 'ap-northeast-1', null],
    ['Yui Tahara', 47, 2, 'offline', 16 * HOUR, 1110.0, 'ap-southeast-1', null],
    ['Clara Benedetti', 71, 5, 'offline', 9 * DAY, 4130.25, 'ap-southeast-1', 'VIP客群']
  ];

  var players = RAW_PLAYERS.map(function (r, i) {
    return {
      id: 10001 + i,
      name: r[0],
      level: r[1],
      vipLevel: r[2],
      status: r[3],
      lastLogin: ago(r[4]),
      totalSpend: r[5],
      region: r[6],
      guild: r[7]
    };
  });

  function playerStats(list) {
    var s = { total: list.length, online: 0, offline: 0, banned: 0, highValue: 0, _sumLevel: 0 };
    list.forEach(function (p) {
      s[p.status] += 1;
      s._sumLevel += p.level;
      // 阈值对齐后端 players_handler.rs::get_player_stats：total_spent > 1000.0
      if (p.totalSpend > 1000) s.highValue += 1;
    });
    s.averageLevel = list.length ? Math.round((s._sumLevel / list.length) * 10) / 10 : 0;
    delete s._sumLevel;
    return s;
  }

  /* ------------------------------------------------------------- servers -- */
  /* Server{id,name,region,status:'running'|'stopped',onlinePlayers,lastUpdated}
     电商服务实例：catalog / cart / order / identity / bff / payment / inventory。 */

  var servers = [
    { id: 'catalog-1',   name: 'Catalog Service #1',   region: 'ap-northeast-1', status: 'running', onlinePlayers: 2874,  lastUpdated: ago(6 * 1000) },
    { id: 'cart-1',      name: 'Cart Service #1',      region: 'ap-northeast-1', status: 'running', onlinePlayers: 6103,  lastUpdated: ago(6 * 1000) },
    { id: 'order-1',     name: 'Order Service #1',     region: 'ap-northeast-1', status: 'running', onlinePlayers: 8421,  lastUpdated: ago(11 * 1000) },
    { id: 'identity-1',  name: 'Identity Service #1',  region: 'ap-northeast-1', status: 'running', onlinePlayers: 3950,  lastUpdated: ago(9 * 1000) },
    { id: 'bff-1',       name: 'BFF Service #1',       region: 'ap-northeast-1', status: 'running', onlinePlayers: 12,    lastUpdated: ago(4 * 1000) },
    { id: 'payment-1',   name: 'Payment Service #1',   region: 'ap-southeast-1', status: 'stopped', onlinePlayers: 0,     lastUpdated: ago(2 * HOUR) },
    { id: 'inventory-1', name: 'Inventory Service #1', region: 'ap-southeast-1', status: 'running', onlinePlayers: 0,     lastUpdated: ago(41 * 1000) }
  ];

  /* --------------------------------------------------------- mall items -- */
  /* MallItem{id,name,price} —— 价格为整数元，与电商侧「整数最小货币单位」口径对齐。 */

  var mallItems = [
    { id: 101, name: '咖啡豆礼盒',      price: 128 },
    { id: 102, name: '保温杯',          price: 89 },
    { id: 103, name: '会员月卡',        price: 39 },
    { id: 104, name: '手冲咖啡壶',      price: 168 },
    { id: 105, name: '帆布托特包',      price: 59 },
    { id: 106, name: '烘焙挂耳咖啡',    price: 45 },
    { id: 107, name: '咖啡杯套装',      price: 99 },
    { id: 108, name: '线上会员年卡',    price: 399 }
  ];

  /* -------------------------------------------------------------- grants -- */
  /* GrantRecord{id,playerId,itemId,amount,admin,createdAt}
     补偿项 ID 沿用 item_ 前缀（契约形状），语义换成电商补偿权益。 */

  var RAW_GRANTS = [
    [2001, '10008', 'item_voucher_500',    1,    'sre_lead',     7 * MIN],
    [2002, '10001', 'item_coupon_10pct',   3,    'ops_admin',    22 * MIN],
    [2003, '10012', 'item_shipping_free',  1,    'ops_admin',    1 * HOUR],
    [2004, '10017', 'item_credit_1000',    1000, 'sre_lead',     2 * HOUR],
    [2005, '10022', 'item_voucher_500',    2,    'ops_admin',    3 * HOUR],
    [2006, '10003', 'item_gift_card_200',  1,    'commerce_ops', 5 * HOUR],
    [2007, '10009', 'item_membership_month', 1,  'sre_lead',     9 * HOUR],
    [2008, '10024', 'item_voucher_500',    5,    'ops_admin',    14 * HOUR],
    [2009, '10005', 'item_coupon_10pct',   1,    'commerce_ops', 1 * DAY],
    [2010, '10014', 'item_shipping_free',  1,    'ops_admin',    1 * DAY + 4 * HOUR],
    [2011, '10019', 'item_credit_1000',    1000, 'sre_lead',     2 * DAY],
    [2012, '10002', 'item_gift_card_200',  1,    'ops_admin',    2 * DAY + 9 * HOUR],
    [2013, '10026', 'item_voucher_500',    2,    'commerce_ops', 3 * DAY],
    [2014, '10011', 'item_membership_month', 1,  'sre_lead',     4 * HOUR]
  ];

  var grants = RAW_GRANTS.map(function (r) {
    return {
      id: r[0],
      playerId: r[1],
      itemId: r[2],
      amount: r[3],
      admin: r[4],
      createdAt: ago(r[5])
    };
  });

  /* ------------------------------------------------------------- tickets -- */
  /* Ticket{id,playerId,message,status:'open'|'pending'|'resolved',admin,createdAt,updatedAt}
     诉求内容为电商售后场景：未收到货 / 退款未到账 / 商品破损 / 发票问题 等。 */

  var RAW_TICKETS = [
    [301, '10008', '订单 PSP-884213 的咖啡豆礼盒至今未收到，物流只显示已揽收。', 'open', 'sre_lead', 18 * MIN, 18 * MIN],
    [302, '10012', '退款申请提交后 5 个工作日仍未到账，请协助查询退款流水。', 'open', null, 2 * HOUR, 2 * HOUR],
    [303, '10022', '保温杯收到时外包装破损，杯身外壳有两处凹陷。', 'pending', 'ops_admin', 6 * HOUR, 1 * HOUR],
    [304, '10005', '会员月卡已扣款，但会员权益没有开通。', 'open', null, 9 * HOUR, 9 * HOUR],
    [305, '10026', '同一订单被重复扣款两次，请退回多收的一笔。', 'pending', 'sre_lead', 1 * DAY, 5 * HOUR],
    [306, '10017', '账号被误封禁（登错账号），已附申诉材料。', 'resolved', 'sre_lead', 2 * DAY, 1 * DAY],
    [307, '10014', '商品详情页图片与实物颜色不一致，希望换成同款。', 'resolved', 'ops_admin', 3 * DAY, 2 * DAY],
    [308, '10009', '下单后一直提示库存不足，但商品页显示有货。', 'resolved', 'ops_admin', 4 * DAY, 3 * DAY],
    [309, '10019', '请确认本月会员月卡是否已完成续费。', 'pending', null, 5 * DAY, 2 * DAY],
    [310, '10002', '发票抬头修改后没有生效，收到的发票仍是旧抬头。', 'open', null, 6 * DAY, 6 * DAY]
  ];

  var tickets = RAW_TICKETS.map(function (r) {
    return {
      id: r[0],
      playerId: r[1],
      message: r[2],
      status: r[3],
      admin: r[4],
      createdAt: ago(r[5]),
      updatedAt: ago(r[6])
    };
  });

  /* ------------------------------------------------------------- reports -- */
  /* Reports are free-form: Reports.tsx reads type|category|reportType and
     period|range|window, and renders whatever `metrics` object arrives. The
     shapes below keep that duck-typing intact (one record even omits
     `summary`, exactly like a real sparse backend payload).
     type 值（Retention / Monetisation / Performance）是契约枚举，界面上的
     中文口径由 reports.html 的标签映射给出。 */

  var reports = [
    {
      id: 'RP-2026-W41', type: 'Retention', period: '2026-W41',
      generatedAt: ago(2 * HOUR),
      summary: '本周复购率 41.2%（较上周 +3.4pt）；7 日复购 18.9%；30 日复购 9.4%。',
      metrics: { d1_retention: 41.2, d7_retention: 18.9, d30_retention: 9.4, new_accounts: 41280 }
    },
    {
      id: 'RP-2026-W41', type: 'Monetisation', period: '2026-W41',
      generatedAt: ago(3 * HOUR),
      summary: '本周 GMV 214,800 元，覆盖 8 个在售商品；客单价 61.20 元。',
      metrics: { gross_revenue: 214800, arppu: 61.2, paying_users: 3510, conversion: 3.8 }
    },
    {
      id: 'RP-2026-W40', type: 'Retention', period: '2026-W40',
      generatedAt: ago(1 * DAY + 2 * HOUR),
      summary: '复购率 37.8%（09-14 促销结束后回落）。',
      metrics: { d1_retention: 37.8, d7_retention: 17.4, d30_retention: 9.1, new_accounts: 38770 }
    },
    {
      id: 'RP-2026-W40', type: 'Monetisation', period: '2026-W40',
      generatedAt: ago(1 * DAY + 3 * HOUR),
      metrics: { gross_revenue: 198400, arppu: 59.8, paying_users: 3317, conversion: 3.5 }
    },
    {
      id: 'RP-2026-W40', type: 'Performance', period: '2026-W40',
      generatedAt: ago(1 * DAY + 5 * HOUR),
      summary: '订单履约平均 26.4 小时；准时发货率 94.2%。',
      metrics: { fulfilment_hours_avg: 26.4, on_time_rate: 94.2, refund_rate: 1.8, cancelled_orders: 12 }
    },
    {
      id: 'RP-2026-W39', type: 'Performance', period: '2026-W39',
      generatedAt: ago(7 * DAY),
      summary: '周末高峰订单履约平均 31.8 小时；退款率升至 2.6%。',
      metrics: { fulfilment_hours_avg: 31.8, on_time_rate: 90.4, refund_rate: 2.6, cancelled_orders: 27 }
    },
    {
      id: 'RP-2026-W39', type: 'Retention', period: '2026-W39',
      generatedAt: ago(7 * DAY + 1 * HOUR),
      metrics: { d1_retention: 39.5, d7_retention: 18.1, d30_retention: 9.6, new_accounts: 40110 }
    }
  ];

  /* ---------------------------------------------------------- broadcasts -- */
  /* BroadcastEntry{id,message,admin,createdAt} —— 电商运营公告。 */

  var broadcasts = [
    { id: 9001, message: '系统维护 04:00–04:20 JST，期间下单与结算可能短暂失败。', admin: 'sre_lead',     createdAt: ago(26 * MIN) },
    { id: 9002, message: '会员月卡已上架，在售至 2026-10-31。',                     admin: 'ops_admin',    createdAt: ago(3 * HOUR) },
    { id: 9003, message: '咖啡豆礼盒到货，店铺新增 3 款冲煮器具。',                   admin: 'commerce_ops', createdAt: ago(7 * HOUR) },
    { id: 9004, message: '09-14 订单系统故障补偿：每单补偿 50 元代金券。',            admin: 'sre_lead',     createdAt: ago(1 * DAY) },
    { id: 9005, message: '支付服务实例 payment-1 维护中，结算请稍后再试。',             admin: 'sre_lead',     createdAt: ago(2 * DAY) }
  ];

  /* -------------------------------------------------------------- summary -- */
  /* One payload for GET /gm/summary — Dashboard renders every branch of it. */

  function buildSummary() {
    var ps = playerStats(players);
    var running = servers.filter(function (s) { return s.status === 'running'; }).length;
    return {
      playerStats: ps,
      activity: {
        totalBroadcasts: 128,
        recentBroadcasts: broadcasts,
        totalGrants: 4312
      },
      support: {
        open: tickets.filter(function (t) { return t.status === 'open'; }).length,
        total: tickets.length
      },
      mall: { totalItems: mallItems.length },
      servers: {
        stats: { total: servers.length, running: running },
        list: servers
      }
    };
  }

  /* ---------------------------------------------- /gm/metrics (Prometheus) */
  /* servers_handler.rs::metrics 的真实 body 恰好三行，**没有** # HELP / # TYPE：
       online_connections {sum(online_players)}
       running_servers {n}
       total_servers {n}
     ActiveUsersChart.tsx 用 /online_connections\s+(\d+)/ 从这段文本里抓当前连接数。
     原型按此原样生成——不杜撰 HELP/TYPE 行,也不编造后端并不产出的指标。 */

  function onlineConnections() {
    return servers.reduce(function (acc, s) { return acc + (s.onlinePlayers || 0); }, 0);
  }

  function metricsText(onlineConnectionsValue) {
    var running = servers.filter(function (s) { return s.status === 'running'; }).length;
    return 'online_connections ' + onlineConnectionsValue + '\n' +
      'running_servers ' + running + '\n' +
      'total_servers ' + servers.length + '\n';
  }

  return {
    players: players,
    playerStats: playerStats,
    servers: servers,
    mallItems: mallItems,
    grants: grants,
    tickets: tickets,
    reports: reports,
    buildSummary: buildSummary,
    metricsText: metricsText,
    onlineConnections: onlineConnections
  };
})();
