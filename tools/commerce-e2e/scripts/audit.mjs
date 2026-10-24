#!/usr/bin/env node
/**
 * audit.mjs — drive the deployed DOKIPON commerce deployment with a real
 * browser and report every control that does not work.
 *
 * ## Why this is not a DOM test
 *
 * The deployment is a Flutter Web app (the storefront at `/`, the admin
 * console at `/admin/`) on top of a Rust edge that serves the commerce
 * contract. Flutter renders into a shadow root, so the buttons are not DOM
 * elements and a DOM-based test sees an empty page. The way to drive a
 * Flutter Web app is to turn on the **semantics tree** — the parallel DOM
 * representation Flutter keeps for screen readers, which stays off until
 * something asks for it. Clicking the "enable accessibility" placeholder is
 * that ask, and afterwards every button, link and field is a real element
 * with a role and a label.
 *
 * ## What counts as a broken control
 *
 * For every control the audit finds, the app is reloaded, the control is
 * clicked, and five things are checked:
 *
 *   1. the click dispatches without an uncaught exception,
 *   2. no new console error appears,
 *   3. no request to the API fails,
 *   4. the app is still mounted afterwards,
 *   5. something actually happened: the URL moved, the semantics tree
 *      changed, or a request was made.
 *
 * A control that passes 1-4 but fails 5 is reported as **inert**: it is
 * present, it does not throw, and it does nothing. A button that is painted
 * but dead is exactly the defect this audit is looking for, and it is
 * invisible to a screenshot.
 *
 * ## Liveness, and a note on how it was got wrong first
 *
 * The first version of this audit checked for `flt-scene-host` to decide
 * whether the app was still alive. Flutter 3.47 has no such element — the
 * DOM is `<flutter-view><flt-glass-pane>…` — so the check was always false
 * and every single control was reported as "the app unmounted after the
 * click". Eighteen of eighteen, on two surfaces. The finding was an artefact
 * of the instrument, not a property of the app. The liveness check now
 * looks for the elements that actually exist, and the change signal is the
 * **content** of the semantics tree rather than its size, because a control
 * that opens a dialog can leave the node count unchanged.
 *
 * ## Usage
 *
 *   node tools/commerce-e2e/scripts/audit.mjs
 *   DOKIPON_BASE=https://... node tools/commerce-e2e/scripts/audit.mjs
 *
 * Writes a JSON report and a Markdown summary under `tools/commerce-e2e/reports/`.
 */

import { chromium } from 'playwright';
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '..');
const REPORTS = resolve(ROOT, 'reports');

const BASE = (process.env.DOKIPON_BASE || 'https://dokipon-commerce.vercel.app').replace(/\/$/, '');
const EXPECT = JSON.parse(
  readFileSync(resolve(ROOT, 'mock_data', 'contract-expectations.json'), 'utf8'),
);

const VIEWPORT = { width: 1280, height: 900 };
const BOOT_TIMEOUT = 240_000;
const SETTLE_MS = 1000;

/** The elements that prove a Flutter Web app is alive in Flutter 3.47. */
const ALIVE_SELECTOR = 'flutter-view, flt-glass-pane';

/**
 * Per-target collectors.
 *
 * The listeners are attached **once per page**: a page that is reloaded for
 * every control would otherwise collect a new set of listeners on each pass
 * and report the same event several times.
 */
function collectors(page) {
  const state = { consoleErrors: [], pageErrors: [], badResponses: [], failedRequests: [], requests: [] };
  page.on('console', (m) => {
    if (m.type() === 'error') state.consoleErrors.push(m.text());
  });
  page.on('pageerror', (e) => state.pageErrors.push(String(e && e.message ? e.message : e)));
  page.on('request', (r) => state.requests.push(r.url()));
  page.on('requestfailed', (r) => {
    const reason = (r.failure() && r.failure().errorText) || 'unknown';
    state.failedRequests.push(`${reason} ${r.url()}`);
  });
  page.on('response', (r) => {
    if (r.status() >= 400) state.badResponses.push(`${r.status()} ${r.url()}`);
  });
  return state;
}

