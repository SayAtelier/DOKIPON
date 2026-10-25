/**
 * uat/lib.mjs — the harness the UAT journeys wear.
 *
 * ## What a UAT is, and what it is not
 *
 * The audit (`scripts/audit.mjs`) asks a different question: **does every
 * control do something?** It reloads the page before each control and clicks
 * it on its own, so one control's effect cannot be mistaken for the next
 * one's. That is the right shape for finding a dead button, and the wrong
 * shape for finding out whether a **shop** works.
 *
 * A UAT asks the other question: **can a person do the things a person has
 * to do?** A journey — browse, filter, open a product, fill the cart, pay,
 * and have the order turn up on the operator's screen — and the outcome,
 * not the individual click.
 *
 * So a journey here is a sequence, and its checks are about **outcomes**:
 * the catalogue narrowed to the right product, the cart holding the line the
 * server priced, the order existing afterwards, the operator seeing it.
 * A click that happens to work is not a result.
 *
 * ## The three answers
 *
 *   - `PASS` — the journey ran and every check held.
 *   - `FAIL` — a check did not hold, or the journey threw. This is a
 *     finding, and the report says which check and with what evidence.
 *   - `UNMEASURED` — the journey could not be asked. The usual reason is a
 *     missing mock: the LINE journey needs a token the deployment's
 *     identity service accepts, and a deployment whose signing key is a
 *     platform secret cannot be given one, because the key cannot be read
 *     back. A question nobody could ask is not a finding, and it is not a
 *     pass either.
 *
 * ## The expectations live in the fixture set
 *
 * The journey code reads `mock_data/local/local.json` → `uat`, never a
 * literal. A price that changes in the catalogue must not silently change
 * what the UAT expects, and a fixture that disagrees with the catalogue is
 * a finding in itself.
 */

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  bootFlutter,
  collectors,
  enableSemantics,
  launchChrome,
  openSurface,
} from '../scripts/probe-lib.mjs';

export const UAT_ROOT = dirname(fileURLToPath(import.meta.url));
export const E2E_ROOT = resolve(UAT_ROOT, '..');
export const REPO_ROOT = resolve(E2E_ROOT, '..', '..');
export const REPORTS = resolve(E2E_ROOT, 'reports');

/** The fixture set's UAT expectations. */
export function fixture() {
  const file = resolve(E2E_ROOT, 'mock_data', 'local', 'local.json');
  return JSON.parse(readFileSync(file, 'utf8')).uat;
}

/**
 * A journey.
 *
 * `needs` says what the journey cannot run without: `mock` (the LINE token
 * issuer) and `admin` (the operator credential). A journey whose
 * requirement is missing is `UNMEASURED`, never `FAIL` — the deployment did
 * not do anything wrong, the question was not asked.
 */
export function journey({ id, title, needs = {}, run }) {
  return { id, title, needs, run };
}

/** The recorder a journey writes its checks into. */
export function recorder() {
  const checks = [];
  const notes = [];
  return {
    checks,
    notes,
    /** Record one check. `detail` is the evidence, and it is printed. */
    check(name, ok, detail = '') {
      checks.push({ name, ok: Boolean(ok), detail: String(detail) });
      return Boolean(ok);
    },
    /** Something the reader should know that is not a check. */
    note(text) {
      notes.push(String(text));
    },
  };
}

/** The labels the app's semantics tree exposes. */
export async function semanticsLabels(page) {
  return page.locator('flt-semantics').evaluateAll((nodes) =>
    nodes.map((node) => ({
      role: node.getAttribute('role'),
      label: (node.getAttribute('aria-label') || node.textContent || '').trim(),
      disabled: node.getAttribute('aria-disabled') === 'true',
    })),
  );
}

/** Whether a label matches the needle — a substring, or a pattern. */
function labelMatches(label, needle) {
  return needle instanceof RegExp ? needle.test(label) : label.includes(needle);
}

/** Whether any exposed label contains the needle. */
export function hasLabel(labels, needle) {
  return labels.some((node) => labelMatches(node.label, needle));
}

/** The first exposed label that contains the needle. */
export function findLabel(labels, needle, role = null) {
  return (
    labels.find(
      (node) => labelMatches(node.label, needle) && (role === null || node.role === role),
    ) || null
  );
}

