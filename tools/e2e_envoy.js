#!/usr/bin/env node
/**
 * e2e_envoy.js — verify the commerce storefront and admin console THROUGH Envoy.
 *
 * This is the check that matters after putting a gateway in front: curl against
 * each backend proves the backends work, and curl against each NodePort proves
 * they are reachable, but neither proves Envoy is routing. Everything here goes
 * through one Envoy listener and is selected by the Host header, exactly as a
 * browser would send it.
 *
 * Host-header selection is deliberate: shop.local and admin.local are two
 * different Flutter bundles on two different Deployments, and Envoy tells them
 * apart by which name the client asked for. A path-prefix-only test would pass
 * even with Host routing misconfigured, which is the failure mode worth catching.
 *
 * Runs on the k3s VM (Playwright's Chromium is installed there only):
 *   node e2e_envoy.js
 *
 * Env:
 *   ENVOY     base URL of the Envoy listener        (default http://127.0.0.1:30880)
 *   SHOP      hostname routed to the storefront     (default shop.local)
 *   ADMIN     hostname routed to the admin console  (default admin.local)
 *   ADMIN_USER / ADMIN_PASSWORD   operator credential for the API round-trip
 */

const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const ENVOY = process.env.ENVOY || 'http://127.0.0.1:30880';
const ENVOY_PORT = new URL(ENVOY).port || '80';
const SHOP = process.env.SHOP || 'shop.local';
const ADMIN = process.env.ADMIN || 'admin.local';
const ADMIN_USER = process.env.ADMIN_USER || 'operator';
const ADMIN_PASSWORD = process.env.ADMIN_PASSWORD || '';
const SHOTS = process.env.SHOTS || '/home/ubuntu/e2e-browser/shots';

fs.mkdirSync(SHOTS, { recursive: true });

let pass = 0;
const failures = [];

function check(name, ok, detail = '') {
  if (ok) {
    pass += 1;
    console.log(`  PASS  ${name}${detail ? '  ' + detail : ''}`);
  } else {
    failures.push(name);
    console.log(`  FAIL  ${name}${detail ? '  ' + detail : ''}`);
  }
}

/**
 * Point the browser at the Envoy listener while asking for a virtual host.
 *
 * The Host header is set per-request rather than by rewriting the URL with
 * page.route().continue({ url }). That rewrite makes Playwright treat the
 * response as opaque, so the top-level document load can come back 404 while a
 * plain curl to the identical URL returns 200 — a false failure that looks like
 * an Envoy routing bug. Setting the header keeps the request a normal one.
 */
async function newSite(browser, hostname, errors) {
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(`[${hostname}] ${m.text()}`);
  });
  page.on('pageerror', (e) => errors.push(`[${hostname}] pageerror: ${e.message}`));
  page.__ctx = ctx;
  return page;
}

/** A Flutter app renders into its own view; an empty <body> does not mean broken. */
async function waitForFlutter(page, timeout = 30000) {
  try {
    await page.waitForFunction(
      () => !!document.querySelector('flt-glass-pane, flutter-view, flt-scene-host, canvas'),
      { timeout },
    );
    return true;
  } catch {
    return false;
  }
}