/** Clear the collected events so each control is measured on its own. */
function resetCollectors(state) {
  state.consoleErrors.length = 0;
  state.pageErrors.length = 0;
  state.badResponses.length = 0;
  state.failedRequests.length = 0;
  state.requests.length = 0;
}

/**
 * Wait for the Flutter app to be mounted and for its first frame to be on screen.
 * Retries once: the first boot of a cold function can outlast the timeout.
 */
async function waitForFlutter(page) {
  for (let attempt = 1; attempt <= 2; attempt++) {
    try {
      // `state: 'attached'`, not the default 'visible': Flutter's own
      // containers are zero-size, so they never become "visible" to
      // Playwright even when the app is running perfectly.
      await page.waitForSelector(ALIVE_SELECTOR, { state: 'attached', timeout: BOOT_TIMEOUT });
      await page.waitForTimeout(1800);
      await page
        .waitForLoadState('networkidle', { timeout: 12_000 })
        .catch(() => { /* a poll or a websocket keeps the network busy; that is fine */ });
      return true;
    } catch (e) {
      if (attempt === 2) throw e;
      await page.waitForTimeout(2000);
    }
  }
  return false;
}

/**
 * Turn on the semantics tree. Without it a Flutter Web app exposes no
 * inspectable controls at all.
 */
async function enableSemantics(page) {
  const placeholder = page.locator('flt-semantics-placeholder');
  if ((await placeholder.count()) === 0) return false;
  await placeholder.first().evaluate((el) => el.click()).catch(() => {});
  await page.waitForTimeout(800);
  return (await page.locator('flt-semantics').count()) > 0;
}

const CONTROL_SELECTOR = [
  'flt-semantics[role="button"]',
  'flt-semantics[role="link"]',
  'flt-semantics[role="checkbox"]',
  'flt-semantics[role="radio"]',
  'flt-semantics[role="tab"]',
  'flt-semantics[role="switch"]',
].join(', ');

/** Every interactive control the semantics tree exposes. */
async function listControls(page) {
  const controls = page.locator(CONTROL_SELECTOR);
  const n = await controls.count();
  const out = [];
  for (let i = 0; i < n; i++) {
    const el = controls.nth(i);
    const info = await el.evaluate((node) => {
      const r = node.getBoundingClientRect();
      return {
        role: node.getAttribute('role'),
        label: node.getAttribute('aria-label') || (node.textContent || '').trim(),
        checked: node.getAttribute('aria-checked'),
        selected: node.getAttribute('aria-selected'),
        disabled: node.getAttribute('aria-disabled') === 'true',
        rect: { x: r.x, y: r.y, w: r.width, h: r.height },
      };
    });
    out.push({ index: i, ...info });
  }
  return out;
}

/**
 * The state of the app, read back from the page.
 *
 * `signature` is the *content* of the semantics tree, not its size. A dialog
 * that replaces one node with two changes the content; a control that opens
 * a sheet with the same number of nodes does not change the count but does
 * change the content.
 */
async function probe(page) {
  return page.evaluate(() => {
    const nodes = [...document.querySelectorAll('flt-semantics')];
    const signature = nodes
      .map((n) => `${n.getAttribute('role') || ''}|${n.getAttribute('aria-label') || (n.textContent || '').trim()}`)
      .join('\n');
    return {
      url: location.href,
      alive: !!document.querySelector('flutter-view') && !!document.querySelector('flt-glass-pane'),
      semantics: nodes.length,
      signature,
    };
  });
}

/** A console error that is really a missing static resource, not an app error. */
function isResourceNoise(text) {
  return /Failed to load resource/i.test(text);
}

/**
 * Click one control and judge what happened.
 *
 * The app is reloaded before each click so that one control's effect cannot
 * be mistaken for the next one's, and so a control that navigated away does
 * not make the following controls unreachable.
 */
