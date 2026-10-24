#!/usr/bin/env node
/**
 * probe-lib.mjs — the shape every probe in this suite wears.
 *
 * ## What a probe is
 *
 * A probe answers **one** question about a deployment that `audit.mjs`
 * cannot answer, and it can be wrong in only three directions:
 *
 *   PASS         the question was measured, and the deployment behaved.
 *   FAIL         the question was measured, and the deployment did not behave.
 *   UNMEASURED   the question could not be measured at all — the link
 *                dropped, the page never arrived, the instrument could not
 *                run the app.
 *
 * UNMEASURED is not a softer FAIL. It is the honest answer to "the link is
 * slow and Google Fonts is unreachable from here": the deployment did not
 * get a fair trial, and saying so is the only truthful thing to print.
 *
 * ## Why the distinction is worth a module
 *
 * The first versions of these probes printed free-form output and always
 * exited 0. A dead button, a page that never loaded and a page that loaded
 * fine all looked the same in the logs, and none of them was visible in an
 * exit code. Now every assertion is exactly one line, and the exit code
 * says which of the three things happened:
 *
 *   0   every assertion was measured and passed
 *   1   at least one assertion failed
 *   2   nothing failed, but at least one assertion could not be measured
 *
 * ## The environment facts this module knows
 *
 * The deployment is reached over a slow, flaky link, and `fonts.googleapis.com`
 * is unreachable from here. Two rules follow, and both are about not
 * blaming the product for the machine:
 *
 *   - a request that never arrived (a network-level error) is UNMEASURED,
 *     not FAIL: the deployment never got the chance to answer;
 *   - a request to a third-party host (the font CDN) is a NOTE, not a
 *     finding: the deployment does not own that host.
 *
 * A response the deployment itself sent — a 4xx or a 5xx — is a fact about
 * the deployment and is judged as one.
 */

import { chromium } from 'playwright';
import { pathToFileURL } from 'node:url';

export const DEFAULT_BASE = 'https://dokipon-commerce.vercel.app';

export const VIEWPORT = { width: 1280, height: 900 };

/** The same flags the audit uses: software rendering, so no GPU is needed. */
export const CHROME_ARGS = ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'];

/** The target every probe reads. */
export function baseUrl() {
  return (process.env.DOKIPON_BASE || DEFAULT_BASE).replace(/\/$/, '');
}

/**
 * The one-line result printer.
 *
 * `report.pass` / `report.fail` / `report.unmeasured` / `report.note` each
 * print exactly one line and record it, so the summary at the end of a run
 * can be built from the same data the reader saw.
 */
export function createReport(title) {
  const failedItems = [];
  const unmeasuredItems = [];
  let passed = 0;

  const print = (kind, name, detail) => {
    const suffix = detail ? ` — ${detail}` : '';
    console.log(`${kind.padEnd(11)} ${name}${suffix}`);
  };

  return {
    pass(name, detail = '') {
      passed += 1;
      print('PASS', name, detail);
    },
    fail(name, detail = '') {
      failedItems.push(detail ? `${name}: ${detail}` : name);
      print('FAIL', name, detail);
    },
    unmeasured(name, detail = '') {
      unmeasuredItems.push(detail ? `${name}: ${detail}` : name);
      print('UNMEASURED', name, detail);
    },
    note(name, detail = '') {
      print('NOTE', name, detail);
    },
    summary() {
      return {
        passed,
        failed: failedItems.length,
        unmeasured: unmeasuredItems.length,
        failedItems: [...failedItems],
        unmeasuredItems: [...unmeasuredItems],
      };
    },
  };
}

/**
 * The exit code a probe's summary earns.
 *
 * 0 = everything measured and passing, 1 = something failed, 2 = nothing
 * failed but something could not be measured.
 */
export function exitCodeFor(summary) {
  if (summary.failed > 0) return 1;
  if (summary.unmeasured > 0) return 2;
  return 0;
}

/** The closing lines of a standalone probe run. */
export function finish(title, summary) {
  console.log('');
  if (summary.failed === 0 && summary.unmeasured === 0) {
    console.log(`${title}: OK (${summary.passed} checks)`);
    return 0;
  }
  if (summary.failed === 0) {
    console.log(`${title}: ${summary.unmeasured} check(s) could not be measured`);
    for (const item of summary.unmeasuredItems) console.log(`  - ${item}`);
    return 2;
  }
  console.log(`${title}: ${summary.failed} failure(s), ${summary.unmeasured} unmeasured`);
  for (const item of summary.failedItems) console.log(`  - ${item}`);
  for (const item of summary.unmeasuredItems) console.log(`  - ${item}`);
  return 1;
}

