/* ==========================================================================
   DOKIPON 运营后台 — prototype shell
   --------------------------------------------------------------------------
   Vanilla JS on purpose: the prototype must open from file:// with no build
   step and no server. Behaviour mirrors tools/gm-console/frontend/src:

     storage  → App.tsx readStorage/writeStorage (try/catch, swallow errors)
     auth     → App.tsx login()/logout() + localStorage keys gm.authToken / gm.username
     nav      → components/NavBar.tsx route order（标签文案改为电商口径，路由不变）
     roles    → crates/gm-backend/src/auth_handler.rs issue_jwt() role mapping

   本文件只改文案（品牌名、导航标签、横幅、页脚），结构与行为保持原样：
   状态类名（status-*）、chip 类名（chip--doc / chip--gap / chip--demo）、
   localStorage 键名与 JWT 角色映射都是契约，不动。
   ========================================================================== */

window.GM = (function () {
  'use strict';

  var TOKEN_KEY = 'gm.authToken';
  var USER_KEY = 'gm.username';
  var ROLE_KEY = 'gm.role';

  /* 导航标签沿用电商口径的中文名；href / id 仍镜像后端端点族（/gm/players …），
     不随界面文案改名——文件与端点的对应关系见 index.html 的改造对照表。 */
  var NAV = [
    { id: 'dashboard', label: '经营总览', href: 'dashboard.html' },
    { id: 'players',   label: '消费者',   href: 'players.html' },
    { id: 'servers',   label: '服务实例', href: 'servers.html' },
    { id: 'items',     label: '补偿发放', href: 'items.html' },
    { id: 'canvas',    label: '营销下发', href: 'canvas.html' },
    { id: 'mall',      label: '店铺商品', href: 'mall.html' },
    { id: 'reports',   label: '经营报表', href: 'reports.html' },
    { id: 'support',   label: '售后工单', href: 'support.html' }
  ];

  /* ------------------------------------------------------------- storage -- */
  /* file:// origins are opaque in some browsers — every access is guarded and
     falls back to an in-memory store, exactly like App.tsx. */

  var memory = {};

  function read(key) {
    try {
      return window.localStorage.getItem(key) || memory[key] || '';
    } catch (e) {
      return memory[key] || '';
    }
  }

  function write(key, value) {
    memory[key] = value;
    try {
      window.localStorage.setItem(key, value);
    } catch (e) {
      /* private mode / quota — in-memory copy already set */
    }
  }

  function drop(key) {
    delete memory[key];
    try {
      window.localStorage.removeItem(key);
    } catch (e) {
      /* ignore */
    }
  }

  /* --------------------------------------------------------------- auth -- */

  function session() {
    var token = read(TOKEN_KEY);
    if (!token) return null;
    return {
      token: token,
      username: read(USER_KEY) || 'gm_operator',
      role: read(ROLE_KEY) || 'admin'
    };
  }

  function isSuperadmin() {
    var s = session();
    return !!s && s.role === 'superadmin';
  }

  function signIn(username, role) {
    // auth_handler.rs: superadmin → GM_READ + GM_WRITE + GM_ADMIN,
    // otherwise → GM_READ + GM_WRITE. Token TTL is 3600s.
    write(TOKEN_KEY, 'proto.' + Math.random().toString(36).slice(2) + '.' + Date.now());
    write(USER_KEY, username);
    write(ROLE_KEY, role);
  }

  function signOut() {
    drop(TOKEN_KEY);
    drop(USER_KEY);
    drop(ROLE_KEY);
  }

  function roles() {
    return isSuperadmin()
      ? ['GM_READ', 'GM_WRITE', 'GM_ADMIN']
      : ['GM_READ', 'GM_WRITE'];
  }

  /* ------------------------------------------------------------- format -- */

  function esc(value) {
    if (value === null || value === undefined) return '';
    return String(value)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function fmtDateTime(value) {
    if (!value) return '—';
    var d = new Date(value);
    if (isNaN(d.getTime())) return String(value);
    return d.toLocaleString();
  }

  function fmtMoney(value) {
    if (typeof value !== 'number') return '—';
    return '$' + value.toFixed(2);
  }

  function fmtCompact(value) {
    if (!isFinite(value) || value < 0) return '—';
    if (value >= 1e6) return (value / 1e6).toFixed(1) + 'M';
    if (value >= 1e3) return (value / 1e3).toFixed(1) + 'K';
    return String(value);
  }

  function el(id) {
    return document.getElementById(id);
  }

  function statusTag(value, dot) {
    var cls = 'status-tag status-' + String(value || '').toLowerCase();
    if (dot) cls += ' status-tag--dot';
    return '<span class="' + cls + '">' + esc(value) + '</span>';
  }

  function setStatus(node, message, level) {
    if (!node) return;
    if (!message) {
      node.className = 'status-message';
      node.textContent = '';
      return;
    }
    node.className = 'status-message' + (level ? ' status-' + level : '');
    node.textContent = message;
  }

  /* -------------------------------------------------------------- modal -- */

  function modal(options) {
    return new Promise(function (resolve) {
      var backdrop = el('gm-modal-backdrop');
      if (!backdrop) {
        resolve(window.confirm(options.message || ''));
        return;
      }
      backdrop.innerHTML =
        '<div class="modal" role="dialog" aria-modal="true" aria-label="' + esc(options.title || 'Confirm') + '">' +
          '<h3>' + esc(options.title || 'Confirm') + '</h3>' +
          '<p>' + (options.html || esc(options.message || '')) + '</p>' +
          '<div class="button-row">' +
            '<button type="button" class="button-secondary" data-role="cancel">' + esc(options.cancelText || 'Cancel') + '</button>' +
            '<button type="button" data-role="ok"' + (options.danger ? ' class="button-danger"' : '') + '>' +
              esc(options.confirmText || 'Confirm') +
            '</button>' +
          '</div>' +
        '</div>';

      function close(result) {
        backdrop.classList.remove('open');
        backdrop.innerHTML = '';
        document.removeEventListener('keydown', onKey);
        resolve(result);
      }

      function onKey(e) {
        if (e.key === 'Escape') close(false);
      }

      backdrop.onclick = function (e) {
        if (e.target === backdrop) close(false);
      };
      backdrop.querySelector('[data-role="cancel"]').onclick = function () { close(false); };
      backdrop.querySelector('[data-role="ok"]').onclick = function () { close(true); };

      document.addEventListener('keydown', onKey);
      backdrop.classList.add('open');
      backdrop.querySelector('[data-role="ok"]').focus();
    });
  }

  /* --------------------------------------------------------- json pretty -- */

  function highlightJson(value) {
    var json = JSON.stringify(value, null, 2);
    return esc(json).replace(
      /(&quot;[^&]*?&quot;)(\s*:)?|(\b-?\d+(\.\d+)?\b)|(\b(true|false|null)\b)/g,
      function (match, str, colon, num, numFrac, bool) {
        if (str) {
          var key = colon ? ' class="k"' : ' class="s"';
          if (!colon) return '<span class="s">' + str + '</span>';
          return '<span class="k">' + str + '</span>' + colon;
        }
        if (num) return '<span class="n">' + num + (numFrac ? numFrac : '') + '</span>';
        if (bool) return '<span class="b">' + bool + '</span>';
        return match;
      }
    );
  }

  /* -------------------------------------------------------------- chart -- */
  /* Stands in for components/ActiveUsersChart.tsx: a rolling 20-sample window
     fed by /gm/metrics (scraped with /online_connections\s+(\d+)/ every 5s). */

  function lineChart(container, options) {
    options = options || {};
    var points = options.points || [];
    var color = options.color || '#4bc0c0';
    var height = options.height || 190;
    var id = container.id || 'chart';

    function render() {
      if (!points.length) {
        container.innerHTML =
          '<div class="chart-frame"><p class="muted" style="padding:40px 0;text-align:center">' +
          'Waiting for <span class="mono">online_connections</span> samples…</p></div>';
        return;
      }

      var w = 760;
      var padL = 44;
      var padR = 14;
      var padT = 14;
      var padB = 26;
      var innerW = w - padL - padR;
      var innerH = height - padT - padB;

      var ys = points.map(function (p) { return p.y; });
      var max = Math.max.apply(null, ys);
      var min = Math.min.apply(null, ys);
      var span = max - min || 1;
      var top = max + span * 0.25;
      var bottom = Math.max(0, min - span * 0.25);

      function x(i) {
        return padL + (points.length === 1 ? innerW / 2 : (i / (points.length - 1)) * innerW);
      }
      function y(v) {
        return padT + innerH - ((v - bottom) / (top - bottom)) * innerH;
      }

      var line = points.map(function (p, i) {
        return (i ? 'L' : 'M') + x(i).toFixed(1) + ' ' + y(p.y).toFixed(1);
      }).join(' ');

      var area = line +
        ' L' + x(points.length - 1).toFixed(1) + ' ' + (padT + innerH) +
        ' L' + x(0).toFixed(1) + ' ' + (padT + innerH) + ' Z';

      var grid = '';
      for (var g = 0; g <= 3; g++) {
        var value = top - ((top - bottom) / 3) * g;
        var gy = y(value).toFixed(1);
        grid +=
          '<line x1="' + padL + '" y1="' + gy + '" x2="' + (w - padR) + '" y2="' + gy +
          '" stroke="#ececf3" stroke-width="1"/>' +
          '<text x="' + (padL - 8) + '" y="' + (parseFloat(gy) + 4) +
          '" text-anchor="end" font-size="10" fill="#8a8fa6">' + fmtCompact(Math.round(value)) + '</text>';
      }

      var dots = points.map(function (p, i) {
        var last = i === points.length - 1;
        return '<circle cx="' + x(i).toFixed(1) + '" cy="' + y(p.y).toFixed(1) + '" r="' +
          (last ? 4 : 2) + '" fill="' + color + '"' + (last ? ' stroke="#fff" stroke-width="2"' : '') + '/>';
      }).join('');

      var lastPoint = points[points.length - 1];
      var labelX = Math.min(x(points.length - 1), w - padR - 46);

      container.innerHTML =
        '<div class="chart-frame">' +
          '<svg viewBox="0 0 ' + w + ' ' + height + '" preserveAspectRatio="none" role="img" ' +
            'aria-label="online_connections over the last samples">' +
            '<defs><linearGradient id="' + id + '-fill" x1="0" y1="0" x2="0" y2="1">' +
              '<stop offset="0%" stop-color="' + color + '" stop-opacity="0.28"/>' +
              '<stop offset="100%" stop-color="' + color + '" stop-opacity="0.02"/>' +
            '</linearGradient></defs>' +
            grid +
            '<path d="' + area + '" fill="url(#' + id + '-fill)"/>' +
            '<path d="' + line + '" fill="none" stroke="' + color + '" stroke-width="2" ' +
              'stroke-linejoin="round" stroke-linecap="round"/>' +
            dots +
            '<text x="' + labelX + '" y="' + (y(lastPoint.y) - 12).toFixed(1) +
              '" text-anchor="end" font-size="11" font-weight="700" fill="#1d1f33">' +
              lastPoint.y.toLocaleString() + '</text>' +
            '<text x="' + padL + '" y="' + (height - 6) + '" font-size="10" fill="#8a8fa6">' +
              '−' + (points.length * 5) + 's</text>' +
            '<text x="' + (w - padR) + '" y="' + (height - 6) + '" text-anchor="end" ' +
              'font-size="10" fill="#8a8fa6">now</text>' +
          '</svg>' +
          '<div class="chart-legend">' +
            '<span><i></i>online_connections</span>' +
            '<span>sample window ' + points.length + '/20 · poll 5s</span>' +
          '</div>' +
        '</div>';
    }

    render();
    return { render: render };
  }

  /* -------------------------------------------------------------- shell -- */

  function shell(options) {
    options = options || {};
    var host = el('gm-topbar');
    if (!host) return;

    var s = session();
    var links = NAV.map(function (item) {
      var active = item.id === options.active ? ' class="active"' : '';
      return '<a href="' + item.href + '"' + active + '>' + item.label + '</a>';
    }).join('');

    host.className = 'topbar';
    host.innerHTML =
      '<a class="brand" href="index.html">' +
        '<span class="brand__mark">GM</span>' +
        '<span>运营后台（电商）</span>' +
        '<span class="brand__sub">prototype</span>' +
      '</a>' +
      '<nav class="nav">' + links +
        '<a href="index.html"' + (options.active === 'index' ? ' class="active"' : '') + '>原型索引</a>' +
      '</nav>' +
      (s
        ? '<div class="nav-user">' +
            '<span class="who"><span class="who__dot">' + esc(s.username.slice(0, 1).toUpperCase()) + '</span>' +
              esc(s.username) + '</span>' +
            '<span class="chip chip--demo">' + esc(s.role) + '</span>' +
            '<button type="button" class="button-secondary button-sm" id="gm-logout">Logout</button>' +
          '</div>'
        : '<div class="nav-user"><a class="chip chip--doc" href="login.html">未登录 · 前往登录页</a></div>');

    var logout = el('gm-logout');
    if (logout) {
      logout.onclick = function () {
        signOut();
        window.location.href = 'login.html';
      };
    }

    var banner = el('gm-demo-banner');
    if (banner) {
      if (s) {
        banner.innerHTML =
          '<div class="banner banner--doc">' +
            '<div><strong>原型会话</strong>' +
            esc(s.username) + ' · ' + esc(s.role) + ' · 角色映射 ' +
            roles().map(function (r) { return '<span class="mono">' + r + '</span>'; }).join(' / ') +
            '（对应 <span class="mono">auth_handler.rs</span> issue_jwt 的 role→roles 映射）。' +
            '页面内所有数字为内置演示数据，不来自任何运行中的服务。' +
            '界面文案已按电商口径改写，接口字段名仍沿用游戏服务器词汇（playerId / vipLevel / guild / mall）。' +
            '</div>' +
          '</div>';
      } else {
        banner.innerHTML =
          '<div class="banner banner--demo">' +
            '<div><strong>原型演示模式 · 未登录</strong>' +
            '真实构建中 <span class="mono">App.tsx</span> 对未登录用户一律渲染 Login 屏（<span class="mono">* → Login</span>）。' +
            '原型为便于走查放行全部页面，此处所有数据均为内置演示值。' +
            '界面文案已按电商口径改写，接口字段名仍沿用游戏服务器词汇（playerId / vipLevel / guild / mall）。' +
            ' <a href="login.html">前往登录页</a>' +
            '</div>' +
          '</div>';
      }
    }
  }

  /* ------------------------------------------------------------- footer -- */

  function footer(options) {
    options = options || {};
    var host = el('gm-footer');
    if (!host) return;
    host.className = 'footer-note';
    host.innerHTML =
      '<span>' + (options.doc ? '对应设计文档: ' + options.doc : '') + '</span>' +
      '<span><a href="index.html">原型索引</a> · ' +
      '<a href="https://github.com/UlyssesLeoLee/RustGameServer" target="_blank" rel="noopener">代码仓库 / Source repo</a></span>';
  }

  /* --------------------------------------------------------------- boot -- */

  function ready(fn) {
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', fn);
    } else {
      fn();
    }
  }

  return {
    shell: shell,
    footer: footer,
    ready: ready,
    esc: esc,
    session: session,
    signIn: signIn,
    signOut: signOut,
    roles: roles,
    fmtDateTime: fmtDateTime,
    fmtMoney: fmtMoney,
    fmtCompact: fmtCompact,
    statusTag: statusTag,
    setStatus: setStatus,
    modal: modal,
    highlightJson: highlightJson,
    lineChart: lineChart
  };
})();