/**
 * Wait until a label shows up, and return it.
 *
 * A Flutter app paints in frames and builds its semantics tree lazily, so
 * the label a journey is waiting for may not be there yet. `null` when it
 * never arrives — which is a finding, not a crash.
 */
export async function waitForLabel(page, needle, { timeout = 30_000, role = null } = {}) {
  const deadline = Date.now() + timeout;
  for (;;) {
    const hit = findLabel(await semanticsLabels(page), needle, role);
    if (hit) return hit;
    if (Date.now() >= deadline) return null;
    await page.waitForTimeout(300);
  }
}

/**
 * Scroll until the element is on the screen, and say whether it got there.
 *
 * Flutter renders into a canvas, so Playwright's own "scroll the element
 * into view" does nothing: the element it would scroll is the semantics
 * node, and the thing that scrolls is the canvas. The node's own rectangle
 * is still correct, though, so the canvas is scrolled with the mouse wheel
 * until the rectangle is on the screen.
 *
 * `false` when the element could not be brought on the screen.
 */
export async function ensureInView(page, locator, { rounds = 14, step = 450 } = {}) {
  const viewport = page.viewportSize() || { width: 1280, height: 900 };
  for (let i = 0; i < rounds; i++) {
    let rect = null;
    try {
      rect = await locator.evaluate(
        (node) => {
          const r = node.getBoundingClientRect();
          return {
            top: r.top,
            bottom: r.bottom,
            left: r.left,
            right: r.right,
            height: r.height,
            width: r.width,
          };
        },
        { timeout: 5000 },
      );
    } catch {
      // The element is not in the tree right now (the tree is rebuilt as the
      // app lays out). Look again on the next round.
      rect = null;
    }
    if (
      rect &&
      rect.height > 0 &&
      rect.top >= 0 &&
      rect.bottom <= viewport.height &&
      rect.left >= 0 &&
      rect.right <= viewport.width
    ) {
      return true;
    }
    // The wheel goes to whatever is under the pointer, so the pointer is
    // moved to the edge the element is off: the operator console's order
    // table scrolls sideways, and its "Open" button is off to the right
    // until the table is scrolled.
    const offRight = rect && rect.right > viewport.width;
    const offLeft = rect && rect.left < 0;
    const x = offRight
      ? viewport.width - 8
      : offLeft
        ? 8
        : Math.max(0, Math.min(viewport.width - 1, rect ? (rect.left + rect.right) / 2 : viewport.width / 2));
    const y = Math.max(0, Math.min(viewport.height - 1, rect ? (rect.top + rect.bottom) / 2 : viewport.height / 2));
    const dx = offRight ? step : offLeft ? -step : 0;
    const dy = rect && rect.top < 0 ? -step : rect && rect.bottom > viewport.height ? step : 0;
    if (dx === 0 && dy === 0) return false;
    await page.mouse.move(x, y);
    await page.mouse.wheel(dx, dy);
    await page.waitForTimeout(350);
  }
  return false;
}

/**
 * Click a control by its accessible name.
 *
 * The name is the stable thing: a Flutter app rebuilds its semantics tree
 * whenever the catalogue arrives, and an index found before the rebuild
 * points at a different element. The name resolves itself.
 */
export async function clickControl(page, role, label, { timeout = 20_000 } = {}) {
  const name = label.replace(/\s+/g, ' ').trim();
  const locator = page.getByRole(role, { name, exact: true }).first();
  await locator.waitFor({ state: 'attached', timeout });
  await ensureInView(page, locator);
  await locator.click({ timeout, force: true });
  return locator;
}

/**
 * Click a control whose name matches a pattern.
 *
 * Some names carry a count ("カート / Cart, 1 items"), so the journey
 * cannot name them exactly. The pattern is matched against the accessible
 * name, which is what a person reads.
 */
export async function clickMatching(page, role, pattern, { timeout = 20_000 } = {}) {
  const locator = page.getByRole(role, { name: pattern }).first();
  await locator.waitFor({ state: 'attached', timeout });
  await ensureInView(page, locator);
  await locator.click({ timeout, force: true });
  return locator;
}

/**
 * Wait until the catalogue is on the page, and return the first product of
 * it that is.
 *
 * A journey that reads the tree the moment the app has mounted reads it
 * too early: the bundle is still booting, the catalogue has not been laid
 * out, and the tree is empty. On a fast local stack that race is lost
 * silently; on a slow link it is a failed check that says nothing about
 * the app.
 */