/** True when this file was started directly, rather than imported. */
export function isMain(metaUrl) {
  if (!process.argv[1]) return false;
  return pathToFileURL(process.argv[1]).href === metaUrl;
}

/**
 * Run one probe on its own, with a browser of its own.
 *
 * `run-all.mjs` does not use this: it drives the browser probes through one
 * Chrome and one browser context, so the Flutter bundle is fetched once and
 * every probe after the first boots from a warm cache.
 */
export async function standalone({ title, run, options = {} }) {
  const report = createReport(title);
  const browser = await launchChrome();
  try {
    const context = await browser.newContext({ viewport: VIEWPORT });
    const page = await context.newPage();
    await run(page, report, options);
  } finally {
    await browser.close();
  }
  return finish(title, report.summary());
}

// ---------------------------------------------------------------------------
// The browser
// ---------------------------------------------------------------------------

export async function launchChrome() {
  return chromium.launch({ channel: 'chrome', args: CHROME_ARGS });
}

/**
 * The events a page produces, collected while the probe runs.
 *
 * `failedRequests` and `badResponses` keep the URL, because the question
 * "was it the deployment's own asset or a third party's?" can only be
 * answered with the URL in hand.
 */
export function collectors(page) {
  const state = { consoleErrors: [], pageErrors: [], failedRequests: [], badResponses: [] };
  page.on('console', (m) => {
    if (m.type() === 'error') state.consoleErrors.push(m.text());
  });
  page.on('pageerror', (e) => state.pageErrors.push(String(e && e.message ? e.message : e)));
  page.on('requestfailed', (r) => {
    const reason = (r.failure() && r.failure().errorText) || 'unknown';
    state.failedRequests.push({ url: r.url(), reason });
  });
  page.on('response', (r) => {
    if (r.status() >= 400) state.badResponses.push({ url: r.url(), status: r.status() });
  });
  return state;
}

function hostOf(url) {
  try {
    return new URL(url).host;
  } catch {
    return '';
  }
}

/** True when the URL is on the deployment's own host. */
export function isOwnHost(url, base = baseUrl()) {
  const host = hostOf(url);
  if (!host) return false;
  try {
    return host === new URL(base).host;
  } catch {
    return false;
  }
}

/** True for the font CDN, which is unreachable from here and is not the deployment's. */
export function isFontCdn(url) {
  return /(^|\.)(fonts\.googleapis\.com|fonts\.gstatic\.com)$/.test(hostOf(url));
}

/** True when the URL is one of the deployment's own API routes. */
export function isApiCall(url) {
  return /\/v1\/|\/admin\/v1\/|\/healthz/.test(url);
}

/**
 * Open a surface.
 *
 * A navigation that times out is a network fact, not an HTTP answer: the
 * caller gets `error` set and the URL it tried, and decides which of the
 * three verdicts that is.
 */
export async function openSurface(page, path, timeoutMs = 300_000) {
  const url = `${baseUrl()}${path}`;
  try {
    const response = await page.goto(url, { waitUntil: 'domcontentloaded', timeout: timeoutMs });
    return { url, status: response ? response.status() : null, error: null };
  } catch (e) {
    return { url, status: null, error: String(e && e.message ? e.message : e).split('\n')[0] };
  }
}

/**
 * Wait for the Flutter app to mount.
 *
 * `flutter-view` is the element that actually exists in Flutter 3.47 — the
 * first version of the audit looked for `flt-scene-host`, which does not
 * exist, and reported every control as "the app unmounted".
 */
export async function bootFlutter(page, timeoutMs = 300_000) {
  try {
    await page.waitForSelector('flutter-view', { state: 'attached', timeout: timeoutMs });
    await page.waitForTimeout(1800);
    await page.waitForLoadState('networkidle', { timeout: 12_000 }).catch(() => {});
    return { mounted: true, reason: '' };
  } catch (e) {
    return { mounted: false, reason: String(e && e.message ? e.message : e).split('\n')[0] };
  }
}

