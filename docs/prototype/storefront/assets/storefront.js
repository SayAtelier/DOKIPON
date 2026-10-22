/* ==========================================================================
   GemArc Commerce — storefront prototype runtime
   --------------------------------------------------------------------------
   Pure static, file:// safe, no network. Every shape below is copied from the
   real implementation:

     domain_models/lib/src/{money,product,checkout,session,problem}.dart
     storefront_state/lib/src/{catalog,checkout,session}_controller.dart
     design_system/lib/src/widgets/*.dart
     contracts/openapi.yaml
     services/crates/commerce-{catalog,cart,order,bff}/src/main.rs

   Where the server and the client disagree on a field name, the prototype
   renders the **client-expected** shape and records the disagreement in
   index.html §「真实实现现状」. That is the honest way to show a design.
   ========================================================================== */

window.SF = (function () {
  'use strict';

  /* ------------------------------------------------------------- money -- */
  /* money.dart: Currency.exponent / decimalDigits; JPY has no minor unit. */
  var CURRENCY = {
    JPY: { code: 'JPY', exponent: 0, symbol: '¥', digits: 0 },
    USD: { code: 'USD', exponent: 2, symbol: '$', digits: 2 },
    CNY: { code: 'CNY', exponent: 2, symbol: 'CN¥', digits: 2 },
    TWD: { code: 'TWD', exponent: 2, symbol: 'NT$', digits: 2 },
    EUR: { code: 'EUR', exponent: 2, symbol: '€', digits: 2 }
  };

  /** Money.format() — money.dart L87-L95. JPY prints without separators. */
  function money(amountMinor, currencyCode) {
    var c = CURRENCY[currencyCode] || CURRENCY.JPY;
    if (c.digits === 0) return c.symbol + amountMinor;
    var whole = Math.trunc(amountMinor / 100);
    var frac = String(Math.abs(amountMinor % 100)).padStart(2, '0');
    return c.symbol + whole + '.' + frac;
  }

  /* -------------------------------------------------------- availability */
  /* availability_badge.dart: labelFor() / colorFor() */
  var AVAILABILITY = {
    in_stock: { label: '在庫あり / In stock', tone: 'in_stock' },
    low_stock: { label: '残りわずか / Low stock', tone: 'low_stock' },
    out_of_stock: { label: '在庫切れ / Out of stock', tone: 'out_of_stock' },
    made_to_order: { label: '受注生産 / Made to order', tone: 'made_to_order' }
  };

  function availabilityBadge(availability, dense) {
    var a = AVAILABILITY[availability] || AVAILABILITY.in_stock;
    return '<span class="badge badge--' + a.tone + (dense ? ' badge--dense' : '') + '">' +
      a.label + '</span>';
  }

  function isPurchasable(availability) {
    return availability !== 'out_of_stock';
  }

  /* -------------------------------------------------------- status banner */
  /* status_banner.dart: BannerTone info/warning/danger/success + icons */
  var TONE_ICON = { info: 'ℹ', warning: '⚠', danger: '✕', success: '✓' };

  function banner(message, tone, actionLabel, actionId) {
    return '<div class="banner banner--' + (tone || 'info') + '">' +
      '<span class="banner__icon" aria-hidden="true">' + (TONE_ICON[tone] || 'ℹ') + '</span>' +
      '<div class="banner__body">' + message + '</div>' +
      (actionLabel
        ? '<button type="button" class="banner__action" data-action="' + (actionId || '') + '">' +
          actionLabel + '</button>'
        : '') +
      '</div>';
  }

  /* ------------------------------------------------------------ checkout */
  /* checkout.dart: CheckoutStatus — wire names, isTerminal, isPollable */
  var CHECKOUT_STATES = [
    { wire: 'new', label: '新規 / New', note: '注文がまだ作成されていません' },
    { wire: 'reserving', label: '在庫確保中 / Reserving', note: '在庫リザーブを要求中' },
    { wire: 'stock_reserved', label: '在庫確保済み / Stock reserved', note: 'リザーブ確定、決済へ' },
    { wire: 'awaiting_payment', label: '決済待ち / Awaiting payment', note: '決済プロバイダへの応答待ち' },
    { wire: 'payment_unknown', label: '決済結果不明 / Payment unknown', note: 'プロバイダ未応答。**再提出は禁止**' },
    { wire: 'confirming', label: '確認中 / Confirming', note: '注文確定の検証中' },
    { wire: 'confirmed', label: '注文確定 / Confirmed', note: '支払完了、注文番号発行' },
    { wire: 'rejected', label: '拒否 / Rejected', note: '決済が拒否された' },
    { wire: 'cancelled', label: 'キャンセル / Cancelled', note: '顧客の意思で注文を取り消し' },
    { wire: 'unknown', label: '不明 / Unknown', note: '未知の状態。注文一覧を確認' }
  ];

  var TERMINAL = ['confirmed', 'rejected', 'cancelled'];
  var POLLABLE = ['awaiting_payment', 'payment_unknown', 'confirming'];

  function isTerminal(wire) { return TERMINAL.indexOf(wire) !== -1; }
  function isPollable(wire) { return POLLABLE.indexOf(wire) !== -1; }
  function isPending(wire) { return !isTerminal(wire) && isPollable(wire); }

  /**
   * CheckoutStatusBar.describe() — the exact wording the real client uses
   * (storefront_app.dart L253-L280). Never says "failed" for payment_unknown
   * and never invites the user to pay again.
   */
  var CHECKOUT_MESSAGE = {
    confirmed: ['ご注文を完了しました / Order confirmed.', 'success'],
    rejected: ['ご注文を処理できませんでした / We could not process this order.', 'danger'],
    payment_unknown: [
      'お支払いを確認中です。Charged at most once — we are confirming with your payment provider.',
      'warning'
    ],
    cancelled: ['ご注文はキャンセルされました / Order cancelled.', 'warning'],
    unknown: [
      'ステータス不明 — 注文一覧をご確認ください / Status unknown — please check your orders.',
      'warning'
    ]
  };

  function checkoutMessage(wire) {
    if (CHECKOUT_MESSAGE[wire]) return CHECKOUT_MESSAGE[wire];
    return ['処理中… / Processing…', 'info'];
  }

  /* ---------------------------------------------------------------- data */
  /* Server-shaped rows. product_id / amount_minor / channel_eligibility are
     the **server** names; the client reads productId / price{amountMinor} /
     channelEligibility. See index.html for the full comparison. */

  var PRODUCTS = [
    {
      product_id: 'prd-1001', sku: 'sku-1001',
      title: '産地直送 スペシャルコーヒー 200g',
      title_i18n: {
        ja: '産地直送 スペシャルコーヒー 200g',
        en: 'Single-origin specialty coffee 200g'
      },
      description_i18n: {
        ja: '焙煎したての豆を 200g で。ご注文後に焙煎して発送します。',
        en: 'Roasted to order. We roast after your order arrives, then ship.'
      },
      amount_minor: 1800, currency: 'JPY', availability: 'in_stock',
      category_id: 'cat-coffee', stock_policy: 'reserved', version: 7,
      channel_eligibility: { web: true, app: true, line_mini_app: true }
    },
    {
      product_id: 'prd-1002', sku: 'sku-1002',
      title: '真空保温ボトル 500ml',
      title_i18n: {
        ja: '真空保温ボトル 500ml',
        en: 'Vacuum flask 500ml'
      },
      description_i18n: {
        ja: '12時間保温。LINE での共有にも対応しています。',
        en: 'Keeps drinks hot for 12 hours. Shareable to LINE.'
      },
      amount_minor: 2400, currency: 'JPY', availability: 'in_stock',
      category_id: 'cat-goods', stock_policy: 'reserved', version: 3,
      channel_eligibility: { web: true, app: true, line_mini_app: true }
    },
    {
      product_id: 'prd-1003', sku: 'sku-1003',
      title: '会員月額パス',
      title_i18n: { ja: '会員月額パス', en: 'Monthly membership' },
      description_i18n: {
        ja: '毎月 980 円で会員特典を利用できます。',
        en: 'Member perks for 980 JPY per month.'
      },
      amount_minor: 980, currency: 'JPY', availability: 'low_stock',
      category_id: 'cat-membership', stock_policy: 'unbounded', version: 12,
      channel_eligibility: { web: true, app: true, line_mini_app: true }
    },
    {
      product_id: 'prd-1004', sku: 'sku-1004',
      title: '限定トートバッグ',
      title_i18n: { ja: '限定トートバッグ', en: 'Limited tote bag' },
      description_i18n: {
        ja: '在庫切れ。再入荷予定はありません。',
        en: 'Out of stock, no restock scheduled.'
      },
      amount_minor: 3200, currency: 'JPY', availability: 'out_of_stock',
      category_id: 'cat-goods', stock_policy: 'reserved', version: 5,
      channel_eligibility: { web: true, app: true, line_mini_app: false }
    },
    {
      product_id: 'prd-1005', sku: 'sku-1005',
      title: '手仕事の陶器セット',
      title_i18n: { ja: '手仕事の陶器セット', en: 'Handmade ceramics set' },
      description_i18n: {
        ja: '受注生産。2週間程度でお送りします。',
        en: 'Made to order. Ships in about two weeks.'
      },
      amount_minor: 8800, currency: 'JPY', availability: 'made_to_order',
      category_id: 'cat-craft', stock_policy: 'made_to_order', version: 2,
      channel_eligibility: { web: true, app: true, line_mini_app: true }
    },
    {
      product_id: 'prd-1006', sku: 'sku-1006',
      title: 'ギフトカード 1000円',
      title_i18n: { ja: 'ギフトカード 1000円', en: 'Gift card 1000 JPY' },
      description_i18n: { ja: '有効期限 1 年。', en: 'Valid for one year.' },
      amount_minor: 1000, currency: 'JPY', availability: 'in_stock',
      category_id: 'cat-gift', stock_policy: 'unbounded', version: 1,
      channel_eligibility: { web: true, app: true, line_mini_app: true }
    },
    {
      product_id: 'prd-1007', sku: 'sku-1007',
      title: '城堡ブレンド 紅茶 100g',
      title_i18n: { ja: '城堡ブレンド 紅茶 100g', en: 'Castle blend black tea 100g' },
      description_i18n: {
        ja: '香りの強い城堡ブレンド。',
        en: 'A strong, malty castle blend.'
      },
      amount_minor: 1200, currency: 'JPY', availability: 'in_stock',
      category_id: 'cat-tea', stock_policy: 'reserved', version: 4,
      channel_eligibility: { web: true, app: true, line_mini_app: true }
    },
    {
      product_id: 'prd-1008', sku: 'sku-1008',
      title: 'LINE 限定トート',
      title_i18n: { ja: 'LINE 限定トート', en: 'LINE exclusive tote' },
      description_i18n: {
        ja: 'LINE でのみ販売しています。',
        en: 'Sold only inside LINE.'
      },
      amount_minor: 1500, currency: 'JPY', availability: 'in_stock',
      category_id: 'cat-goods', stock_policy: 'reserved', version: 2,
      channel_eligibility: { web: false, app: false, line_mini_app: true }
    }
  ];

  var CART = {
    cart_id: 'cart-8f21a0',
    owner_id: 'buyer-7712',
    version: 6,
    expires_at_ms: Date.now() + 1000 * 60 * 60 * 24 * 3,
    items: [
      { sku: 'sku-1001', quantity: 2, unit_amount_snapshot: 1800, currency: 'JPY' },
      { sku: 'sku-1003', quantity: 1, unit_amount_snapshot: 980, currency: 'JPY' }
    ]
  };

  var ORDER = {
    order_id: 'ord-20261022-0142',
    buyer_id: 'buyer-7712',
    state: 'confirmed',
    lines: [
      { sku: 'sku-1001', quantity: 2, unit_amount_minor: 1800, currency: 'JPY' },
      { sku: 'sku-1003', quantity: 1, unit_amount_minor: 980, currency: 'JPY' }
    ],
    total: { amount_minor: 4580, currency: 'JPY' },
    reservation_id: 'rsv-5521',
    provider_ref: 'psp_7c19a2',
    transitions_used: 5,
    awaiting_reconciliation: false,
    trail: [
      { sequence: 1, event: 'checkout.created', from_state: 'new', to_state: 'reserving' },
      { sequence: 2, event: 'inventory.reserved', from_state: 'reserving', to_state: 'stock_reserved' },
      { sequence: 3, event: 'payment.authorized', from_state: 'stock_reserved', to_state: 'awaiting_payment' },
      { sequence: 4, event: 'payment.provider_pending', from_state: 'awaiting_payment', to_state: 'payment_unknown' },
      { sequence: 5, event: 'payment.reconciled', from_state: 'payment_unknown', to_state: 'confirming' },
      { sequence: 6, event: 'order.confirmed', from_state: 'confirming', to_state: 'confirmed' }
    ],
    outbox: [
      { event: 'order.confirmed', dispatched: true },
      { event: 'invoice.requested', dispatched: true }
    ]
  };

  /* ---------------------------------------------------------- problem+json */
  /* problem.dart: ApiProblem {type,title,status,code,detail,traceId,retryable} */
  var PROBLEMS = {
    payment_unknown: {
      type: 'https://gemarc.example/problems/payment.unknown',
      title: 'Payment provider has not answered',
      status: 202,
      code: 'payment.unknown',
      detail: 'The payment provider did not answer in time. The checkout stays open and is polled; re-submitting could charge the customer twice.',
      traceId: 'tr-9f2c41ab',
      retryable: false
    },
    version_conflict: {
      type: 'https://gemarc.example/problems/cart.version_conflict',
      title: 'Cart was modified elsewhere',
      status: 409,
      code: 'cart.version_conflict',
      detail: 'The cart version you based your change on is stale. Re-read the cart and re-apply your change.',
      traceId: 'tr-3b7d10e5',
      retryable: true
    },
    idempotency_conflict: {
      type: 'https://gemarc.example/problems/idempotency.conflict',
      title: 'Idempotency-Key reused with a different payload',
      status: 409,
      code: 'idempotency.conflict',
      detail: 'This checkout was already submitted with a different payload. Start a new checkout with a fresh Idempotency-Key.',
      traceId: 'tr-5aa821cf',
      retryable: false
    },
    line_unavailable: {
      type: 'https://gemarc.example/problems/line.sdk_unavailable',
      title: 'LINE features unavailable',
      status: 200,
      code: 'line.sdk_unavailable',
      detail: 'Browsing still works. Open inside the LINE app for the full experience.',
      traceId: 'tr-6ce02b93',
      retryable: false
    },
    forbidden: {
      type: 'https://gemarc.example/problems/order.forbidden',
      title: 'Not your order',
      status: 403,
      code: 'order.forbidden',
      detail: 'The signed-in user does not own this resource.',
      traceId: 'trace-1',
      retryable: false
    },
    offline: {
      type: 'client.offline',
      title: 'Network unavailable',
      status: 0,
      code: 'client.offline',
      detail: 'The storefront could not reach the API.',
      traceId: '',
      retryable: true
    }
  };

  /* --------------------------------------------------------------- shell */

  var CHANNEL_LABEL = { web: 'Web', app: 'App', line: 'LINE MINI App' };

  function channelSwitcher(active) {
    return '<div class="channels" role="group" aria-label="channel">' +
      ['web', 'app', 'line'].map(function (c) {
        return '<button type="button" data-channel="' + c + '"' +
          (c === active ? ' class="on"' : '') + '>' + CHANNEL_LABEL[c] + '</button>';
      }).join('') +
      '</div>';
  }

  function esc(v) {
    if (v === null || v === undefined) return '';
    return String(v)
      .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function el(id) { return document.getElementById(id); }

  function toast(message) {
    var node = el('sf-toast');
    if (!node) return;
    node.textContent = message;
    node.classList.add('show');
    clearTimeout(node._t);
    node._t = setTimeout(function () { node.classList.remove('show'); }, 1900);
  }

  function highlightJson(value) {
    var json = JSON.stringify(value, null, 2);
    return esc(json).replace(
      /(&quot;[^&]*?&quot;)(\s*:)?|(\b-?\d+(\.\d+)?\b)|(\b(true|false|null)\b)/g,
      function (m, str, colon, num, numFrac, bool) {
        if (str) return colon ? '<span class="k">' + str + '</span>' + colon : '<span class="s">' + str + '</span>';
        if (num) return '<span class="n">' + num + (numFrac || '') + '</span>';
        if (bool) return '<span class="b">' + bool + '</span>';
        return m;
      }
    );
  }

  /** Format a local time like Dart's toLocaleString (en-US style). */
  function fmtTime(ts) {
    if (!ts) return '—';
    var d = new Date(ts);
    return d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  }

  function fmtDateTime(ts) {
    if (!ts) return '—';
    var d = new Date(ts);
    return d.toLocaleString('en-US', {
      month: 'short', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit'
    });
  }

  /**
   * Render the checkout FSM as a vertical timeline.
   * The walk order is the real one: new → reserving → stock_reserved →
   * awaiting_payment → (payment_unknown) → confirming → confirmed.
   */
  function fsmTimeline(current) {
    var order = ['new', 'reserving', 'stock_reserved', 'awaiting_payment', 'payment_unknown', 'confirming', 'confirmed'];
    var idx = order.indexOf(current);
    var failed = current === 'rejected' || current === 'cancelled';
    return '<div class="fsm">' + order.map(function (wire) {
      var meta = CHECKOUT_STATES.filter(function (s) { return s.wire === wire; })[0];
      var cls = '';
      if (failed) {
        cls = wire === 'confirmed' ? '' : (wire === 'awaiting_payment' ? 'is-done' : '');
      } else if (idx >= 0 && order.indexOf(wire) < idx) {
        cls = 'is-done';
      } else if (wire === current) {
        cls = wire === 'payment_unknown' ? 'is-warn' : 'is-current';
      }
      if (failed && wire === 'confirming') cls = 'is-error';
      return '<div class="fsm__step ' + cls + '">' +
        '<div class="fsm__dot">' + (cls.indexOf('is-done') !== -1 ? '✓' : '') + '</div>' +
        '<div class="fsm__body">' +
        '<div class="fsm__name">' + meta.label + '</div>' +
        '<div class="fsm__note">' + (wire === 'payment_unknown'
          ? 'プロバイダ未応答。<b>再提出は禁止</b>、ポーリングのみ'
          : meta.note) + '</div>' +
        '</div></div>';
    }).join('') + '</div>';
  }

  function ready(fn) {
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', fn);
    } else {
      fn();
    }
  }

  return {
    CURRENCY: CURRENCY,
    money: money,
    AVAILABILITY: AVAILABILITY,
    availabilityBadge: availabilityBadge,
    isPurchasable: isPurchasable,
    banner: banner,
    CHECKOUT_STATES: CHECKOUT_STATES,
    isTerminal: isTerminal,
    isPollable: isPollable,
    isPending: isPending,
    checkoutMessage: checkoutMessage,
    PRODUCTS: PRODUCTS,
    CART: CART,
    ORDER: ORDER,
    PROBLEMS: PROBLEMS,
    CHANNEL_LABEL: CHANNEL_LABEL,
    channelSwitcher: channelSwitcher,
    esc: esc,
    el: el,
    toast: toast,
    highlightJson: highlightJson,
    fmtTime: fmtTime,
    fmtDateTime: fmtDateTime,
    fsmTimeline: fsmTimeline,
    ready: ready
  };
})();
