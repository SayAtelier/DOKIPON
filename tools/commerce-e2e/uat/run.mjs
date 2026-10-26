#!/usr/bin/env node
/**
 * uat/run.mjs — the UAT's one entry point.
 *
 * ```
 * node tools/commerce-e2e/uat/run.mjs
 * DOKIPON_BASE=https://... node tools/commerce-e2e/uat/run.mjs
 * ```
 *
 * ## What it runs
 *
 * The journeys, in order. Each one is a thing a person does, from the first
 * click to the outcome; the outcome is what is checked, not the click.
 *
 * ## What it needs
 *
 * - a deployment: `DOKIPON_BASE` (the live one by default);
 * - a browser: the system Chrome, the same one the audit drives;
 * - the **operator credential**: `ADMIN_USER` / `ADMIN_PASSWORD`, or the
 *   credentials file the deploy step wrote. Without it the operator
 *   journey is `UNMEASURED`, not `FAIL`;
 * - the **LINE token issuer**: the mock inside the local stack. A
 *   deployment whose signing key is a platform secret cannot be given a
 *   token by a test, because the key cannot be read back — so the LINE
 *   journey is `UNMEASURED` there, and it says so.
 *
 * ## The exit code
 *
 * 0 = every journey that could be asked was asked and held; 1 = a journey
 * failed; 2 = the run itself could not happen.
 */

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { MOCK_PROCESSES } from '../local/config.mjs';
import {
  E2E_ROOT,
  apiClient,
  fixture,
  launchChrome,
  reportMarkdown,
  runJourneys,
  writeReport,
} from './lib.mjs';
import { guestBrowse } from './journeys/journey-guest-browse.mjs';
import { linePurchase } from './journeys/journey-line-purchase.mjs';
import { operatorView } from './journeys/journey-operator-view.mjs';
import { paymentRefused } from './journeys/journey-payment-refused.mjs';
import { sessionSurvivesReload } from './journeys/journey-session-survives-reload.mjs';
import { stockStates } from './journeys/journey-stock-states.mjs';

const BASE = (process.env.DOKIPON_BASE || 'https://dokipon-commerce.vercel.app').replace(/\/$/, '');

/** The operator credential: the environment first, the credentials file second. */
function adminCredential() {
  if (process.env.ADMIN_PASSWORD) {
    return { user: process.env.ADMIN_USER || 'admin', password: process.env.ADMIN_PASSWORD };
  }
  const file = resolve(E2E_ROOT, 'reports', '.deploy-credentials.json');
  try {
    const stored = JSON.parse(readFileSync(file, 'utf8'));
    if (stored.ADMIN_PASSWORD) {
      return { user: stored.ADMIN_USER || 'admin', password: stored.ADMIN_PASSWORD };
    }
  } catch {
    /* no credentials file: the journey is unmeasured, not failed */
  }
  return null;
}

/**
 * The mock issuer, **if this target accepts its tokens**.
 *
 * "The mock answers" is not enough: the mock on the machine signs with the
 * local stack's key, and a deployment whose key is a platform secret
 * refuses every token it mints. So the mock is usable only when the
 * handshake works — the mock mints, and the target exchanges it for a
 * session.
 *
 * That is the whole point of the mock↔product agreement, and it is checked
 * here rather than assumed: a target that refuses the token gets the LINE
 * journey marked `UNMEASURED`, not a failed one.
 */
async function mockFor(base) {
  const url = MOCK_PROCESSES.lineIdentity.url;
  let minted = null;
  try {
    const response = await fetch(`${url}/mockline/mint?flavour=valid`, {
      signal: AbortSignal.timeout(5_000),
    });
    if (!response.ok) return null;
    minted = await response.json();
  } catch {
    return null;
  }
  if (!minted || !minted.idToken) return null;
  try {
    const exchanged = await fetch(`${base}/v1/sessions/line`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ id_token: minted.idToken, channel: 'web' }),
      signal: AbortSignal.timeout(20_000),
    });
    return exchanged.status === 200 ? url : null;
  } catch {
    return null;
  }
}

// The order matters: the reload journey places an order, and the operator
// journey is the other side of that same order.
const JOURNEYS = [
  guestBrowse,
  stockStates,
  paymentRefused,
  sessionSurvivesReload,
  linePurchase,
  operatorView,
];

async function main() {
  const startedAt = new Date().toISOString();
  console.log(`# DOKIPON commerce — user acceptance test\n`);
  console.log(`- target: ${BASE}`);

  const admin = adminCredential();
  console.log(`- the operator credential: ${admin ? 'available' : 'not available'}`);

  const mock = await mockFor(BASE);
  console.log(`- the LINE token issuer: ${mock || 'not usable against this target'}`);
  console.log('');

  const browser = await launchChrome();
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  let results = [];
  try {
    results = await runJourneys(JOURNEYS, {
      base: BASE,
      api: apiClient(BASE),
      admin,
      mock,
      context,
      fixture: fixture(),
    });
  } finally {
    await context.close().catch(() => {});
    await browser.close().catch(() => {});
  }

  for (const result of results) {
    const mark = result.status === 'pass' ? 'PASS' : result.status === 'fail' ? 'FAIL' : 'UNMEASURED';
    console.log(`${mark}  ${result.title}`);
    for (const check of result.checks) {
      console.log(`      ${check.ok ? 'PASS' : 'FAIL'}  ${check.name}${check.detail ? ` — ${check.detail}` : ''}`);
    }
    if (result.reason) console.log(`      ${result.reason}`);
    if (result.error) console.log(`      the journey threw: ${result.error}`);
  }

  const finishedAt = new Date().toISOString();
  const markdown = reportMarkdown({ base: BASE, mock, results, startedAt, finishedAt });
  const stamp = startedAt.replace(/[:.]/g, '-');
  const file = writeReport(markdown, stamp);

  const failed = results.filter((r) => r.status === 'fail').length;
  const unmeasured = results.filter((r) => r.status === 'unmeasured').length;
  const checks = results.reduce((n, r) => n + r.checks.length, 0);
  const failedChecks = results.reduce(
    (n, r) => n + r.checks.filter((c) => !c.ok).length,
    0,
  );

  console.log('');
  console.log(`journeys: ${results.length}, checks: ${checks}, failed checks: ${failedChecks}`);
  console.log(`journeys failed: ${failed}, journeys not measured: ${unmeasured}`);
  console.log(`overall: ${failed === 0 ? 'PASS' : 'FAIL'}`);
  console.log(`report: ${file}`);

  process.exitCode = failed === 0 ? 0 : 1;
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 2;
});