(async () => {
  // Map the virtual hosts onto the Envoy listener so the browser emits a real
  // Host header. extraHTTPHeaders cannot set Host (Chromium rejects it with
  // ERR_INVALID_ARGUMENT), and rewriting the URL in page.route() makes the
  // top-level document opaque so it can 404 while curl on the same URL returns
  // 200 — a false failure that reads as an Envoy routing bug.
  const hostRules = [SHOP, ADMIN]
    .map((h) => `MAP ${h} 127.0.0.1:${ENVOY_PORT}`)
    .join(', ');
  const browser = await chromium.launch({
    args: ['--no-sandbox', '--disable-dev-shm-usage', `--host-resolver-rules=${hostRules}`],
  });
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const errors = [];

  // ------------------------------------------------------------------- routing
  console.log('\n=== ROUTING: one Envoy listener, selected by Host ===');
  {
    const probe = await ctx.request;
    const cases = [
      ['storefront root', SHOP, '/'],
      ['storefront bundle', SHOP, '/main.dart.js'],
      ['storefront API', SHOP, '/v1/storefront'],
      ['console root', ADMIN, '/'],
      ['console bundle', ADMIN, '/main.dart.js'],
      ['console healthz', ADMIN, '/healthz'],
    ];
    for (const [label, host, p] of cases) {
      const r = await probe.get(`${ENVOY}${p}`, { headers: { host }, timeout: 15000 });
      check(`${label}  ${host}${p}`, r.status() === 200, `status=${r.status()}`);
    }
  }

  // ------------------------------------------------------------ cross-routing
  // The negative case: if Host routing were ignored, everything would answer 200
  // and the positive checks above would be meaningless.
  console.log('\n=== ROUTING: the two hosts must not serve each other bundle ===');
  {
    const probe = await ctx.request;
    const a = await probe.get(`${ENVOY}/main.dart.js`, { headers: { host: SHOP }, timeout: 15000 });
    const b = await probe.get(`${ENVOY}/main.dart.js`, { headers: { host: ADMIN }, timeout: 15000 });
    const same = (await a.body()).equals(await b.body());
    check('the two hosts serve different bundles', !same,
      same ? 'identical body — Host routing is being ignored' : 'bodies differ');
  }

  // --------------------------------------------------------------- storefront
  console.log('\n=== STOREFRONT (through Envoy) ===');
  {
    const page = await newSite(browser, SHOP, errors);
    const before = errors.length;
    const resp = await page.goto(`http://${SHOP}/`, { waitUntil: 'load', timeout: 45000 });
    check('storefront root responds 200', resp.status() === 200, `status=${resp.status()}`);

    const booted = await waitForFlutter(page);
    check('flutter engine mounted a view', booted);
    await page.waitForTimeout(2500);

    const title = await page.title();
    check('document has a title', !!title && title.length > 0, `title="${title}"`);

    const fatal = errors.slice(before).filter((e) => !/favicon|Failed to load resource/i.test(e));
    check('no fatal console errors', fatal.length === 0, fatal.slice(0, 2).join(' | '));

    await page.screenshot({ path: path.join(SHOTS, 'envoy-shopfront.png'), fullPage: true });
    console.log(`        screenshot: ${path.join(SHOTS, 'envoy-shopfront.png')}`);
    await page.__ctx.close();
  }

  // ----------------------------------------------------------- admin console
  console.log('\n=== ADMIN CONSOLE (through Envoy) ===');
  {
    const page = await newSite(browser, ADMIN, errors);
    const before = errors.length;
    const resp = await page.goto(`http://${ADMIN}/`, { waitUntil: 'load', timeout: 45000 });
    check('console root responds 200', resp.status() === 200, `status=${resp.status()}`);

    const booted = await waitForFlutter(page);
    check('flutter engine mounted a view', booted);
    await page.waitForTimeout(2500);

    const title = await page.title();
    check('document has a title', !!title && title.length > 0, `title="${title}"`);

    const fatal = errors.slice(before).filter((e) => !/favicon|Failed to load resource/i.test(e));
    check('no fatal console errors', fatal.length === 0, fatal.slice(0, 2).join(' | '));

    await page.screenshot({ path: path.join(SHOTS, 'envoy-admin.png'), fullPage: true });
    console.log(`        screenshot: ${path.join(SHOTS, 'envoy-admin.png')}`);
    await page.__ctx.close();
  }

  // -------------------------------------------------------------- API via gw
  console.log('\n=== API round-trip from a browser origin, through Envoy ===');
  {
    if (!ADMIN_PASSWORD) {
      check('credential available', false, 'ADMIN_PASSWORD unset');
    } else {
      const page = await newSite(browser, ADMIN, errors);
      await page.goto(`http://${ADMIN}/`, { waitUntil: 'domcontentloaded' });

      const r = await page.evaluate(
        async ([user, pass]) => {
          const login = await fetch('/admin/v1/sessions', {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ user, password: pass }),
          });
          const token = (await login.json().catch(() => ({}))).token || null;
          const orders = await fetch('/admin/v1/orders?limit=100', {
            headers: { authorization: `Bearer ${token}` },
          });
          const products = await fetch('/admin/v1/products', {
            headers: { authorization: `Bearer ${token}` },
          });
          return {
            login: login.status,
            token: !!token,
            orders: orders.status,
            products: products.status,
            productsBody: (await products.text()).slice(0, 160),
          };
        },
        [ADMIN_USER, ADMIN_PASSWORD],
      );

      check('login through the gateway returns 200', r.login === 200, `status=${r.login}`);
      check('login returns a token', r.token);
      check('order list through the gateway returns 200', r.orders === 200, `status=${r.orders}`);
      check('product list through the gateway returns 200', r.products === 200,
        `status=${r.products} ${r.productsBody}`);
      await page.__ctx.close();
    }
  }

  await browser.close();
  console.log(`\n=== RESULT: ${pass} passed, ${failures.length} failed ===`);
  if (failures.length) console.log(`failed: ${failures.join(', ')}`);
  process.exit(failures.length === 0 ? 0 : 1);
})().catch((e) => {
  console.error('fatal:', e.stack || e.message);
  process.exit(2);
});
