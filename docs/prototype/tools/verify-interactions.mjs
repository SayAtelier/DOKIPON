/**
 * RGS GM Console prototype — interaction regression harness.
 *
 * The in-app Browser provider cannot inject clicks/typing into these pages,
 * so this harness runs the real page scripts under jsdom, dispatches real DOM
 * events, and asserts on the resulting page state.
 *
 * Run:
 *   npm install jsdom          # any directory; NODE_PATH must resolve it
 *   node docs/prototype/tools/verify-interactions.mjs
 *
 * It asserts on rendering AND on interaction outcomes. A page that renders but
 * whose handlers are dead will fail here.
 */

import { JSDOM, VirtualConsole } from 'jsdom';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import fs from 'node:fs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
// 默认按脚本自身位置推导；也可用 GM_PROTO_ROOT 覆盖，便于把脚本复制到别处运行
// （ESM 不认 NODE_PATH，jsdom 必须能从脚本所在目录向上解析到）。
const ADMIN_ROOT = process.env.GM_PROTO_ROOT
  ? path.resolve(process.env.GM_PROTO_ROOT)
  : path.resolve(HERE, '..', 'admin-console');
const STOREFRONT_ROOT = path.resolve(path.dirname(ADMIN_ROOT), 'storefront');

if (!fs.existsSync(ADMIN_ROOT)) {
  process.stderr.write('后台原型目录不存在: ' + ADMIN_ROOT + '\n设 GM_PROTO_ROOT 指向 docs/prototype/admin-console\n');
  process.exit(2);
}
if (!fs.existsSync(STOREFRONT_ROOT)) {
  process.stderr.write('消费者端原型目录不存在: ' + STOREFRONT_ROOT + '\n');
  process.exit(2);
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

let passed = 0;
const failures = [];

function check(name, condition, detail) {
  if (condition) {
    passed += 1;
  } else {
    failures.push(name + (detail ? ' → ' + detail : ''));
  }
}

async function load(file, root) {
  const pageErrors = [];
  const vc = new VirtualConsole();
  vc.on('jsdomError', (e) => pageErrors.push(String(e.message || e)));
  vc.on('error', (...a) => pageErrors.push(a.join(' ')));

  const dom = await JSDOM.fromFile(path.join(root || ADMIN_ROOT, file), {
    runScripts: 'dangerously',
    resources: 'usable',
    pretendToBeVisual: true,
    virtualConsole: vc,
    beforeParse(win) {
      win.Element.prototype.scrollIntoView = function () {};
      win.HTMLElement.prototype.scrollIntoView = function () {};
    }
  });

  await new Promise((resolve) => {
    if (dom.window.document.readyState === 'complete') resolve();
    else dom.window.addEventListener('load', resolve);
  });
  await sleep(120);

  const win = dom.window;
  const doc = win.document;
  const $ = (sel) => doc.querySelector(sel);
  const text = (sel) => ($(sel) ? $(sel).textContent.trim() : '');
  const all = (sel) => Array.from(doc.querySelectorAll(sel));

  /* 表格行：传入的元素可能是 <table>，也可能是 <tbody id="...">。
     后者没有嵌套 tbody，必须走 :scope > tr，否则永远返回空。 */
  const rows = (sel) => {
    const host = $(sel);
    if (!host) return [];
    if (host.tagName === 'TBODY') return Array.from(host.querySelectorAll(':scope > tr'));
    if (host.tagName === 'TABLE') return Array.from(host.querySelectorAll('tbody tr'));
    return Array.from(host.querySelectorAll('tr'));
  };

  const fire = (el, type) => {
    el.dispatchEvent(new win.Event(type, { bubbles: true, cancelable: true }));
  };
  const type = (el, value) => {
    el.value = value;
    fire(el, 'input');
    fire(el, 'change');
  };
  const submit = (form) => {
    fire(form, 'submit');
  };
  const click = (el) => {
    el.dispatchEvent(new win.MouseEvent('click', { bubbles: true, cancelable: true }));
  };
  const confirmModal = async () => {
    await sleep(60);
    const ok = doc.querySelector('.modal-backdrop.open [data-role="ok"]');
    if (!ok) return false;
    ok.click();
    await sleep(560);
    return true;
  };

  return { dom, win, doc, $, text, all, rows, fire, type, submit, click, confirmModal, pageErrors, sleep };
}

/* ------------------------------------------------------------------ pages */

async function verifyLogin() {
  const p = await load('login.html');
  check('login: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  // 空提交 → invalid_credentials 分支
  p.$('#login-form').querySelector('button[type="submit"]').click();
  await p.sleep(60);
  check('login: 空提交显示 invalid_credentials 文案',
    p.text('#login-error') === 'Invalid username or password', p.text('#login-error'));

  // superadmin 演示入口 → JWT roles 含 GM_ADMIN
  p.doc.querySelector('[data-demo="superadmin"]').click();
  await p.sleep(80);
  check('login: superadmin 映射出 GM_ADMIN',
    p.text('#s-roles').includes('GM_ADMIN'), p.text('#s-roles'));
  check('login: 登录后会话卡片可见', p.$('#session-card').hidden === false);
  check('login: 导航显示用户名', p.text('#gm-topbar').includes('sre_lead'));

  // admin 演示入口 → 无 GM_ADMIN
  p.doc.querySelector('[data-demo="admin"]').click();
  await p.sleep(80);
  check('login: admin 映射不含 GM_ADMIN',
    !p.text('#s-roles').includes('GM_ADMIN'), p.text('#s-roles'));
  p.win.close();
}

async function verifyDashboard() {
  const p = await load('dashboard.html');
  check('dashboard: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  check('dashboard: 总玩家数渲染', p.text('#m-players') === '28', p.text('#m-players'));
  check('dashboard: 活跃分片渲染', p.text('#m-servers') === '6', p.text('#m-servers'));
  check('dashboard: 分片快照 6 行', p.rows('#snapshot-body').length === 6);
  check('dashboard: 图表已绘制采样点',
    p.doc.querySelectorAll('#chart svg circle').length >= 4,
    String(p.doc.querySelectorAll('#chart svg circle').length));

  // 空消息 → warning 分支，不进列表
  const before = p.doc.querySelectorAll('#broadcast-list li').length;
  p.$('#send-broadcast').click();
  await p.sleep(60);
  check('dashboard: 空广播走 warning',
    p.text('#broadcast-status') === 'Please enter a broadcast message', p.text('#broadcast-status'));
  check('dashboard: 空广播不新增条目',
    p.doc.querySelectorAll('#broadcast-list li').length === before);

  // 有消息 → 进列表且计数 +1
  const totalBefore = Number(p.text('#m-broadcasts').replace(/,/g, ''));
  p.type(p.$('#broadcast-input'), 'Prototype check broadcast');
  p.$('#send-broadcast').click();
  await p.sleep(80);
  check('dashboard: 广播发送成功', p.text('#broadcast-status') === 'ok', p.text('#broadcast-status'));
  check('dashboard: 广播置顶显示',
    p.$('#broadcast-list').textContent.includes('Prototype check broadcast'));
  check('dashboard: 广播计数 +1',
    Number(p.text('#m-broadcasts').replace(/,/g, '')) === totalBefore + 1);

  // SSE 模拟推送（#broadcast-list 是 <ul>，不是表格，用 li 计数）
  const listLen = p.doc.querySelectorAll('#broadcast-list li').length;
  p.$('#simulate-sse').click();
  await p.sleep(60);
  check('dashboard: SSE 推送置顶',
    p.doc.querySelectorAll('#broadcast-list li').length === Math.min(10, listLen + 1),
    listLen + ' → ' + p.doc.querySelectorAll('#broadcast-list li').length);
  p.win.close();
}

async function verifyPlayers() {
  const p = await load('players.html');
  check('players: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  check('players: 总体 28 人', p.text('#overall-total') === '28', p.text('#overall-total'));
  check('players: 首页 10 行', p.rows('#players-body').length === 10);
  check('players: 分页 Page 1 of 3', p.text('#page-label') === 'Page 1 of 3', p.text('#page-label'));

  // 排序：lastLogin 降序
  const first = p.rows('#players-body')[0];
  check('players: 默认按最后登录倒序', first.textContent.includes('Kaede Tsukimori'), first.textContent.slice(0, 40));

  // 搜索 VIP客群（按公会名）——演示数据里 VIP客群 公会共 5 人，其中 4 人在线
  p.type(p.$('#search'), 'VIP客群');
  p.submit(p.$('#filters'));
  await p.sleep(60);
  const filtered = p.rows('#players-body');
  check('players: 公会搜索命中 5 人',
    p.text('#filtered-total') === '5', p.text('#filtered-total'));
  check('players: 搜索结果全部属 VIP客群',
    filtered.every((r) => r.textContent.includes('VIP客群')),
    String(filtered.length));

  // 状态筛选（select 走 input 事件路径）
  p.type(p.$('#status-filter'), 'online');
  await p.sleep(60);
  check('players: 状态筛选叠加生效',
    p.text('#filtered-total') === '4', p.text('#filtered-total'));

  p.type(p.$('#status-filter'), 'all');
  p.type(p.$('#search'), '');
  p.submit(p.$('#filters'));
  await p.sleep(60);

  // 分页
  p.$('#next').click();
  await p.sleep(60);
  check('players: 翻页到第 2 页', p.text('#page-label') === 'Page 2 of 3', p.text('#page-label'));

  // 详情
  p.$('#players-body [data-view]').click();
  await p.sleep(80);
  check('players: 详情卡可见', p.$('#player-detail').hidden === false);
  check('players: 详情含 Player ID',
    p.text('#detail-grid').includes('Player ID'));
  p.win.close();
}

async function verifyServers() {
  const p = await load('servers.html');
  check('servers: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  check('servers: 7 个分片', p.text('#s-total') === '7', p.text('#s-total'));
  check('servers: 6 运行中', p.text('#s-running') === 'Running 6', p.text('#s-running'));
  check('servers: 1 台离线', p.text('#s-offline') === '1', p.text('#s-offline'));

  // running 分片的 Start 应置灰
  const runningRow = p.rows('#servers-body').find((r) => r.textContent.includes('running'));
  const startBtn = runningRow.querySelector('[data-action="start"]');
  check('servers: 运行中分片 Start 置灰', startBtn.disabled === true);

  // 停服必须先弹确认框
  const stopBtn = p.rows('#servers-body')[0].querySelector('[data-action="stop"]');
  stopBtn.click();
  await p.sleep(80);
  check('servers: 停服弹出二次确认', p.doc.querySelector('.modal-backdrop.open') !== null);

  const ok = p.doc.querySelector('.modal-backdrop.open [data-role="ok"]');
  ok.click();
  await p.sleep(600);
  check('servers: 确认后分片转 stopped',
    p.rows('#servers-body')[0].textContent.includes('stopped'),
    p.rows('#servers-body')[0].textContent.slice(0, 40));
  check('servers: 离线计数变为 2', p.text('#s-offline') === '2', p.text('#s-offline'));
  p.win.close();
}

async function verifyItems() {
  const p = await load('items.html');
  check('items: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  const rowsBefore = p.rows('#grants-body').length;
  check('items: 发放历史有数据', rowsBefore > 0, String(rowsBefore));

  // 空表单 → warning
  p.$('#grant').click();
  await p.sleep(60);
  check('items: 缺参走 warning',
    p.text('#grant-status') === 'Player ID and Item ID are required', p.text('#grant-status'));

  // 发放（带确认框）
  p.type(p.$('#grant-player'), '10008');
  p.type(p.$('#grant-item'), 'item_gem_600');
  p.type(p.$('#grant-amount'), '2');
  p.$('#grant').click();
  await p.sleep(80);
  check('items: 发放弹出确认', p.doc.querySelector('.modal-backdrop.open') !== null);
  p.doc.querySelector('.modal-backdrop.open [data-role="ok"]').click();
  await p.sleep(600);

  check('items: 发放成功提示', p.text('#grant-status') === 'granted', p.text('#grant-status'));
  check('items: 新记录置顶',
    p.rows('#grants-body')[0].textContent.includes('item_gem_600'),
    p.rows('#grants-body')[0].textContent.slice(0, 50));
  check('items: 表单已重置', p.$('#grant-player').value === '');

  // 过滤器
  p.type(p.$('#history-search'), 'nothing-matches');
  await p.sleep(60);
  check('items: 过滤空态文案区分正确',
    p.$('#grants-body').textContent.includes('No grants matching the filter'),
    p.$('#grants-body').textContent.trim());
  p.win.close();
}

async function verifyCanvas() {
  const p = await load('canvas.html');
  check('canvas: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  check('canvas: 默认图层名 gm_overlay', p.$('#layer').value === 'gm_overlay');

  // 校验 1：缺 Client ID
  p.$('#send').click();
  await p.sleep(60);
  check('canvas: 缺 Client ID 走 warning',
    p.text('#send-status') === 'Client ID is required', p.text('#send-status'));

  // 校验 2：宽高非法
  p.type(p.$('#client-id'), 'client-7f21');
  p.type(p.$('#width'), '0');
  p.$('#send').click();
  await p.sleep(60);
  check('canvas: 宽高非法走 warning',
    p.text('#send-status') === 'Width and height must be positive numbers', p.text('#send-status'));

  // 校验 3：无图且未勾 clear
  p.type(p.$('#width'), '640');
  p.type(p.$('#height'), '360');
  await p.sleep(60);
  p.$('#send').click();
  await p.sleep(60);
  check('canvas: 无图未勾 clear 走 warning',
    p.text('#send-status').includes('Upload an image'), p.text('#send-status'));

  // 像素计量 chips
  check('canvas: 像素计量正确',
    p.text('#chip-pixels') === 'Pixels 640 × 360', p.text('#chip-pixels'));
  check('canvas: RGBA 字节计量正确',
    p.text('#chip-bytes') === 'RGBA bytes 921.6K', p.text('#chip-bytes'));

  // 预览落点
  check('canvas: 位置预览已渲染', p.doc.querySelector('#stage .canvas-preview') !== null);

  // 勾 clear → 确认 → Clear command queued
  p.$('#clear').checked = true;
  p.fire(p.$('#clear'), 'change');
  p.$('#send').click();
  await p.sleep(80);
  check('canvas: 清屏指令弹出确认', p.doc.querySelector('.modal-backdrop.open') !== null);
  p.doc.querySelector('.modal-backdrop.open [data-role="ok"]').click();
  await p.sleep(600);
  check('canvas: 清屏指令入队成功',
    p.text('#send-status') === 'Clear command queued successfully', p.text('#send-status'));

  // 九宫格锚点双向同步
  p.doc.querySelector('[data-a="bottom_right"]').click();
  await p.sleep(60);
  check('canvas: 九宫格切换 anchor 同步下拉',
    p.$('#anchor').value === 'bottom_right', p.$('#anchor').value);
  p.win.close();
}

async function verifyMall() {
  const p = await load('mall.html');
  check('mall: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  const before = p.rows('#mall-body').length;
  check('mall: 目录 8 条', before === 8, String(before));

  // 行内编辑
  p.$('#mall-body [data-edit]').click();
  await p.sleep(60);
  check('mall: 进入行内编辑', p.$('#mall-body [data-edit="name"]') !== null);
  p.doc.querySelector('[data-edit="name"]').value = 'Renamed Bundle';
  p.doc.querySelector('[data-edit="price"]').value = '6.49';
  p.$('#mall-body [data-save]').click();
  await p.sleep(60);
  check('mall: 编辑保存成功', p.text('#status') === 'Item updated', p.text('#status'));
  check('mall: 行内容已更新', p.rows('#mall-body')[0].textContent.includes('Renamed Bundle'));

  // 删除（需确认）
  p.$('#mall-body [data-delete]').click();
  await p.sleep(80);
  check('mall: 删除弹出确认', p.doc.querySelector('.modal-backdrop.open') !== null);
  p.doc.querySelector('.modal-backdrop.open [data-role="ok"]').click();
  await p.sleep(80);
  check('mall: 删除后少一条', p.rows('#mall-body').length === before - 1, String(p.rows('#mall-body').length));

  // 新建校验
  p.$('#create').click();
  await p.sleep(60);
  check('mall: 空名称走 warning', p.text('#status') === 'Name is required', p.text('#status'));

  p.type(p.$('#name'), 'New Item');
  p.type(p.$('#price'), 'abc');
  p.$('#create').click();
  await p.sleep(60);
  check('mall: 非法价格走 warning', p.text('#status') === 'Enter a valid price', p.text('#status'));

  p.type(p.$('#price'), '3.99');
  p.$('#create').click();
  await p.sleep(500);
  check('mall: 新建成功', p.text('#status') === 'Item created', p.text('#status'));

  // 搜索空态
  p.type(p.$('#search'), 'zzz-nothing');
  await p.sleep(60);
  check('mall: 搜索空态文案',
    p.$('#mall-body').textContent.includes('No items match the current search'),
    p.$('#mall-body').textContent.trim());
  p.win.close();
}

async function verifyReports() {
  const p = await load('reports.html');
  check('reports: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  const all = p.rows('#reports-body').length;
  check('reports: 全部 7 份', all === 7, String(all));

  // 类型筛选
  p.type(p.$('#report-type'), 'Retention');
  await p.sleep(60);
  check('reports: 类型筛选生效',
    p.rows('#reports-body').length === 3, String(p.rows('#reports-body').length));
  check('reports: 筛选后计数更新',
    p.text('#count-line') === 'Showing 3 of 7 reports', p.text('#count-line'));

  // 聚合卡片随筛选变化
  const aggText = p.$('#metric-cards').textContent;
  check('reports: 聚合卡片渲染', aggText.includes('d 1 retention') || aggText.includes('d1 retention'), aggText.slice(0, 60));

  // 搜索无结果
  p.type(p.$('#report-search'), 'zzz-no-such-metric');
  await p.sleep(60);
  check('reports: 无匹配空态',
    p.$('#reports-body').textContent.includes('No reports match the current filters'),
    p.$('#reports-body').textContent.trim());

  // 详情 JSON
  p.type(p.$('#report-search'), '');
  p.type(p.$('#report-type'), 'all');
  await p.sleep(60);
  p.$('#reports-body [data-view]').click();
  await p.sleep(80);
  check('reports: 详情卡可见', p.$('#report-detail').hidden === false);
  check('reports: JSON 高亮渲染', p.$('#detail-json').textContent.includes('"id"'),
    p.$('#detail-json').textContent.slice(0, 60));
  p.win.close();
}

async function verifySupport() {
  const p = await load('support.html');
  check('support: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  check('support: 工单总数 10', p.text('#ticket-chips').includes('Total 10'), p.text('#ticket-chips'));

  // 状态筛选
  p.type(p.$('#ticket-status'), 'resolved');
  await p.sleep(60);
  check('support: 状态筛选生效',
    p.$('#ticket-chips').textContent.includes('Total 3'), p.text('#ticket-chips'));

  p.type(p.$('#ticket-status'), 'all');
  await p.sleep(60);

  // 流转 open → resolved
  const resolveBtn = p.$('#tickets-body [data-set="resolved"]');
  check('support: 未 resolved 的工单显示 Resolve', resolveBtn !== null);
  resolveBtn.click();
  await p.sleep(60);
  check('support: 状态流转提示',
    p.text('#send-status').includes('updated to resolved'), p.text('#send-status'));
  check('support: resolved 计数 +1',
    p.text('#ticket-chips').includes('Resolved 4'), p.text('#ticket-chips'));

  // pending 工单不应有 Mark pending（按状态标签判断，
  // 不能用 textContent.includes('pending')——"Mark pending" 按钮文本本身含该词）
  const pendingRow = p.rows('#tickets-body').find(
    (r) => {
      const tag = r.querySelector('.status-tag');
      return tag && tag.textContent.trim() === 'pending';
    }
  );
  check('support: 存在 pending 工单', !!pendingRow);
  check('support: pending 工单隐藏 Mark pending',
    !!pendingRow && pendingRow.querySelector('[data-set="pending"]') === null);

  // 建单校验
  p.$('#send').click();
  await p.sleep(60);
  check('support: 空建单走 warning',
    p.text('#send-status') === 'Player ID and message are required', p.text('#send-status'));

  p.type(p.$('#ticket-player'), '10008');
  p.type(p.$('#ticket-message'), 'Prototype created ticket');
  p.$('#send').click();
  await p.sleep(500);
  check('support: 建单成功', p.text('#send-status') === 'sent', p.text('#send-status'));
  check('support: 新工单置顶',
    p.rows('#tickets-body')[0].textContent.includes('Prototype created ticket'));

  // 详情
  p.$('#tickets-body [data-view]').click();
  await p.sleep(80);
  check('support: 详情卡可见', p.$('#ticket-detail').hidden === false);
  p.win.close();
}

async function verifyIndex() {
  const p = await load('index.html');
  check('index: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  const coverageRows = p.doc.querySelectorAll('main table')[0].querySelectorAll('tbody tr').length;
  check('index: 路由覆盖矩阵行数 ≥ 30', coverageRows >= 30, String(coverageRows));

  // 只数覆盖矩阵里的「后端有 · 前端无」徽标，设计令牌区的示例标签不计入
  const gaps = Array.from(p.doc.querySelectorAll('.status-tag.status-pending'))
    .filter((n) => n.textContent.includes('后端有')).length;
  check('index: 标注出 7 个未接入端点', gaps === 7, String(gaps));

  const screens = p.doc.querySelectorAll('a.metric-card[href$=".html"]').length;
  check('index: 9 屏入口齐全', screens === 9, String(screens));

  // 契约漂移矩阵必须与正文声称的处数一致（防止改表忘改正文）
  const driftTable = Array.from(p.doc.querySelectorAll('table'))
    .find((t) => {
      const cap = t.querySelector('caption');
      return cap && cap.textContent.includes('前端 interface vs 后端 struct');
    });
  check('index: 契约漂移矩阵存在', !!driftTable);
  const driftRows = driftTable ? driftTable.querySelectorAll('tbody tr').length : 0;
  check('index: 契约漂移 13 行', driftRows === 13, String(driftRows));

  const driftClaim = p.text('main');
  check('index: 正文声称的漂移处数与矩阵行数一致',
    driftClaim.includes('13 处契约不一致') && driftRows === 13);

  // 13 行必须逐行标记为已修复（防止改了后端却忘了更新矩阵）
  const driftCells = driftTable
    ? Array.from(driftTable.querySelectorAll('tbody tr')).map((r) => r.textContent)
    : [];
  const unfixed = driftCells.filter((t) => !t.includes('已修复'));
  check('index: 漂移矩阵 13 行全部标记已修复',
    unfixed.length === 0, unfixed.length + ' 行未标记');

  // 硬数字：路由数必须是 32
  check('index: 路由数标注为 32',
    driftClaim.includes('32 条后端路由') && driftClaim.includes('共 32 条'),
    '未找到 32 的标注');
  p.win.close();
}

/* ==========================================================================
   消费者端 storefront 套件（docs/prototype/storefront）
   ========================================================================== */

async function verifyStorefrontIndex() {
  const p = await load('index.html', STOREFRONT_ROOT);
  check('storefront/index: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  const body = p.text('body');
  check('storefront/index: 声明 12 条契约路径', body.includes('12 条契约路径'), '未找到');
  check('storefront/index: 列出 BFF 不代理的结论', body.includes('BFF 不代理'), '未找到');
  check('storefront/index: 记录 /v1/sessions/line 只在 identity',
    body.includes('commerce-identity') && body.includes('只有它有'), '未找到');
  check('storefront/index: 记录路径落在 5 个独立进程',
    body.includes('5 个独立进程'), '未找到');
  check('storefront/index: 记录桩/真两套实现',
    body.includes('桩') && body.includes('两套实现'), '未找到');
  check('storefront/index: 记录电商线字段漂移',
    body.includes('channel_eligibility') && body.includes('title_i18n'), '未找到');
  check('storefront/index: 记录订单金额显示 ¥0 的后果',
    body.includes('订单行与订单总额显示 ¥0'), '未找到');

  const screens = p.all('.screen-card').length;
  check('storefront/index: 8 屏入口齐全', screens === 8, String(screens));
  p.win.close();
}

async function verifyStorefrontCatalog() {
  const p = await load('catalog.html', STOREFRONT_ROOT);
  check('storefront/catalog: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  const cards = p.all('#list .pcard');
  check('storefront/catalog: 首屏渲染商品卡', cards.length > 0, String(cards.length));
  check('storefront/catalog: 卡片含价格', cards[0].textContent.includes('¥'), cards[0].textContent.slice(0, 40));
  check('storefront/catalog: 缺货商品按钮为「不可 / N/A」',
    p.$('#list').textContent.includes('不可 / N/A'), '未找到');

  // 渠道过滤：LINE 下 prd-1004（line_mini_app:false）不出现
  p.click(p.$('#channel-slot [data-channel="line"]'));
  await p.sleep(60);
  const lineText = p.text('#list');
  check('storefront/catalog: LINE 渠道隐藏非 LINE 商品',
    !lineText.includes('限定トートバッグ'), 'LINE 下仍出现');
  // LINE 限定商品排在第 7 位，先翻页
  p.click(p.$('#load-more'));
  await p.sleep(60);
  check('storefront/catalog: LINE 渠道显示 LINE 限定商品',
    p.text('#list').includes('LINE 限定トート'), 'LINE 限定商品缺失');

  // 回到 Web：LINE 限定商品应消失
  p.click(p.$('#channel-slot [data-channel="web"]'));
  await p.sleep(60);
  check('storefront/catalog: Web 渠道隐藏 LINE 限定商品',
    !p.text('#list').includes('LINE 限定トート'), 'Web 下仍出现');

  // 游客横幅
  p.click(p.$('#sim-guest'));
  await p.sleep(60);
  check('storefront/catalog: 游客态显示降级横幅',
    p.text('#banner-slot').includes('ゲスト表示'), p.text('#banner-slot').slice(0, 40));

  // 加载失败 → 错误态渲染在列表区（保留既有列表的语义）
  p.click(p.$('#sim-guest'));
  p.click(p.$('#sim-error'));
  await p.sleep(60);
  check('storefront/catalog: 失败态给出重试',
    p.text('#list').includes('再試行'), p.text('#list').slice(0, 40));

  // 空态
  p.click(p.$('#sim-error'));
  p.click(p.$('#sim-empty'));
  await p.sleep(60);
  check('storefront/catalog: 空态文案',
    p.text('#list').includes('商品がありません'), p.text('#list').slice(0, 40));
  p.win.close();
}

async function verifyStorefrontProduct() {
  const p = await load('product.html', STOREFRONT_ROOT);
  check('storefront/product: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  check('storefront/product: 显示权威价格', p.text('#p-price').includes('¥'), p.text('#p-price'));
  check('storefront/product: 显示价格版本', p.text('#p-chips').includes('v'), p.text('#p-chips'));
  check('storefront/product: 渠道资格表存在',
    p.text('#p-table').includes('LINE MINI App'), p.text('#p-table').slice(0, 40));
  check('storefront/product: 购买按钮可用', p.$('#add-cart').disabled === false);

  p.click(p.$('#bump-version'));
  await p.sleep(60);
  check('storefront/product: 版本可推进', p.text('#p-chips').includes('v8'), p.text('#p-chips'));

  // 切到 LINE：prd-1001 三渠道都可售，按钮仍可用
  p.click(p.$('[data-channel="line"]'));
  await p.sleep(60);
  check('storefront/product: LINE 渠道下三渠道商品仍可购买',
    p.$('#add-cart').disabled === false, '按钮被禁用');
  p.win.close();
}

async function verifyStorefrontCart() {
  const p = await load('cart.html', STOREFRONT_ROOT);
  check('storefront/cart: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  const lines = p.all('#cart-lines .line-item');
  check('storefront/cart: 两行商品', lines.length === 2, String(lines.length));
  check('storefront/cart: 小计 = 快照合计', p.text('#cart-subtotal').includes('¥4580'), p.text('#cart-subtotal'));
  check('storefront/cart: 版本号显示', p.text('#cart-version').includes('v6'), p.text('#cart-version'));

  // 数量 +1 → 版本 +1
  p.click(p.$('#cart-lines [data-inc]'));
  await p.sleep(60);
  check('storefront/cart: 改数量后版本递增', p.text('#cart-version').includes('v7'), p.text('#cart-version'));

  // 旧版本 → 409 冲突横幅
  p.click(p.$('#stale'));
  await p.sleep(60);
  check('storefront/cart: 版本冲突给出 409 提示',
    p.text('#banner-slot').includes('409'), p.text('#banner-slot').slice(0, 50));
  p.win.close();
}

async function verifyStorefrontCheckout() {
  const p = await load('checkout.html', STOREFRONT_ROOT);
  check('storefront/checkout: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  check('storefront/checkout: 状态机时间线渲染',
    p.all('#fsm .fsm__step').length === 7, String(p.all('#fsm .fsm__step').length));

  // 切到 payment_unknown
  p.click(p.$('[data-state="payment_unknown"]'));
  await p.sleep(80);
  const bottom = p.text('#bottom-bar');
  check('storefront/checkout: payment_unknown 文案不说失败',
    bottom.includes('お支払いを確認中です') && !bottom.includes('処理できませんでした'), bottom.slice(0, 60));
  check('storefront/checkout: 明确禁止重提',
    bottom.includes('再送はしません'), bottom.slice(0, 80));
  check('storefront/checkout: 轮询卡片可见', p.$('#poll-card').hidden === false);
  check('storefront/checkout: 轮询次数可见', p.text('#poll-chips').includes('poll'), p.text('#poll-chips'));

  // 刷新（轮询）→ 状态前进，不回退
  const before = p.text('#fsm');
  p.click(p.$('#bottom-bar [data-action="poll"]'));
  await p.sleep(80);
  check('storefront/checkout: 轮询后状态变化', p.text('#fsm') !== before, '状态未变化');

  // confirmed
  p.click(p.$('[data-state="confirmed"]'));
  await p.sleep(80);
  check('storefront/checkout: confirmed 提示成功',
    p.text('#bottom-bar').includes('ご注文を完了しました'), p.text('#bottom-bar').slice(0, 40));
  p.win.close();
}

async function verifyStorefrontOrder() {
  const p = await load('order.html', STOREFRONT_ROOT);
  check('storefront/order: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  const trail = p.all('#trail li');
  check('storefront/order: 事件轨迹 6 条', trail.length === 6, String(trail.length));
  check('storefront/order: 轨迹含状态跃迁',
    p.text('#trail').includes('payment_unknown'), p.text('#trail').slice(0, 60));
  check('storefront/order: 订单总额正确', p.text('#order-total').includes('¥4580'), p.text('#order-total'));
  check('storefront/order: Outbox 列出', p.all('#outbox li').length === 2, String(p.all('#outbox li').length));
  p.win.close();
}

async function verifyStorefrontSession() {
  const p = await load('session.html', STOREFRONT_ROOT);
  check('storefront/session: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  check('storefront/session: 游客不能结算',
    p.text('#session-kv').includes('canCheckout'), p.text('#session-kv').slice(0, 60));
  check('storefront/session: 游客态为 false',
    p.text('#session-chips').includes('canCheckout=false'), p.text('#session-chips'));

  // 切到 LINE 登录成功
  p.click(p.$('[data-outcome="line_ok"]'));
  await p.sleep(60);
  check('storefront/session: LINE 登录后 canCheckout=true',
    p.text('#session-chips').includes('canCheckout=true'), p.text('#session-chips'));
  check('storefront/session: 顶部显示已登录', p.text('#appbar-sub').includes('サインイン済'), p.text('#appbar-sub'));

  // LIFF-003 SDK 失败
  p.click(p.$('[data-outcome="liff_unavailable"]'));
  await p.sleep(60);
  check('storefront/session: SDK 失败有降级提示',
    p.text('#banner-slot').includes('LINE 機能を読み込めませんでした'), p.text('#banner-slot').slice(0, 50));
  p.win.close();
}

async function verifyStorefrontProblem() {
  const p = await load('problem.html', STOREFRONT_ROOT);
  check('storefront/problem: 页面无脚本错误', p.pageErrors.length === 0, p.pageErrors.join(' | '));

  check('storefront/problem: 显示 problem+json',
    p.text('#p-json').includes('traceId'), p.text('#p-json').slice(0, 60));
  check('storefront/problem: payment.unknown 命中 isUnresolvedPayment',
    p.text('#branch-list').includes('isUnresolvedPayment'), '未命中');
  check('storefront/problem: 提供轮询动作而非重提',
    p.text('#p-actions').includes('再送（禁止）'), p.text('#p-actions').slice(0, 60));

  p.click(p.$('[data-problem="version_conflict"]'));
  await p.sleep(60);
  check('storefront/problem: 切到版本冲突后命中 isVersionConflict',
    p.text('#branch-list').includes('isVersionConflict'), '未命中');
  p.win.close();
}

/* ------------------------------------------------------------------- main */

const SUITES = [
  ['admin-index', verifyIndex],
  ['login', verifyLogin],
  ['dashboard', verifyDashboard],
  ['players', verifyPlayers],
  ['servers', verifyServers],
  ['items', verifyItems],
  ['canvas', verifyCanvas],
  ['mall', verifyMall],
  ['reports', verifyReports],
  ['support', verifySupport],
  ['storefront-index', verifyStorefrontIndex],
  ['storefront-catalog', verifyStorefrontCatalog],
  ['storefront-product', verifyStorefrontProduct],
  ['storefront-cart', verifyStorefrontCart],
  ['storefront-checkout', verifyStorefrontCheckout],
  ['storefront-order', verifyStorefrontOrder],
  ['storefront-session', verifyStorefrontSession],
  ['storefront-problem', verifyStorefrontProblem]
];

const only = process.argv[2];
for (const [name, fn] of SUITES) {
  if (only && only !== name) continue;
  process.stdout.write('▶ ' + name + '\n');
  try {
    await fn();
  } catch (e) {
    failures.push(name + ': 抛出异常 → ' + (e && e.stack ? e.stack.split('\n')[0] : e));
  }
}

process.stdout.write('\n────────────────────────────────\n');
process.stdout.write('通过 ' + passed + ' 项，失败 ' + failures.length + ' 项\n');
if (failures.length) {
  process.stdout.write('\n失败明细：\n');
  failures.forEach((f) => process.stdout.write('  ✗ ' + f + '\n'));
  process.exit(1);
}
process.stdout.write('全部通过。\n');