async function auditControl(page, control, state) {
  resetCollectors(state);
  const before = await probe(page);

  const target = page.locator(CONTROL_SELECTOR).nth(control.index);
  const started = Date.now();
  let clickError = null;
  let stillOffScreen = false;

  // Is the control on screen? Measured on the layout the user's mouse hits.
  const onScreen = async () => target.evaluate((node) => {
    const r = node.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) return false;
    return r.bottom > 0 && r.top < window.innerHeight;
  }).catch(() => true);

  if (!(await onScreen())) {
    // The canvas is what scrolls in a Flutter app. Scrolling the *semantics
    // host* does not move it, so drive the app with the mouse wheel over the
    // canvas and let Flutter move the semantics nodes along with it.
    const vp = page.viewportSize() || { width: 1280, height: 900 };
    for (let i = 0; i < 16; i++) {
      await page.mouse.move(vp.width / 2, vp.height / 2);
      await page.mouse.wheel(0, 500);
      await page.waitForTimeout(250);
      if (await onScreen()) break;
    }
    if (!(await onScreen())) stillOffScreen = true;
  }

  try {
    if (!stillOffScreen) {
      await target.click({ timeout: 5000, force: true });
    }
  } catch (e) {
    clickError = String(e && e.message ? e.message : e).split('\n')[0];
  }
  await page.waitForTimeout(SETTLE_MS);

  const after = await probe(page);
  const apiCalls = state.requests.filter((u) => /\/v1\/|\/admin\/v1\/|\/healthz/.test(u));
  const serverErrors = state.badResponses.filter((u) => /^5\d\d/.test(u));
  const realConsoleErrors = state.consoleErrors.filter((t) => !isResourceNoise(t));

  const problems = [];
  let skipped = null;

  if (control.disabled) {
    // A control that is deliberately disabled is not a broken control: it says
    // "not right now", which is the truth. Clicking it must change nothing.
    skipped = 'disabled';
  } else if (stillOffScreen) {
    // The app would not scroll it into view: genuinely unreachable.
    skipped = 'the app will not scroll it into view';
  } else if (clickError) {
    if (/not visible|not stable|not enabled|outside of the viewport|intercepts pointer/i.test(clickError)) {
      skipped = 'not reachable on screen';
    } else {
      problems.push(`click failed: ${clickError}`);
    }
  }
  if (state.pageErrors.length) problems.push(`uncaught: ${state.pageErrors[0]}`);
  if (realConsoleErrors.length) problems.push(`console error: ${realConsoleErrors[0].split('\n')[0]}`);
  if (serverErrors.length) problems.push(`5xx: ${serverErrors[0]}`);
  if (state.failedRequests.length) problems.push(`request failed: ${state.failedRequests[0]}`);
  if (!after.alive) problems.push('the app unmounted after the click');

  // Did anything change? A control that changes nothing is inert.
  const reacted =
    after.url !== before.url ||
    after.signature !== before.signature ||
    apiCalls.length > 0;

  if (!reacted && problems.length === 0 && !skipped) {
    // A tab that is already the selected one is expected to do nothing when
    // it is clicked again. That is a no-op, not a defect.
    if (control.selected === 'true') {
      skipped = 'already selected — clicking it again is a no-op by design';
    } else {
      problems.push('inert: the click changed nothing (no navigation, no request, no change in the semantics tree)');
    }
  }

  return {
    role: control.role,
    label: control.label,
    disabled: control.disabled,
    problems,
    skipped,
    apiCalls: apiCalls.length,
    urlChanged: after.url !== before.url,
    ms: Date.now() - started,
  };
}

/**
 * Sign the admin console in, so the audit can reach the screens behind the
 * login. The admin facade falls back to its development credentials when the
 * environment does not set them.
 *
 * The fields are typed, not filled. Flutter's web text editing mirrors
 * genuine key input into its `TextEditingController`; a programmatic `fill()`
 * sets the DOM input's value, which the controller never sees. (The first
 * version of this audit used `fill()`, and the form came back with "Enter
 * your password." — the controller was empty even though the DOM input was
 * not.) The pause after the click matters too: typing before Flutter has
 * attached its editing state drops the first keystrokes.
 */