export async function waitForCatalogue(page, products, { timeout = 60_000 } = {}) {
  const deadline = Date.now() + timeout;
  for (;;) {
    const labels = await semanticsLabels(page);
    const hit = products.find((product) => findLabel(labels, product.title, 'button'));
    if (hit) return hit;
    if (Date.now() >= deadline) return null;
    await page.waitForTimeout(500);
  }
}

/**
 * Scroll the page to the end.
 *
 * Flutter renders into a canvas, so the semantics tree is built lazily: the
 * products further down the grid are not in the tree until the canvas has
 * been scrolled there. A journey that counts the catalogue has to scroll
 * before it counts.
 */
export async function scrollToEnd(page, { rounds = 8, step = 700 } = {}) {
  const viewport = page.viewportSize() || { width: 1280, height: 900 };
  for (let i = 0; i < rounds; i++) {
    await page.mouse.move(viewport.width / 2, viewport.height / 2);
    await page.mouse.wheel(0, step);
    await page.waitForTimeout(350);
  }
  await page.waitForTimeout(800);
}

/**
 * Scroll the page back to the top.
 *
 * The tree is built lazily in **both** directions: what is off the screen
 * is not in it. A journey that scrolls down to count the catalogue and then
 * wants the navigation strip has to come back up first — the strip is not
 * there after the scroll, and a control that is not in the tree cannot be
 * clicked.
 */
export async function scrollToTop(page, { rounds = 8, step = 700 } = {}) {
  const viewport = page.viewportSize() || { width: 1280, height: 900 };
  for (let i = 0; i < rounds; i++) {
    await page.mouse.move(viewport.width / 2, viewport.height / 2);
    await page.mouse.wheel(0, -step);
    await page.waitForTimeout(300);
  }
  await page.waitForTimeout(800);
}

/**
 * Click a product card by its title.
 *
 * The card's accessible name is the whole card — the title, the price and
 * the stock state, joined — so a journey cannot name it exactly. The title
 * is a substring of it, and the title is what the fixture set knows.
 */
export async function clickProduct(page, title, { timeout = 20_000 } = {}) {
  const escaped = title.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const locator = page.getByRole('button', { name: new RegExp(escaped) }).first();
  await locator.waitFor({ state: 'attached', timeout });
  await ensureInView(page, locator);
  await locator.click({ timeout, force: true });
  return locator;
}

/** A JSON HTTP client with the shape the checks use. */
export function apiClient(base) {
  return {
    async fetch(path, { method = 'GET', headers = {}, body, timeout = 20_000 } = {}) {
      const response = await fetch(`${base}${path}`, {
        method,
        // The content type matters: a service that extracts JSON refuses a
        // body that does not say it is JSON, and it says so with a 422 that
        // looks like a bad payload rather than a bad request.
        headers: { accept: 'application/json', 'content-type': 'application/json', ...headers },
        body: body === undefined ? undefined : JSON.stringify(body),
        signal: AbortSignal.timeout(timeout),
      });
      const text = await response.text();
      let json = null;
      try {
        json = text ? JSON.parse(text) : null;
      } catch {
        json = null;
      }
      return { status: response.status, json, text };
    },
  };
}

/**
 * Run the journeys.
 *
 * Every journey gets its own page, so one journey's page state cannot be
 * mistaken for another's, and one journey's failure cannot take the rest
 * down. The browser context is shared, because a journey may hand the next
 * one something (the order the LINE journey placed, the credential the
 * operator journey needs).
 */
