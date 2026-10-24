#!/usr/bin/env node
/**
 * probe-admin-boot.mjs — does the admin console actually boot?
 *
 * ## The question
 *
 * The admin surface stopped loading in the browser while its assets were
 * served fine over HTTP and the storefront booted normally. That pattern
 * smells like the app throwing during startup, not like a slow link — and
 * the two need completely different fixes.
 *
 * The audit can only say "the surface did not load". It cannot say *why*.
 * This probe answers the narrower question: **does the admin app boot, or
 * is it the link that is broken?**
 *
 * The distinction is made from the evidence the page leaves behind:
 *
 *   - the document answers and Flutter mounts → the app booted;
 *   - the document answers but Flutter never mounts, and nothing on the
 *     network failed → the app did not start: a finding;
 *   - something on the network did not arrive → the link: UNMEASURED.
 *
 * ## What it needs
 *
 * A deployment to look at (`DOKIPON_BASE`) and a browser. No credential.
 */

import {
  baseUrl,
  bootFlutter,
  collectors,
  enableSemantics,
  isFontCdn,
  isMain,
  isOwnHost,
  openSurface,
  standalone,
} from './probe-lib.mjs';

export const id = 'probe-admin-boot';
export const title = 'probe-admin-boot';
export const surface = '/admin/';
export const question = 'does the admin console boot, or is it the link that is broken?';

export async function run(page, report) {
  const state = collectors(page);

  const opened = await openSurface(page, '/admin/');
  if (opened.error) {
    report.unmeasured('the admin console is reachable', `${opened.error} (${opened.url})`);
    return;
  }
  if (opened.status >= 400) {
    report.fail('the admin console is served', `the edge answered ${opened.status}`);
    return;
  }
  report.pass('the admin console is served', `status ${opened.status}`);

  const boot = await bootFlutter(page);
  if (!boot.mounted) {
    const dropped = state.failedRequests.filter((r) => !isFontCdn(r.url));
    if (dropped.length) {
      report.unmeasured('the admin app mounts', `${dropped[0].reason} — the link did not deliver ${dropped[0].url}`);
    } else {
      report.fail('the admin app mounts', boot.reason);
    }
    return;
  }
  report.pass('the admin app mounts', 'flutter-view is attached');

  // Give the app a moment to finish starting up, so a late exception is
  // caught rather than missed.
  await page.waitForTimeout(5000);
  const shape = await page.evaluate(() => ({
    semantics: document.querySelectorAll('flt-semantics').length,
    placeholder: !!document.querySelector('flt-semantics-placeholder'),
  }));
  report.note('the page shape', JSON.stringify(shape));

  if (state.pageErrors.length) {
    report.fail('the admin app starts without an uncaught exception', state.pageErrors[0].split('\n')[0]);
  } else {
    report.pass('the admin app starts without an uncaught exception', 'no uncaught exception');
  }

  const fontFailures = state.failedRequests.filter((r) => isFontCdn(r.url));
  if (fontFailures.length) {
    report.note(
      'third-party assets',
      `${fontFailures.length} font request(s) did not arrive — the environment, not the deployment`,
    );
  }

  const ownFailures = state.failedRequests.filter((r) => isOwnHost(r.url));
  if (ownFailures.length) {
    report.fail('the admin app’s own assets all arrive', `${ownFailures[0].reason} ${ownFailures[0].url}`);
  } else {
    report.pass('the admin app’s own assets all arrive', 'no request of the deployment’s own failed');
  }

  const serverErrors = state.badResponses.filter((b) => b.status >= 500);
  if (serverErrors.length) {
    report.fail('no 5xx while the admin console boots', `${serverErrors[0].status} ${serverErrors[0].url}`);
  } else {
    report.pass('no 5xx while the admin console boots', 'the deployment answered no error status');
  }

  const semanticsOn = await enableSemantics(page);
  if (semanticsOn) {
    report.pass('the console exposes inspectable controls', 'the semantics tree is on');
  } else {
    report.fail('the console exposes inspectable controls', 'the semantics tree could not be turned on');
  }
}

if (isMain(import.meta.url)) {
  console.log(`probing ${baseUrl()}\n`);
  process.exit(await standalone({ title, run }));
}
