#!/usr/bin/env node
/**
 * audit.mjs — drive the deployed DOKIPON commerce deployment with a real
 * browser and report every control that does not work.
 *
 * ## Why this is not a DOM test
 *
 * The deployment is a Flutter Web app (the storefront at `/`, the admin
 * console at `/admin/`) on top of a Rust edge that serves the commerce
 * contract. Flutter renders to a canvas, so the buttons are not DOM elements
 * and a DOM-based test sees an empty page. The way to drive a Flutter Web
 * app is to turn on the **semantics tree**: Flutter keeps a parallel DOM
 * representation of the widget tree for screen readers, and it stays off
 * until something asks for it. Clicking the "enable accessibility"
 * placeholder is that ask, and afterwards every button, link and field is a
 * real element with a role and a label.
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

const VIEWPORTS = { width: 1280, height: 900 };
const BOOT_TIMEOUT = 45_000;
const SETTLE_MS = 900;

/** Per-target collectors. Reset between targets so one page cannot pollute the next. */
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

/** Wait for the Flutter app to be mounted and for its first frame to be on screen. */
async function waitForFlutter(page) {
  // `state: 'attached'`, not the default 'visible': Flutter's scene host is a
  // zero-size container that hosts the canvas, so it never becomes "visible"
  // to Playwright even when the app is running perfectly.
  await page.waitForSelector('flt-scene-host, flt-glass-pane, canvas', {
    state: 'attached',
    timeout: BOOT_TIMEOUT,
  });
  // The first frame is painted a tick after the scene host exists. Give the
  // engine a moment, then wait for the network to go quiet.
  await page.waitForTimeout(1500);
  await page
    .waitForLoadState('networkidle', { timeout: 15_000 })
    .catch(() => { /* a websocket or a poll keeps the network busy; that is fine */ });
}

/**
 * Turn on the semantics tree. Without it a Flutter Web app exposes no
 * inspectable controls at all.
 */
async function enableSemantics(page) {
  const placeholder = page.locator('flt-semantics-placeholder');
  if ((await placeholder.count()) === 0) return false;
  await placeholder
    .first()
    .evaluate((el) => el.click())
    .catch(() => {});
  await page.waitForTimeout(700);
  return (await page.locator('flt-semantics').count()) > 0;
}

/** Every interactive control the semantics tree exposes. */
async function listControls(page) {
  const controls = page.locator(
    'flt-semantics[role="button"], flt-semantics[role="link"], flt-semantics[role="checkbox"], flt-semantics[role="radio"], flt-semantics[role="tab"], flt-semantics[role="switch"]',
  );
  const n = await controls.count();
  const out = [];
  for (let i = 0; i < n; i++) {
    const el = controls.nth(i);
    const info = await el.evaluate((node) => ({
      role: node.getAttribute('role'),
      label: node.getAttribute('aria-label') || (node.textContent || '').trim(),
      checked: node.getAttribute('aria-checked'),
      selected: node.getAttribute('aria-selected'),
      disabled: node.getAttribute('aria-disabled') === 'true',
      w: node.getBoundingClientRect().width,
      h: node.getBoundingClientRect().height,
    }));
    out.push({ index: i, ...info });
  }
  return out;
}

/** The state of the app, read back from the page. */
async function probe(page) {
  return page.evaluate(() => ({
    url: location.href,
    sceneHost: !!document.querySelector('flt-scene-host'),
    semantics: document.querySelectorAll('flt-semantics').length,
    // Flutter renders into a canvas; a blank canvas and a painted canvas have
    // very different byte counts, so the size of the canvas bitmap is a cheap
    // proxy for "something was drawn".
    canvasNonBlank: (() => {
      const c = document.querySelector('canvas');
      if (!c) return false;
      try {
        const ctx = c.getContext('2d');
        if (!ctx) return true; // WebGL canvas: cannot read back, assume painted
        const d = ctx.getImageData(0, 0, Math.min(c.width, 200), Math.min(c.height, 200)).data;
        for (let i = 3; i < d.length; i += 4) if (d[i] !== 0) return true;
        return false;
      } catch {
        return true;
      }
    })(),
  }));
}

