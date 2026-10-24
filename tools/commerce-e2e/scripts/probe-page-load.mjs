#!/usr/bin/env node
/**
 * probe-page-load.mjs — does a page load with nothing missing?
 *
 * ## The question
 *
 * The audit records what happened while a surface loaded, but it does not
 * judge it: a 404 on a static asset or a request that never arrived shows
 * up in the report as a line, not as a finding. This probe judges it.
 *
 * So the question is: **does <path> load with no failed request and no
 * error response of the deployment's own?**
 *
 * ## What counts and what does not
 *
 *   - a 4xx or 5xx on a static resource of the deployment → the
 *     deployment's own defect: FAIL;
 *   - a 4xx on an API route → not a defect: the admin routes answer 401
 *     without a session on purpose, and the contract says so. A NOTE;
 *   - a request that never arrived → the link: UNMEASURED;
 *   - a request to the font CDN, which is unreachable from here → a NOTE.
 *
 * ## What it needs
 *
 * A deployment to look at (`DOKIPON_BASE`) and a browser. No credential.
 * The path defaults to `/`; give it another one with `DOKIPON_PROBE_PATH`
 * or as the first argument.
 */

import {
  baseUrl,
  bootFlutter,
  collectors,
  enableSemantics,
  isApiCall,
  isFontCdn,
  isMain,
  isOwnHost,
  openSurface,
  standalone,
} from './probe-lib.mjs';

export const id = 'probe-page-load';
export const title = 'probe-page-load';
export const surface = '(the path under test)';
export const question = 'does a page load with no failed request and no error response?';

export async function run(page, report, options = {}) {
  const path = options.path || process.env.DOKIPON_PROBE_PATH || '/';
  const state = collectors(page);

  const opened = await openSurface(page, path);
  if (opened.error) {
    report.unmeasured('the page is reachable', `${opened.error} (${opened.url})`);
    return;
  }
  if (opened.status >= 400) {
    report.fail('the page is served', `the edge answered ${opened.status} for ${path}`);
    return;
  }
  report.pass('the page is served', `status ${opened.status} for ${path}`);

  const boot = await bootFlutter(page);
  if (!boot.mounted) {
    const dropped = state.failedRequests.filter((r) => !isFontCdn(r.url));
    if (dropped.length) {
      report.unmeasured('the app mounts', `${dropped[0].reason} — the link did not deliver ${dropped[0].url}`);
    } else {
      report.fail('the app mounts', boot.reason);
    }
    return;
  }
  report.pass('the app mounts', 'flutter-view is attached');

  const fontFailures = state.failedRequests.filter((r) => isFontCdn(r.url));
  if (fontFailures.length) {
    report.note(
      'third-party assets',
      `${fontFailures.length} font request(s) did not arrive — the environment, not the deployment`,
    );
  }

  const ownFailures = state.failedRequests.filter((r) => isOwnHost(r.url));
  if (ownFailures.length) {
    report.unmeasured('the deployment’s own requests all arrive', `${ownFailures[0].reason} ${ownFailures[0].url}`);
  } else {
    report.pass('the deployment’s own requests all arrive', 'no request of the deployment’s own failed');
  }

  const ownErrors = state.badResponses.filter((b) => isOwnHost(b.url) && !isApiCall(b.url));
  if (ownErrors.length) {
    report.fail('the deployment answers no error status of its own', `${ownErrors[0].status} ${ownErrors[0].url}`);
  } else {
    report.pass('the deployment answers no error status of its own', 'no 4xx or 5xx outside the API routes');
  }

  const apiErrors = state.badResponses.filter((b) => isApiCall(b.url));
  if (apiErrors.length) {
    report.note('API routes answered with an error status', apiErrors.map((b) => `${b.status} ${b.url}`).join(', '));
  }

  if (await enableSemantics(page)) {
    report.pass('the page exposes inspectable controls', 'the semantics tree is on');
  } else {
    report.fail('the page exposes inspectable controls', 'the semantics tree could not be turned on');
  }
}

if (isMain(import.meta.url)) {
  const path = process.argv[2] || process.env.DOKIPON_PROBE_PATH || '/';
  console.log(`probing ${baseUrl()}${path}\n`);
  process.exit(await standalone({ title, run, options: { path } }));
}