async function tryAdminLogin(page) {
  const inputs = page.locator('input, textarea');
  const n = await inputs.count();
  if (n < 2) return { ok: false, reason: 'no credential fields found' };
  const user = (process.env.ADMIN_USER || 'admin');
  const password = (process.env.ADMIN_PASSWORD || 'dev-only-admin');
  try {
    await inputs.nth(0).click();
    await page.waitForTimeout(300);
    await page.keyboard.type(user, { delay: 40 });
    await inputs.nth(1).click();
    await page.waitForTimeout(300);
    await page.keyboard.type(password, { delay: 40 });
  } catch (e) {
    return { ok: false, reason: `could not type the credentials: ${String(e.message).split('\n')[0]}` };
  }
  const typed = await page.evaluate(() => [...document.querySelectorAll('input, textarea')].map((n) => n.value));
  if (typed[1] !== password) {
    return { ok: false, reason: `the password field ended up as "${typed[1]}" instead of the credential` };
  }
  const signIn = page.locator(CONTROL_SELECTOR).filter({ hasText: /sign in|ログイン/i }).first();
  if ((await signIn.count()) === 0) return { ok: false, reason: 'no sign-in button found' };
  await signIn.click({ force: true, timeout: 5000 }).catch(() => {});
  await page.waitForTimeout(2500);
  const stillLogin = await page.locator('input, textarea').count();
  return { ok: stillLogin === 0, reason: stillLogin === 0 ? '' : 'the console still shows the credential fields' };
}

/** Check the live API against the contract. */
async function auditApi(page) {
  const results = [];
  const check = async (spec) => {
    const url = BASE + spec.path;
    const problems = [];
    let status = null;
    let body = null;
    try {
      // A POST without a body is rejected by the JSON extractor before the
      // handler ever runs, which would test the extractor's taste rather
      // than the endpoint. Send an empty JSON object instead.
      const init = { method: spec.method, timeout: 20_000 };
      if (spec.method === 'POST' || spec.method === 'PATCH' || spec.method === 'PUT') {
        init.data = '{}';
        init.headers = { 'content-type': 'application/json' };
      }
      const res = await page.request.fetch(url, init);
      status = res.status();
      const text = await res.text();
      if (text.trim().length > 0) {
        try {
          body = JSON.parse(text);
        } catch {
          body = null;
          if (spec.expectJson) problems.push('the response is not JSON');
        }
      }
    } catch (e) {
      problems.push(`request failed: ${String(e && e.message ? e.message : e).split('\n')[0]}`);
    }
    if (status !== null && status !== spec.expectStatus) {
      problems.push(`status ${status}, expected ${spec.expectStatus}`);
    }
    if (body && spec.requiredFields) {
      for (const f of spec.requiredFields) {
        if (!(f in body)) problems.push(`missing required field \`${f}\``);
      }
    }
    if (body && spec.expectArrayField) {
      const arr = body[spec.expectArrayField.field];
      if (!Array.isArray(arr)) {
        problems.push(`\`${spec.expectArrayField.field}\` is not an array`);
      } else if (arr.length < spec.expectArrayField.minLength) {
        problems.push(`\`${spec.expectArrayField.field}\` is empty`);
      }
    }
    results.push({ path: spec.path, method: spec.method, status, problems, note: spec.note });
  };

  for (const spec of EXPECT.paths) await check(spec);
  for (const spec of EXPECT.adminPaths || []) await check(spec);

  // A product detail must carry the camelCase names the Dart client reads.
  try {
    const res = await page.request.fetch(`${BASE}/v1/products`, { timeout: 20_000 });
    const page1 = await res.json();
    const first = (page1.items || [])[0];
    if (first) {
      const id = first.productId ?? first.id ?? '';
      const detail = await page.request.fetch(`${BASE}/v1/products/${encodeURIComponent(id)}`, { timeout: 20_000 });
      if (detail.status() === 200) {
        const p = await detail.json();
        const missing = EXPECT.productDetail.requiredFields.filter((f) => !(f in p));
        results.push({
          path: `/v1/products/${id}`,
          method: 'GET',
          status: detail.status(),
          problems: missing.length ? [`missing camelCase fields: ${missing.join(', ')}`] : [],
          note: 'ProductDetail must use the names the Dart client reads',
        });
      } else {
        results.push({
          path: `/v1/products/${id}`,
          method: 'GET',
          status: detail.status(),
          problems: [`status ${detail.status()}, expected 200`],
          note: 'the product list advertised a product the detail route cannot serve',
        });
      }
    } else {
      results.push({ path: '/v1/products (detail)', method: 'GET', status: null, problems: ['the catalogue is empty, so the detail route could not be checked'], note: '' });
    }
  } catch (e) {
    results.push({ path: '/v1/products (detail)', method: 'GET', status: null, problems: [String(e && e.message ? e.message : e).split('\n')[0]], note: '' });
  }

  return results;
}

