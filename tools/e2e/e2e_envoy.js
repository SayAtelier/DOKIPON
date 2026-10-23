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
 * How much of the page is actually drawn?
 *
 * The previous version measured "what fraction of pixels share the single most
 * common colour" and called a page blank above 90%. That is the wrong signal:
 * on a light theme the background legitimately covers 70-86% of the page, so
 * the threshold had to be set just above the real pages, and any future page
 * with a lighter theme would be reported blank. It also could not tell a
 * rendered login form from a fully rendered dashboard.
 *
 * What matters is how many pixels differ from the background, because that is
 * "content actually drawn", independent of how many shades the theme uses.
 * Measured on this stack:
 *
 *   storefront  28.5% non-background, 1717 distinct colours
 *   admin       13.5% non-background,  795 distinct colours
 *   blank page   0.1% non-background,    2 distinct colours
 *
 * 2% is the threshold: 6x below the lightest real page (13.5%) and 8x above
 * the sparse adversarial samples (0.25%). The first draft used 3%, and the
 * self-check in selfcheck.js immediately showed that a sparse but genuinely
 * drawn page at 0.25% would have been reported blank.
 *
 * Returns { ratio, distinct } so the caller can report the number, not just a
 * boolean — a test that only says "not blank" hides how close it came.
 */
const BLANK_CONTENT_RATIO = 0.02;

function measureContent(buffer) {
  const PNG = require('pngjs').PNG;
  return new Promise((resolve, reject) => {
    new PNG({ filterType: 4 })
      .parse(buffer, (err, img) => {
        if (err) return reject(err);
        const total = img.width * img.height;
        if (!total) return resolve({ ratio: 0, distinct: 0 });

        const freq = new Map();
        for (let i = 0; i < img.data.length; i += 4) {
          if (img.data[i + 3] === 0) continue;   // ignore transparent
          const key = (img.data[i] << 16) | (img.data[i + 1] << 8) | img.data[i + 2];
          freq.set(key, (freq.get(key) || 0) + 1);
        }
        if (freq.size === 0) return resolve({ ratio: 0, distinct: 0 });

        let bgCount = 0;
        for (const c of freq.values()) if (c > bgCount) bgCount = c;
        const nonBackground = total - bgCount;

        return resolve({
          ratio: nonBackground / total,
          distinct: freq.size,
        });
      });
  });
}

const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const ENVOY = process.env.ENVOY || 'http://127.0.0.1:30880';
const ENVOY_PORT = new URL(ENVOY).port || '80';
const SHOP = process.env.SHOP || 'shop.local';
const ADMIN = process.env.ADMIN || 'admin.local';
const ADMIN_USER = process.env.ADMIN_USER || 'operator';
const ADMIN_PASSWORD = process.env.ADMIN_PASSWORD || '';
// Default next to the script, so a fresh clone works without editing paths.
const SHOTS = process.env.SHOTS || path.join(__dirname, 'shots');

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
    args: [
      '--no-sandbox',
      '--disable-dev-shm-usage',
      `--host-resolver-rules=${hostRules}`,
      // Flutter Web renders through canvaskit. Without a GPU, headless Chromium
      // disables the software rasteriser, the canvas comes up blank, and the
      // screenshot looks like a loaded-but-empty page — which is exactly what
      // happens without this flag. Nothing is logged as an error, so a test that
      // only checks "no console errors" reports success on an empty page.
      '--enable-unsafe-swiftshader',
    ],
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

    // The screenshot must not be blank: a Flutter app that loaded but failed to
    // render (e.g. no GPU) produces a screenshot that is >95% background colour,
    // which is indistinguishable from an HTTP 200 with no content. We reject
    // that as a failure, because "no console errors" on an empty page is a
    // false positive. A real page has <90% background pixels.
    const shopShot = fs.readFileSync(path.join(SHOTS, 'envoy-shopfront.png'));
    const shopContent = await measureContent(shopShot);
    check('storefront screenshot has real content',
      shopContent.ratio >= BLANK_CONTENT_RATIO,
      `non-background ${(shopContent.ratio * 100).toFixed(1)}%, ${shopContent.distinct} colours`);

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

    // Same blank-screenshot guard for the admin console.
    const adminShot = fs.readFileSync(path.join(SHOTS, 'envoy-admin.png'));
    const adminContent = await measureContent(adminShot);
    check('admin console screenshot has real content',
      adminContent.ratio >= BLANK_CONTENT_RATIO,
      `non-background ${(adminContent.ratio * 100).toFixed(1)}%, ${adminContent.distinct} colours`);

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