/**
 * Turn on the semantics tree.
 *
 * Flutter renders into a shadow root, so the buttons are not DOM elements
 * until the semantics tree is on. Clicking the "enable accessibility"
 * placeholder is the ask.
 */
export async function enableSemantics(page) {
  const placeholder = page.locator('flt-semantics-placeholder').first();
  if ((await placeholder.count()) === 0) return false;
  await placeholder.evaluate((el) => el.click()).catch(() => {});
  await page.waitForTimeout(800);
  return (await page.locator('flt-semantics').count()) > 0;
}

/**
 * The *content* of the semantics tree.
 *
 * A screen that changed produces a different content, even when the number
 * of nodes is the same — a dialog that replaces one node with two does not
 * change the count but does change the content.
 */
export async function signature(page) {
  return page.evaluate(() =>
    [...document.querySelectorAll('flt-semantics')]
      .map((n) => `${n.getAttribute('role') || ''}|${n.getAttribute('aria-label') || (n.textContent || '').trim()}`)
      .join('\n'),
  );
}

/** Every control label the semantics tree exposes, in order. */
export async function controlLabels(page, role = 'button') {
  const controls = page.locator(`flt-semantics[role="${role}"]`);
  const count = await controls.count();
  const out = [];
  for (let i = 0; i < count; i++) {
    out.push(await controls.nth(i).evaluate((node) => (node.getAttribute('aria-label') || node.textContent || '').trim()));
  }
  return out;
}

/**
 * Find the control whose label matches.
 *
 * The role is searched in the order given, and the label is matched against
 * the control's aria-label (or its text). `pattern` may be a regular
 * expression or a predicate, which is how a caller excludes one label from
 * another (the cart button from the add-to-cart button). Returns `null`
 * when nothing matches, so a caller can tell "the control is not there"
 * from "the control is there and it did nothing".
 */
export async function findControl(page, pattern, roles = ['button', 'tab', 'link']) {
  const matches = (label) => (typeof pattern === 'function' ? Boolean(pattern(label)) : pattern.test(label));
  for (const role of roles) {
    const controls = page.locator(`flt-semantics[role="${role}"]`);
    const count = await controls.count();
    for (let i = 0; i < count; i++) {
      const label = await controls.nth(i).evaluate((node) => (node.getAttribute('aria-label') || node.textContent || '').trim());
      if (label && matches(label)) return { role, index: i, label };
    }
  }
  return null;
}

/** The locator for a control found by `findControl`. */
export function controlLocator(page, role, index) {
  return page.locator(`flt-semantics[role="${role}"]`).nth(index);
}

/**
 * Sign in to the admin console.
 *
 * The fields are **typed**, not filled: Flutter's web text editing mirrors
 * genuine key input into its `TextEditingController`, and a programmatic
 * `fill()` sets the DOM input's value, which the controller never sees.
 */
export async function signIn(page, { user, password, settleMs = 4000 } = {}) {
  const inputs = page.locator('input, textarea');
  if ((await inputs.count()) < 2) return { ok: false, reason: 'no credential fields found' };
  const loginUser = user || process.env.ADMIN_USER || 'admin';
  const loginPassword = password || process.env.ADMIN_PASSWORD || 'dev-only-admin';
  try {
    await inputs.nth(0).click();
    await page.waitForTimeout(300);
    await page.keyboard.type(loginUser, { delay: 40 });
    await inputs.nth(1).click();
    await page.waitForTimeout(300);
    await page.keyboard.type(loginPassword, { delay: 40 });
  } catch (e) {
    return { ok: false, reason: `could not type the credentials: ${String(e.message).split('\n')[0]}` };
  }
  const typed = await page.evaluate(() => [...document.querySelectorAll('input, textarea')].map((n) => n.value));
  if (typed[1] !== loginPassword) {
    return { ok: false, reason: 'the password field did not receive the credential' };
  }
  const signInButton = page
    .locator('flt-semantics[role="button"]')
    .filter({ hasText: /sign in|ログイン/i })
    .first();
  if ((await signInButton.count()) === 0) return { ok: false, reason: 'no sign-in button found' };
  await signInButton.click({ force: true, timeout: 8000 }).catch(() => {});
  await page.waitForTimeout(settleMs);
  const stillLogin = await page.locator('input, textarea').count();
  return {
    ok: stillLogin === 0,
    reason: stillLogin === 0 ? '' : 'the console still shows the credential fields',
  };
}