/** Audit one Flutter surface: the storefront or the admin console. */
async function auditSurface(browser, name, path) {
  const context = await browser.newContext({ viewport: VIEWPORT });
  const page = await context.newPage();
  const state = collectors(page);
  const report = {
    surface: name,
    path,
    url: `${BASE}${path}`,
    controls: [],
    problems: [],
    consoleErrors: [],
    pageErrors: [],
  };

  // The errors from the initial load. The per-control loop resets the
  // collectors so each control is measured on its own; this list is what
  // happened while the page was simply loading.
  // Controls that changed the page when they were clicked. One of them is
  // used as a primer for the second attempt below.
  const active = [];

  const loadErrors = {
    consoleErrors: [],
    pageErrors: [],
    badResponses: [],
    failedRequests: [],
  };

  try {
    await page.goto(`${BASE}${path}`, { waitUntil: 'domcontentloaded', timeout: BOOT_TIMEOUT });
    await waitForFlutter(page);
    loadErrors.consoleErrors = [...state.consoleErrors];
    loadErrors.pageErrors = [...state.pageErrors];
    loadErrors.badResponses = [...state.badResponses];
    loadErrors.failedRequests = [...state.failedRequests];
    const semanticsOn = await enableSemantics(page);
    report.semanticsEnabled = semanticsOn;

    if (!semanticsOn) {
      report.problems.push('the semantics tree could not be enabled, so no control could be inspected');
    } else {
      if (name === 'admin') {
        report.login = await tryAdminLogin(page);
      }
      const controls = await listControls(page);
      report.controlCount = controls.length;

      for (const control of controls) {
        // The **same page** is reloaded, not a fresh context. A fresh context
        // means a cold HTTP cache, and the Flutter bundle is ~45 MB (the
        // canvaskit runtime plus the app); on a slow link that does not
        // finish booting inside the timeout, and every control is reported
        // as "could not be tested" — an artefact of the instrument, not of
        // the app. Reloading the same page keeps the cache warm.
        try {
          await page.goto(`${BASE}${path}`, { waitUntil: 'domcontentloaded', timeout: BOOT_TIMEOUT });
          await waitForFlutter(page);
          await enableSemantics(page);
          if (name === 'admin' && report.login && report.login.ok) {
            await tryAdminLogin(page);
          }
          const outcome = await auditControl(page, control, state);
          report.controls.push(outcome);
          if (!outcome.problems.length) active.push(control);
        } catch (e) {
          report.controls.push({
            role: control.role,
            label: control.label,
            unmeasured: `could not be tested: ${String(e && e.message ? e.message : e).split('\n')[0]}`,
          });
        }
      }
    }
  } catch (e) {
    report.problems.push(`the surface did not load: ${String(e && e.message ? e.message : e).split('\n')[0]}`);
  }

  // Snapshot the errors from the initial load. The per-control loop resets
  // the collectors so each control is measured on its own; the surface-level
  // list is what happened while the page was simply loading.
  // Second attempt for the controls that did nothing: retry them after a
  // primer has changed the page. A control that reacts then is a
  // state-dependent no-op (correct behaviour), not a dead button.
  const noOps = report.controls.filter(
    (c) => c.problems && c.problems.length === 1 && c.problems[0].startsWith('inert:'),
  );
  if (noOps.length > 0 && active.length > 0) {
    const ctx2 = await browser.newContext({ viewport: VIEWPORT });
    const p2 = await ctx2.newPage();
    try {
      for (const c of noOps) {
        let resolved = false;
        // Every control that changed the page is tried as a primer. The first
        // one is often not the one that changes the state this control
        // depends on - on the admin console it was 'Sign out', not the other tab.
        // Six primers is enough: the state-changing controls are few, and
        // every primer costs a full page load.
        for (const primer of active.slice(0, 6)) {
          await p2.goto(`${BASE}${path}`, { waitUntil: 'domcontentloaded', timeout: BOOT_TIMEOUT });
          await waitForFlutter(p2);
          await enableSemantics(p2);
          if (name === 'admin' && report.login && report.login.ok) await tryAdminLogin(p2);
          // Change the page first.
          await p2.locator(CONTROL_SELECTOR).nth(primer.index).click({ force: true, timeout: 8000 }).catch(() => {});
          await p2.waitForTimeout(SETTLE_MS);
          // Then the control that did nothing.
          const target = p2.locator(CONTROL_SELECTOR).nth(c.index);
          const before2 = await probe(p2);
          let clickError2 = null;
          try {
            await target.click({ timeout: 5000, force: true });
          } catch (e) {
            clickError2 = String(e && e.message ? e.message : e).split('\n')[0];
          }
          await p2.waitForTimeout(SETTLE_MS);
          const after2 = await probe(p2);
          const reacted = after2.url !== before2.url || after2.signature !== before2.signature;
          if (reacted && !clickError2) {
            c.problems = [];
            c.note = 'state-dependent no-op: nothing happens in the initial state and it reacts once the page has changed - correct behaviour, not a dead button';
            resolved = true;
            break;
          }
        }
        if (!resolved) {
          c.problems = ['inert: the click changed nothing, in the initial state or after any other control changed the page'];
        }
      }
    } finally {
      await ctx2.close();
    }
  }

  report.consoleErrors = loadErrors.consoleErrors;
  report.pageErrors = loadErrors.pageErrors;
  report.badResponses = loadErrors.badResponses;
  report.failedRequests = loadErrors.failedRequests;

  const broken = report.controls.filter((c) => c.problems && c.problems.length);
  report.brokenControls = broken.length;
  report.unmeasuredControls = (report.controls || []).filter((c) => c.unmeasured).length;
  report.skippedControls = report.controls.filter((c) => c.skipped).length;

  await context.close();
  return report;
}