export async function runJourneys(journeys, ctx) {
  const results = [];
  for (const j of journeys) {
    const result = {
      id: j.id,
      title: j.title,
      status: 'pass',
      checks: [],
      notes: [],
      error: null,
      reason: null,
    };

    if (j.needs.mock && !ctx.mock) {
      result.status = 'unmeasured';
      result.reason =
        'the LINE token issuer is not usable against this target: the mock mints a token, ' +
        'and the target did not exchange it for a session — a deployment whose LINE signing ' +
        'key is a platform secret refuses every token a test can sign, because the key ' +
        'cannot be read back';
      results.push(result);
      continue;
    }
    if (j.needs.admin && !ctx.admin) {
      result.status = 'unmeasured';
      result.reason = 'the operator credential is not set, so the console behind the login cannot be reached';
      results.push(result);
      continue;
    }

    const t = recorder();
    let page = null;
    try {
      page = await ctx.context.newPage();
      const state = collectors(page);
      // The session the app got, and the exchange that produced it. The
      // exchange is kept because "the sign-in did not happen" is not an
      // answer: the identity service's own refusal says why.
      const session = { id: null, kind: null, exchange: null };
      page.on('response', async (response) => {
        const url = response.url();
        if (!/\/v1\/sessions\//.test(url)) return;
        let body = null;
        try {
          body = await response.json();
        } catch {
          /* a session answer we cannot read is not a session we can use */
        }
        if (body && body.sessionId) {
          session.id = body.sessionId;
          session.kind = body.kind || null;
        }
        if (/\/v1\/sessions\/line/.test(url)) {
          session.exchange = { status: response.status(), body };
        }
      });
      await j.run({ ...ctx, t, page, state, session });
      result.checks = t.checks;
      result.notes = t.notes;
      result.status = t.checks.some((check) => !check.ok) ? 'fail' : 'pass';
    } catch (error) {
      result.checks = t.checks;
      result.notes = t.notes;
      result.status = 'fail';
      result.error = String(error && error.message ? error.message : error)
        .split('\n')
        .slice(0, 3)
        .join(' | ');
    } finally {
      if (page) await page.close().catch(() => {});
    }
    results.push(result);
  }
  return results;
}

/** The Markdown report. */
export function reportMarkdown({ base, mock, results, startedAt, finishedAt }) {
  const lines = [];
  lines.push('# DOKIPON commerce — user acceptance test');
  lines.push('');
  lines.push(`- target: ${base}`);
  lines.push(`- the LINE token issuer: ${mock || '(not available)'}`);
  lines.push(`- started: ${startedAt}`);
  lines.push(`- finished: ${finishedAt}`);
  lines.push('');

  const failed = results.filter((r) => r.status === 'fail');
  const unmeasured = results.filter((r) => r.status === 'unmeasured');
  const passed = results.filter((r) => r.status === 'pass');
  lines.push('## Verdict');
  lines.push('');
  lines.push(`- journeys passed: **${passed.length}**`);
  lines.push(`- journeys failed: **${failed.length}**`);
  lines.push(`- journeys not measured: **${unmeasured.length}**`);
  lines.push(`- checks: **${results.reduce((n, r) => n + r.checks.length, 0)}**`);
  lines.push(`- overall: **${failed.length === 0 ? 'PASS' : 'FAIL'}**`);
  lines.push('');

  // The findings a journey recorded. A journey's note is something the
  // reader has to act on, so it belongs at the top of the report, not
  // buried under the journey that noticed it.
  const notes = results.flatMap((result) =>
    result.notes.map((note) => ({ journey: result.title, note })),
  );
  if (notes.length) {
    lines.push('## Findings');
    lines.push('');
    for (const finding of notes) {
      lines.push(`- **${finding.journey}** — ${finding.note}`);
    }
    lines.push('');
  }

  for (const result of results) {
    lines.push(`## ${result.title}`);
    lines.push('');
    lines.push(`- id: \`${result.id}\``);
    lines.push(`- status: **${result.status.toUpperCase()}**`);
    if (result.reason) lines.push(`- why it was not measured: ${result.reason}`);
    if (result.error) lines.push(`- the journey threw: ${result.error}`);
    lines.push('');
    if (result.checks.length) {
      lines.push('| check | result | evidence |');
      lines.push('|---|---|---|');
      for (const check of result.checks) {
        lines.push(
          `| ${check.name} | ${check.ok ? 'PASS' : '**FAIL**'} | ${(check.detail || '')
            .replace(/\|/g, '\\|')
            .replace(/\n/g, ' ')} |`,
        );
      }
      lines.push('');
    }
    for (const note of result.notes) lines.push(`> ${note}`);
    if (result.notes.length) lines.push('');
  }

  return lines.join('\n');
}

/** Write the report and print it. */
export function writeReport(markdown, stamp) {
  mkdirSync(REPORTS, { recursive: true });
  const file = resolve(REPORTS, `uat-${stamp}.md`);
  writeFileSync(file, markdown);
  return file;
}

export { bootFlutter, collectors, enableSemantics, launchChrome, openSurface };