/**
 * Click one control and judge what happened.
 *
 * The app is reloaded before each click so that one control's effect cannot
 * be mistaken for the next one's, and so a control that navigated away does
 * not make the following controls unreachable.
 */
async function auditControl(page, control) {
  const state = collectors(page);
  const before = await probe(page);

  const target = page.locator(
    'flt-semantics[role="button"], flt-semantics[role="link"], flt-semantics[role="checkbox"], flt-semantics[role="radio"], flt-semantics[role="tab"], flt-semantics[role="switch"]',
  ).nth(control.index);

  const started = Date.now();
  let clickError = null;
  try {
    await target.click({ timeout: 5000, force: true });
  } catch (e) {
    clickError = String(e && e.message ? e.message : e).split('\n')[0];
  }
  await page.waitForTimeout(SETTLE_MS);

  const after = await probe(page);
  const apiCalls = state.requests.filter((u) => /\/v1\/|\/admin\/v1\/|\/healthz/.test(u));
  const serverErrors = state.badResponses.filter((u) => /^5\d\d/.test(u));

  const problems = [];
  if (clickError) problems.push(`click failed: ${clickError}`);
  if (state.pageErrors.length) problems.push(`uncaught: ${state.pageErrors[0]}`);
  if (state.consoleErrors.length) problems.push(`console error: ${state.consoleErrors[0]}`);
  if (serverErrors.length) problems.push(`5xx: ${serverErrors[0]}`);
  if (state.failedRequests.length) problems.push(`request failed: ${state.failedRequests[0]}`);
  if (!after.sceneHost) problems.push('the app unmounted after the click');
  if (after.url !== before.url) {
    // A navigation is a legitimate outcome, but the app must still be alive.
    if (!after.sceneHost) problems.push('navigated away and the app is gone');
  }

  // Did anything change? A control that changes nothing is inert.
  const reacted =
    after.url !== before.url ||
    after.semantics !== before.semantics ||
    apiCalls.length > 0 ||
    after.canvasNonBlank !== before.canvasNonBlank;

  if (!reacted && problems.length === 0) {
    problems.push('inert: the click changed nothing (no navigation, no request, no state change)');
  }

  return {
    role: control.role,
    label: control.label,
    disabled: control.disabled,
    problems,
    apiCalls: apiCalls.length,
    urlChanged: after.url !== before.url,
    ms: Date.now() - started,
  };
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
      const res = await page.request.fetch(url, { method: spec.method, timeout: 20_000 });
      status = res.status();
      const text = await res.text();
      try {
        body = JSON.parse(text);
      } catch {
        body = null;
        if (spec.expectJson) problems.push('the response is not JSON');
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
      const detail = await page.request.fetch(`${BASE}/v1/products/${encodeURIComponent(first.productId ?? first.id ?? '')}`, { timeout: 20_000 });
      if (detail.status() === 200) {
        const p = await detail.json();
        const missing = EXPECT.productDetail.requiredFields.filter((f) => !(f in p));
        results.push({
          path: `/v1/products/${first.productId ?? first.id}`,
          method: 'GET',
          status: detail.status(),
          problems: missing.length ? [`missing camelCase fields: ${missing.join(', ')}`] : [],
          note: 'ProductDetail must use the names the Dart client reads',
        });
      } else {
        results.push({
          path: `/v1/products/${first.productId ?? first.id}`,
          method: 'GET',
          status: detail.status(),
          problems: [`status ${detail.status()}, expected 200`],
          note: 'the product list advertised a product that the detail route cannot serve',
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
  const context = await browser.newContext({ viewport: VIEWPORTS });
  const page = await context.newPage();
  const state = collectors(page);
  const report = { surface: name, path, url: `${BASE}${path}`, controls: [], problems: [], consoleErrors: [], pageErrors: [] };

  try {
    await page.goto(`${BASE}${path}`, { waitUntil: 'domcontentloaded', timeout: BOOT_TIMEOUT });
    await waitForFlutter(page);
    const semanticsOn = await enableSemantics(page);
    report.semanticsEnabled = semanticsOn;

    if (!semanticsOn) {
      report.problems.push('the semantics tree could not be enabled, so no control could be inspected');
    } else {
      const controls = await listControls(page);
      report.controlCount = controls.length;
      for (const control of controls) {
        // A fresh page per control: one control's navigation must not hide
        // the next one, and each measurement starts from the same state.
        const ctx = await browser.newContext({ viewport: VIEWPORTS });
        const p2 = await ctx.newPage();
        try {
          await p2.goto(`${BASE}${path}`, { waitUntil: 'domcontentloaded', timeout: BOOT_TIMEOUT });
          await waitForFlutter(p2);
          await enableSemantics(p2);
          report.controls.push(await auditControl(p2, control));
        } catch (e) {
          report.controls.push({ role: control.role, label: control.label, problems: [`could not be tested: ${String(e && e.message ? e.message : e).split('\n')[0]}`] });
        } finally {
          await ctx.close();
        }
      }
    }
  } catch (e) {
    report.problems.push(`the surface did not load: ${String(e && e.message ? e.message : e).split('\n')[0]}`);
  }

  report.consoleErrors = state.consoleErrors;
  report.pageErrors = state.pageErrors;
  report.badResponses = state.badResponses;
  report.failedRequests = state.failedRequests;

  const broken = report.controls.filter((c) => c.problems && c.problems.length);
  report.brokenControls = broken.length;

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
    const ctx = await browser.newContext({ viewport: VIEWPORTS });
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
  out.verdict = {
    apiFindings: apiProblems.length,
    brokenControls: brokenTotal,
    surfacesWithProblems: surfaceProblems.length,
    pass: apiProblems.length === 0 && brokenTotal === 0 && surfaceProblems.length === 0,
  };

  mkdirSync(REPORTS, { recursive: true });
  const stamp = started.replace(/[:.]/g, '-');
  writeFileSync(resolve(REPORTS, `audit-${stamp}.json`), JSON.stringify(out, null, 2));

  const lines = [];
  lines.push(`# DOKIPON commerce deployment audit`);
  lines.push('');
  lines.push(`- base: ${BASE}`);
  lines.push(`- started: ${started}`);
  lines.push(`- finished: ${out.finishedAt}`);
  lines.push('');
  lines.push(`## Verdict`);
  lines.push('');
  lines.push(`- API findings: **${out.verdict.apiFindings}**`);
  lines.push(`- broken controls: **${out.verdict.brokenControls}**`);
  lines.push(`- surfaces with problems: **${out.verdict.surfacesWithProblems}**`);
  lines.push(`- overall: **${out.verdict.pass ? 'PASS' : 'FAIL'}**`);
  lines.push('');
  lines.push(`## API`);
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
    if (s.problems.length) {
      lines.push('Surface problems:');
      for (const p of s.problems) lines.push(`- ${p}`);
      lines.push('');
    }
    if (s.consoleErrors && s.consoleErrors.length) {
      lines.push('Console errors:');
      for (const e of s.consoleErrors.slice(0, 10)) lines.push(`- ${e}`);
      lines.push('');
    }
    if (s.pageErrors && s.pageErrors.length) {
      lines.push('Uncaught exceptions:');
      for (const e of s.pageErrors.slice(0, 10)) lines.push(`- ${e}`);
      lines.push('');
    }
    lines.push(`Controls found: **${s.controlCount ?? 0}**, broken: **${s.brokenControls ?? 0}**`);
    lines.push('');
    const broken = (s.controls || []).filter((c) => c.problems && c.problems.length);
    if (broken.length) {
      lines.push('| role | label | problems |');
      lines.push('|---|---|---|');
      for (const c of broken) {
        lines.push(`| ${c.role} | ${(c.label || '').replace(/\|/g, '\\|').slice(0, 60)} | ${c.problems.join('; ')} |`);
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