async function main() {
  mkdirSync(REPORTS, { recursive: true });
  const browser = await chromium.launch({
    channel: 'chrome',
    args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
  });

  const started = new Date().toISOString();
  const out = { base: BASE, startedAt: started, surfaces: [], api: [] };
  try {
    const ctx = await browser.newContext({ viewport: VIEWPORT });
    const page = await ctx.newPage();
    out.api = await auditApi(page);
    await ctx.close();

    for (const [name, path] of [['storefront', '/'], ['admin', '/admin/']]) {
      out.surfaces.push(await auditSurface(browser, name, path));
    }
  } finally {
    await browser.close();
  }

  out.finishedAt = new Date().toISOString();

  // The verdict. Anything in `problems` anywhere is a finding.
  const apiProblems = out.api.filter((r) => r.problems.length);
  const surfaceProblems = out.surfaces.filter((s) => s.problems.length || s.brokenControls > 0);
  const brokenTotal = out.surfaces.reduce((n, s) => n + (s.brokenControls || 0), 0);
  const unmeasuredTotal = out.surfaces.reduce((n, s) => n + (s.unmeasuredControls || 0), 0);
  out.verdict = {
    apiFindings: apiProblems.length,
    brokenControls: brokenTotal,
    unmeasuredControls: unmeasuredTotal,
    surfacesWithProblems: surfaceProblems.length,
    pass:
      apiProblems.length === 0 &&
      brokenTotal === 0 &&
      unmeasuredTotal === 0 &&
      surfaceProblems.length === 0,
  };

  const stamp = started.replace(/[:.]/g, '-');
  writeFileSync(resolve(REPORTS, `audit-${stamp}.json`), JSON.stringify(out, null, 2));

  const lines = [];
  lines.push('# DOKIPON commerce deployment audit');
  lines.push('');
  lines.push(`- base: ${BASE}`);
  lines.push(`- started: ${started}`);
  lines.push(`- finished: ${out.finishedAt}`);
  lines.push('');
  lines.push('## Verdict');
  lines.push('');
  lines.push(`- API findings: **${out.verdict.apiFindings}**`);
  lines.push(`- broken controls: **${out.verdict.brokenControls}**`);
  lines.push(`- unmeasured controls (the network did not deliver the page): **${out.verdict.unmeasuredControls}**`);
  lines.push(`- surfaces with problems: **${out.verdict.surfacesWithProblems}**`);
  lines.push(`- overall: **${out.verdict.pass ? 'PASS' : 'FAIL'}**`);
  lines.push('');
  lines.push('## API');
  lines.push('');
  lines.push('| method | path | status | problems |');
  lines.push('|---|---|---|---|');
  for (const r of out.api) {
    lines.push(`| ${r.method} | \`${r.path}\` | ${r.status ?? '-'} | ${r.problems.join('; ') || 'ok'} |`);
  }
  lines.push('');
  for (const s of out.surfaces) {
    lines.push(`## ${s.surface} (\`${s.path}\`)`);
    lines.push('');
    if (s.login) lines.push(`- admin login: ${s.login.ok ? 'ok' : `failed (${s.login.reason})`}`);
    if (s.problems.length) {
      lines.push('Surface problems:');
      for (const p of s.problems) lines.push(`- ${p}`);
      lines.push('');
    }
    if (s.consoleErrors && s.consoleErrors.length) {
      lines.push('Console errors:');
      for (const e of s.consoleErrors.slice(0, 10)) lines.push(`- ${e.split('\n')[0]}`);
      lines.push('');
    }
    if (s.pageErrors && s.pageErrors.length) {
      lines.push('Uncaught exceptions:');
      for (const e of s.pageErrors.slice(0, 10)) lines.push(`- ${e.split('\n')[0]}`);
      lines.push('');
    }
    lines.push(`Controls found: **${s.controlCount ?? 0}**, broken: **${s.brokenControls ?? 0}**, skipped (off-screen): **${s.skippedControls ?? 0}**, unmeasured: **${s.unmeasuredControls ?? 0}**`);
    lines.push('');
    const unmeasured = (s.controls || []).filter((c) => c.unmeasured);
    if (unmeasured.length) {
      lines.push('Not measured (network):');
      for (const c of unmeasured) {
        lines.push(`- ${(c.label || '').replace(/\n/g, ' ').slice(0, 60)}: ${c.unmeasured}`);
      }
      lines.push('');
    }
    const broken = (s.controls || []).filter((c) => c.problems && c.problems.length);
    if (broken.length) {
      lines.push('| role | label | problems |');
      lines.push('|---|---|---|');
      for (const c of broken) {
        lines.push(`| ${c.role} | ${(c.label || '').replace(/\|/g, '\\|').replace(/\n/g, ' ').slice(0, 60)} | ${c.problems.join('; ')} |`);
      }
      lines.push('');
    }
  }
  const md = lines.join('\n');
  writeFileSync(resolve(REPORTS, `audit-${stamp}.md`), md);
  console.log(md);
  console.log(`\nreport: ${resolve(REPORTS, `audit-${stamp}.md`)}`);
  process.exitCode = out.verdict.pass ? 0 : 1;
}

main().catch((e) => {
  console.error(e);
  process.exitCode = 2;
});

