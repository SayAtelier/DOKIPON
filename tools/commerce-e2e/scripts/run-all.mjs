#!/usr/bin/env node
/**
 * run-all.mjs — the one entry point for the DOKIPON commerce e2e suite.
 *
 * ## What it does
 *
 * It runs every step of the suite and prints **one** summary: what ran, what
 * passed, what failed, what could not be measured, and the total. The exit
 * code is the gate:
 *
 *   0   every step ran and every step passed
 *   1   at least one step found a defect in the deployment
 *   2   no defect was found, but at least one question could not be
 *       answered (the link dropped, the credential was not available, a
 *       dependency was missing)
 *
 * The difference between 1 and 2 is the difference between "the deployment
 * is wrong" and "nobody found out". Both are non-zero, because a suite that
 * answers nothing must not look like a suite that found nothing.
 *
 * ## The steps
 *
 *   static   the two checkers that read the repository, not the deployment
 *   audit    `audit.mjs` — the contract and every control on both surfaces
 *   verify   `verify-deployment.mjs` — the facts the contract cannot see
 *   probe    the questions the audit cannot answer, one probe each
 *
 * ## The browser
 *
 * The audit launches its own Chrome: it is a self-contained script and its
 * findings must not depend on how the suite runs it. The browser probes
 * share **one** Chrome and **one** browser context, and each probe gets a
 * page of its own from that context — so the Flutter bundle is fetched once
 * and every probe after the first boots from a warm cache, instead of every
 * probe paying for the download again.
 *
 * No probe re-answers the audit's question: the audit clicks every control
 * in the state the app starts in, and each probe asks something the audit
 * cannot ask. `scripts/README.md` says which question belongs to which
 * script.
 *
 * ## Environment
 *
 *   DOKIPON_BASE              the deployment to test (default: the live one)
 *   DOKIPON_RUN_ALL_ONLY      comma-separated step ids to run
 *   DOKIPON_RUN_ALL_SKIP      comma-separated step ids to skip
 *   DOKIPON_RUN_ALL_API_ONLY  1 = contract half only, no browser at all
 *   DOKIPON_RUN_ALL_STEP_TIMEOUT  seconds after which one step is killed
 *   DOKIPON_AUDIT_API_ONLY    1 = the audit checks the contract only
 *   DOKIPON_PROBE_PATH        the path `probe-page-load.mjs` opens
 */

