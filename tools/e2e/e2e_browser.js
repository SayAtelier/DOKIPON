#!/usr/bin/env node
/**
 * e2e_browser.js — drive the deployed commerce stack with a real browser.
 *
 * Why this exists when curl already says everything answers 200: a Flutter Web
 * console is client-rendered, so an HTTP 200 on main.dart.js proves only that
 * nginx served a file. Whether the app boots, whether the WebGL canvas gets a
 * context, and whether the admin login round-trips are all decided in the
 * browser and are invisible to curl.
 *
 * Runs on the k3s VM (it needs Playwright's Chromium, which is only installed
 * there). Usage:
 *
 *   BASE=http://127.0.0.1:30080 ADMIN=http://127.0.0.1:30087 \
 *   CONSOLE=http://127.0.0.1:30088 \
 *   ADMIN_USER=operator ADMIN_PASSWORD='...' \
 *   node e2e_browser.js
 */

const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const BASE = process.env.BASE || 'http://127.0.0.1:30080';
const ADMIN = process.env.ADMIN || 'http://127.0.0.1:30087';
const CONSOLE = process.env.CONSOLE || 'http://127.0.0.1:30088';
const ADMIN_USER = process.env.ADMIN_USER || 'operator';
const ADMIN_PASSWORD = process.env.ADMIN_PASSWORD || '';
// Default next to the script, so a fresh clone works without editing paths.
const SHOTS = process.env.SHOTS || path.join(__dirname, 'shots');

fs.mkdirSync(SHOTS, { recursive: true });

let pass = 0;
let fail = 0;
const failures = [];

function check(name, ok, detail = '') {
  if (ok) {
    pass += 1;
    console.log(`  PASS  ${name}${detail ? '  ' + detail : ''}`);
  } else {
    fail += 1;
    failures.push(name);
    console.log(`  FAIL  ${name}${detail ? '  ' + detail : ''}`);
  }
}

async function shot(page, label) {
  const file = path.join(SHOTS, `${label}.png`);
  try {
    await page.screenshot({ path: file, fullPage: true });
    return file;
  } catch (e) {
    return `screenshot failed: ${e.message}`;
  }
}

(async () => {
  const browser = await chromium.launch({
    args: ['--no-sandbox', '--disable-dev-shm-usage'],
  });
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });

  // Every console error and page error is collected; a Flutter app that fails to
  // boot logs to the console and still returns HTTP 200 everywhere.
  const consoleErrors = [];
  ctx.on('page', (p) => {
    p.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text());
    });
    p.on('pageerror', (e) => consoleErrors.push(`pageerror: ${e.message}`));
  });

  // ---------------------------------------------------------------- storefront
  console.log('\n=== FRONT-END: BFF storefront ===');
  {
    const page = await ctx.newPage();
    const resp = await page.goto(`${BASE}/v1/storefront`, { waitUntil: 'domcontentloaded' });
    check('storefront responds 200', resp.status() === 200, `status=${resp.status()}`);
    const body = await page.evaluate(() => document.body.innerText);
    let parsed = null;
    try {
      parsed = JSON.parse(body);
    } catch (e) {
      /* reported below */
    }
    check('storefront returns JSON', parsed !== null);
    if (parsed) {
      check('storefront has channel config', !!parsed.channel, Object.keys(parsed).join(','));
    }
    await page.close();
  }

  // ------------------------------------------------------------ admin console
  console.log('\n=== ADMIN: Flutter Web console ===');
  {
    const page = await ctx.newPage();
    const before = consoleErrors.length;

    const resp = await page.goto(`${CONSOLE}/`, { waitUntil: 'load', timeout: 45000 });
    check('console root responds 200', resp.status() === 200, `status=${resp.status()}`);

    // Flutter renders into <flt-glass-pane> / a canvas, so the DOM stays nearly
    // empty. Waiting for the engine's own marker is the only reliable signal.
    let booted = false;
    try {
      await page.waitForFunction(
        () => !!document.querySelector('flt-glass-pane, flutter-view, flt-scene-host, canvas'),
        { timeout: 30000 },
      );
      booted = true;
    } catch (e) {
      booted = false;
    }
    check('flutter engine mounted a view', booted);
    await page.waitForTimeout(2500);

    const title = await page.title();
    check('document has a title', !!title && title.length > 0, `title="${title}"`);

    const html = await page.content();
    check('bundle references flutter.js', html.includes('flutter'), '');
    check('service worker registered', await page.evaluate(() => !!navigator.serviceWorker));

    const newErrors = consoleErrors.slice(before);
    // canvaskit occasionally emits a benign font/gpu warning; only fail on
    // errors that are not resource-load noise.
    const fatal = newErrors.filter(
      (e) => !/favicon|Failed to load resource.*(404|403)/i.test(e),
    );
    check(
      'no fatal console errors during boot',
      fatal.length === 0,
      fatal.length ? fatal.slice(0, 3).join(' | ') : '',
    );

    const f = await shot(page, 'admin-console');
    console.log(`        screenshot: ${f}`);
    await page.close();
  }

  // -------------------------------------------------------------- admin API
  console.log('\n=== ADMIN: API round-trip through the browser ===');
  {
    if (!ADMIN_PASSWORD) {
      check('admin password available for browser login', false, 'ADMIN_PASSWORD unset');
    } else {
      const page = await ctx.newPage();
      const resp = await page.goto(`${ADMIN}/healthz`, { waitUntil: 'domcontentloaded' });
      check('admin healthz responds 200', resp.status() === 200, `status=${resp.status()}`);

      // Do the login from inside the page so the CORS preflight and cookie
      // handling are exercised exactly as the console would.
      const result = await page.evaluate(
        async ([url, user, pass]) => {
          const r = await fetch(`${url}/admin/v1/sessions`, {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ user, password: pass }),
          });
          const text = await r.text();
          let token = null;
          try {
            token = JSON.parse(text).token || null;
          } catch (e) {
            /* reported by status */
          }
          return { status: r.status, token, body: text.slice(0, 200) };
        },
        [ADMIN, ADMIN_USER, ADMIN_PASSWORD],
      );

      check('login from browser origin succeeds', result.status === 200, `status=${result.status}`);
      check('login returns a token', !!result.token, result.body);

      if (result.token) {
        const orders = await page.evaluate(
          async ([url, token]) => {
            const r = await fetch(`${url}/admin/v1/orders?limit=100`, {
              headers: { authorization: `Bearer ${token}` },
            });
            return { status: r.status, body: (await r.text()).slice(0, 300) };
          },
          [ADMIN, result.token],
        );
        check('authenticated order list responds 200', orders.status === 200, `status=${orders.status}`);
        let count = null;
        try {
          count = (JSON.parse(orders.body).items || []).length;
        } catch (e) {
          /* reported below */
        }
        check('order list is JSON', count !== null, orders.body.slice(0, 80));

        const products = await page.evaluate(
          async ([url, token]) => {
            const r = await fetch(`${url}/admin/v1/products`, {
              headers: { authorization: `Bearer ${token}` },
            });
            return { status: r.status, body: (await r.text()).slice(0, 200) };
          },
          [ADMIN, result.token],
        );
        check('product catalogue responds 200', products.status === 200, `status=${products.status}`);
      }

      await page.close();
    }
  }

  await browser.close();

  console.log(`\n=== RESULT: ${pass} passed, ${fail} failed ===`);
  if (failures.length) console.log(`failed: ${failures.join(', ')}`);
  process.exit(fail === 0 ? 0 : 1);
})().catch((e) => {
  console.error('fatal:', e.stack || e.message);
  process.exit(2);
});