import { spawn, spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { VIEWPORT, baseUrl, createReport, exitCodeFor, launchChrome } from './probe-lib.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));

const BASE = baseUrl();
const STARTED = new Date().toISOString();

const ONLY = (process.env.DOKIPON_RUN_ALL_ONLY || '')
  .split(',')
  .map((s) => s.trim())
  .filter(Boolean);
const SKIP = (process.env.DOKIPON_RUN_ALL_SKIP || '')
  .split(',')
  .map((s) => s.trim())
  .filter(Boolean);
const API_ONLY = process.env.DOKIPON_RUN_ALL_API_ONLY === '1';
const STEP_TIMEOUT_MS = Number(process.env.DOKIPON_RUN_ALL_STEP_TIMEOUT || 0) * 1000;

/**
 * The steps, in the order they run.
 *
 * `surface` says which page a browser step drives, so the reader can see
 * what the suite costs on a slow link.
 */
const STEPS = [
  {
    id: 'topology',
    kind: 'static',
    label: 'every contract path is served by both deployment topologies',
    script: 'check-topology-coverage.py',
    surface: 'none (reads the repository)',
  },
  {
    id: 'secrets',
    kind: 'static',
    label: 'the k3s manifests do not lie about credentials',
    script: 'check-manifest-secrets.py',
    surface: 'none (reads the repository)',
  },
  {
    id: 'audit',
    kind: 'audit',
    label: 'the contract, and every control on both surfaces',
    script: 'audit.mjs',
    surface: '/ and /admin/',
  },
  {
    id: 'verify',
    kind: 'verify',
    label: 'the deployment facts the contract cannot see',
    script: 'verify-deployment.mjs',
    surface: 'none (the API only)',
  },
  {
    id: 'cart-click',
    kind: 'probe',
    label: 'add to cart, given the time to render',
    script: 'probe-cart-click.mjs',
    surface: '/ (storefront)',
  },
  {
    id: 'admin-boot',
    kind: 'probe',
    label: 'does the admin console boot, or is it the link',
    script: 'probe-admin-boot.mjs',
    surface: '/admin/',
  },
  {
    id: 'admin-tabs',
    kind: 'probe',
    label: 'do the admin tabs switch, and switch back',
    script: 'probe-admin-tabs.mjs',
    surface: '/admin/',
  },
  {
    id: 'state-dependent',
    kind: 'probe',
    label: 'is a control that does nothing state-dependent, or dead',
    script: 'probe-state-dependent.mjs',
    surface: '/ (storefront)',
  },
  {
    id: 'page-load',
    kind: 'probe',
    label: 'does a page load with no failed request and no error response',
    script: 'probe-page-load.mjs',
    surface: '(the path under test)',
  },
];

// ---------------------------------------------------------------------------
// Running a child process
// ---------------------------------------------------------------------------

async function runChild(command, args, options = {}) {
  const started = Date.now();
  const child = spawn(command, args, {
    cwd: HERE,
    env: { ...process.env, ...(options.env || {}) },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let out = '';
  child.stdout.on('data', (chunk) => {
    out += chunk;
    process.stdout.write(chunk);
  });
  child.stderr.on('data', (chunk) => {
    out += chunk;
    process.stderr.write(chunk);
  });
  const timer = STEP_TIMEOUT_MS > 0 ? setTimeout(() => child.kill(), STEP_TIMEOUT_MS) : null;
  const code = await new Promise((done) => {
    child.on('close', (c) => done(c === null ? -1 : c));
  });
  if (timer) clearTimeout(timer);
  return { code, out, seconds: (Date.now() - started) / 1000 };
}

/** The Python interpreter, or null when there is none. */
function findPython() {
  const candidates = process.platform === 'win32' ? ['python', 'python3'] : ['python3', 'python'];
  for (const candidate of candidates) {
    const probe = spawnSync(candidate, ['--version'], { stdio: 'ignore' });
    if (!probe.error && probe.status === 0) return candidate;
  }
  return null;
}

// ---------------------------------------------------------------------------
// Reading a step's result
// ---------------------------------------------------------------------------

/** The last line of the audit's output that names its report. */
function auditReportPath(out) {
  const matches = [...out.matchAll(/report:\s*(.+audit-[^\s]+\.md)/g)];
  if (matches.length === 0) return null;
  return matches[matches.length - 1][1].trim();
}

/**
 * What the audit found.
 *
 * The audit's own verdict is authoritative, and this only separates the two
 * reasons it can be unhappy: the deployment answered something wrong
 * (a finding), or the page never arrived (nobody found out).
 */
function classifyAudit(out) {
  const reportPath = auditReportPath(out);
  if (!reportPath) {
    return {
      result: 'UNMEASURED',
      reason: ['the audit did not finish and wrote no report — the run itself failed'],
    };
  }
  const jsonPath = reportPath.replace(/\.md$/, '.json');
  if (!existsSync(jsonPath)) {
    return { result: 'UNMEASURED', reason: [`the audit wrote no JSON report at ${jsonPath}`] };
  }
  let report;
  try {
    report = JSON.parse(readFileSync(jsonPath, 'utf8'));
  } catch (e) {
    return { result: 'UNMEASURED', reason: [`the audit report could not be read: ${String(e.message).split('\n')[0]}`] };
  }
  const verdict = report.verdict || {};
  const reason = [];
  if (verdict.apiFindings) reason.push(`${verdict.apiFindings} API finding(s)`);
  if (verdict.brokenControls) reason.push(`${verdict.brokenControls} broken control(s)`);
  if (reason.length) return { result: 'FAIL', reason };

  const notAnswered = [];
  if (verdict.unmeasuredControls) notAnswered.push(`${verdict.unmeasuredControls} control(s) could not be measured`);
  for (const surface of report.surfaces || []) {
    for (const problem of surface.problems || []) notAnswered.push(`${surface.surface}: ${problem}`);
  }
  if (notAnswered.length) return { result: 'UNMEASURED', reason: notAnswered };
  return { result: 'PASS', reason: [] };
}

/** The `  - ...` lines a script prints under its summary. */
function bulletLines(out) {
  return out
    .split('\n')
    .filter((line) => /^\s+- /.test(line))
    .map((line) => line.trim());
}

/** A python checker: 0 = ok, 1 = problems, 2 = it could not run. */
function classifyChecker(code, out) {
  if (code === 0) return { result: 'PASS', reason: [] };
  if (code === 1) {
    const problems = bulletLines(out);
    return { result: 'FAIL', reason: problems.length ? problems : ['the checker reported problems'] };
  }
  if (code === -1) return { result: 'UNMEASURED', reason: ['the checker was killed before it finished'] };
  const problems = bulletLines(out);
  return {
    result: 'SKIPPED',
    reason: problems.length ? problems : [lastMeaningfulLine(out) || `the checker exited ${code}`],
  };
}

/** `verify-deployment.mjs`: 0 = ok, 1 = findings, 2 = nothing to measure. */
function classifyVerify(code, out) {
  if (code === 0) return { result: 'PASS', reason: [] };
  if (code === 1) {
    const findings = bulletLines(out);
    return { result: 'FAIL', reason: findings.length ? findings : ['the verification reported findings'] };
  }
  if (code === -1) return { result: 'UNMEASURED', reason: ['the verification was killed before it finished'] };
  const unmeasured = bulletLines(out);
  return {
    result: 'UNMEASURED',
    reason: unmeasured.length ? unmeasured : [lastMeaningfulLine(out) || `the verification exited ${code}`],
  };
}

function lastMeaningfulLine(out) {
  const lines = out
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  return lines.length ? lines[lines.length - 1] : '';
}

// ---------------------------------------------------------------------------
// The steps
// ---------------------------------------------------------------------------

const results = [];

function record(step, result, reason, seconds) {
  results.push({ id: step.id, kind: step.kind, label: step.label, result, reason, seconds });
}

function wanted(step) {
  if (ONLY.length && !ONLY.includes(step.id)) return false;
  if (SKIP.includes(step.id)) return false;
  if (API_ONLY && step.kind === 'probe') return false;
  return true;
}

function announce(step) {
  console.log('');
  console.log('-'.repeat(78));
  console.log(`>> ${step.id} — ${step.label}   [${step.kind}${step.surface ? `, ${step.surface}` : ''}]`);
  console.log('-'.repeat(78));
}

async function runPythonStep(step, python) {
  announce(step);
  if (!python) {
    record(step, 'SKIPPED', ['no Python interpreter on this machine'], 0);
    console.log('SKIPPED     no Python interpreter on this machine');
    return;
  }
  const run = await runChild(python, [resolve(HERE, step.script)]);
  const verdict = classifyChecker(run.code, run.out);
  record(step, verdict.result, verdict.reason, run.seconds);
}

async function runNodeStep(step, env) {
  announce(step);
  const run = await runChild(process.execPath, [resolve(HERE, step.script)], { env });
  if (step.id === 'audit') {
    const verdict = classifyAudit(run.out);
    record(step, verdict.result, verdict.reason, run.seconds);
    return;
  }
  const verdict = classifyVerify(run.code, run.out);
  record(step, verdict.result, verdict.reason, run.seconds);
}

/**
 * The browser probes, on one Chrome and one browser context.
 *
 * Each probe gets a page of its own: the cache stays warm across the
 * probes, and no probe can see what another probe did to the page.
 */
async function runProbeSteps(steps) {
  if (steps.length === 0) return;
  let browser = null;
  let context = null;
  try {
    browser = await launchChrome();
    context = await browser.newContext({ viewport: VIEWPORT });
    for (const step of steps) {
      announce(step);
      const module = await import(pathToFileURL(resolve(HERE, step.script)).href);
      const report = createReport(step.id);
      const page = await context.newPage();
      try {
        await module.run(page, report, {});
      } catch (e) {
        report.unmeasured('the probe ran to the end', String(e && e.message ? e.message : e).split('\n')[0]);
      } finally {
        await page.close().catch(() => {});
      }
      const summary = report.summary();
      const code = exitCodeFor(summary);
      if (code === 1) record(step, 'FAIL', summary.failedItems, 0);
      else if (code === 2) record(step, 'UNMEASURED', summary.unmeasuredItems, 0);
      else record(step, 'PASS', [], 0);
      console.log('');
      console.log(`${step.id}: ${summary.passed} passed, ${summary.failed} failed, ${summary.unmeasured} unmeasured`);
    }
  } catch (e) {
    for (const step of steps) {
      record(step, 'UNMEASURED', [`the browser could not be started: ${String(e && e.message ? e.message : e).split('\n')[0]}`], 0);
    }
  } finally {
    if (context) await context.close().catch(() => {});
    if (browser) await browser.close().catch(() => {});
  }
}

// ---------------------------------------------------------------------------
// The run
// ---------------------------------------------------------------------------

async function main() {
  console.log('='.repeat(78));
  console.log('DOKIPON commerce e2e — the suite');
  console.log(`target    ${BASE}`);
  console.log(`started   ${STARTED}`);
  console.log('='.repeat(78));

  const python = findPython();
  const steps = STEPS.filter(wanted).map((step) =>
    API_ONLY && step.id === 'audit' ? { ...step, surface: 'none (the contract half only)' } : step,
  );
  const skippedByRequest = STEPS.filter((s) => !wanted(s)).map((s) => s.id);
  if (skippedByRequest.length) {
    console.log(`not run (by request): ${skippedByRequest.join(', ')}`);
  }
  if (API_ONLY) {
    console.log('contract half only: the browser steps are not run');
  }

  for (const step of steps.filter((s) => s.kind === 'static')) {
    await runPythonStep(step, python);
  }
  for (const step of steps.filter((s) => s.kind === 'audit' || s.kind === 'verify')) {
    await runNodeStep(step, API_ONLY && step.kind === 'audit' ? { DOKIPON_AUDIT_API_ONLY: '1' } : {});
  }
  await runProbeSteps(steps.filter((s) => s.kind === 'probe'));

  // --- the one summary ----------------------------------------------------
  const counts = { PASS: 0, FAIL: 0, UNMEASURED: 0, SKIPPED: 0 };
  for (const r of results) counts[r.result] += 1;

  console.log('');
  console.log('='.repeat(78));
  console.log('DOKIPON commerce e2e — suite summary');
  console.log(`target    ${BASE}`);
  console.log(`started   ${STARTED}`);
  console.log('='.repeat(78));
  for (const r of results) {
    const seconds = `${r.seconds.toFixed(1)}s`;
    console.log(`  ${r.result.padEnd(11)}${seconds.padStart(8)}  ${r.id.padEnd(16)} ${r.label}`);
  }
  console.log('-'.repeat(78));
  console.log(
    `steps ${results.length} · passed ${counts.PASS} · failed ${counts.FAIL} · ` +
      `unmeasured ${counts.UNMEASURED} · skipped ${counts.SKIPPED}`,
  );
  const unanswered = counts.UNMEASURED + counts.SKIPPED;
  console.log(`product findings ${counts.FAIL} · questions not answered ${unanswered}`);
  for (const r of results.filter((x) => x.result !== 'PASS')) {
    console.log('');
    console.log(`${r.result} ${r.id}`);
    for (const line of r.reason) console.log(`    ${line}`);
  }
  console.log('');
  console.log('='.repeat(78));

  if (counts.FAIL > 0) {
    console.log(`the suite found ${counts.FAIL} step(s) with a defect in the deployment`);
    process.exit(1);
  }
  if (unanswered > 0) {
    console.log(`the suite answered everything it could; ${unanswered} step(s) could not be answered here`);
    process.exit(2);
  }
  console.log(`the suite passed: ${counts.PASS} step(s)`);
  process.exit(0);
}

main().catch((e) => {
  console.error(e);
  process.exit(2);
});
