#!/usr/bin/env node
/**
 * probe-state-dependent.mjs — is a control that does nothing state-dependent
 * or dead?
 *
 * ## The question
 *
 * Some controls are *supposed* to do nothing in the state the app starts in.
 * The "all products" control is one: the catalogue already shows everything,
 * so clearing the filter changes nothing — that is correct behaviour, not a
 * dead button. It was reported as a dead control once (SHOP COLLECTION),
 * and it was a real defect then.
 *
 * The distinction only shows up in the **round trip**: select a category
 * (the screen changes), then click "all products" (the screen must change
 * back). A control that only ever does nothing is dead; a control that
 * does nothing at first and works after a state change is correct.
 *
 * The audit re-clicks a control that did nothing after some *other*
 * control changed the page, but it chooses the primer itself, so it cannot
 * be relied on to pick the one that matters. This probe picks it.
 *
 * So the question is: **once a category is selected, does the all-products
 * control bring the catalogue back?**
 *
 * ## What it needs
 *
 * A deployment to look at (`DOKIPON_BASE`) and a browser. No credential.
 */

import {
  baseUrl,
  bootFlutter,
  catalogueSize,
  clickControl,
  collectors,
  enableSemantics,
  findControl,
  isMain,
  openSurface,
  signature,
  standalone,
} from './probe-lib.mjs';

export const id = 'probe-state-dependent';
export const title = 'probe-state-dependent';
export const surface = '/ (storefront)';
export const question = 'is a control that does nothing state-dependent, or dead?';

/** A category control on the storefront. */
const CATEGORY = /ランジェリー|lingerie/i;
/** The control that clears the category — "all products". */
const ALL_PRODUCTS = /shop collection|すべて|all products|すべて表示/i;

const productLines = (signatureText) => signatureText.split('\n').filter((line) => /DOKIPON/.test(line)).length;

export async function run(page, report) {
  const state = collectors(page);

  const opened = await openSurface(page, '/');
  if (opened.error) {
    report.unmeasured('the storefront is reachable', `${opened.error} (${opened.url})`);
    return;
  }
  if (opened.status >= 400) {
    report.fail('the storefront is served', `the edge answered ${opened.status}`);
    return;
  }
  report.pass('the storefront is served', `status ${opened.status}`);

  const boot = await bootFlutter(page);
  if (!boot.mounted) {
    const dropped = state.failedRequests.filter((r) => !/fonts\.(googleapis|gstatic)\.com/.test(r.url));
    if (dropped.length) {
      report.unmeasured('the storefront app mounts', `${dropped[0].reason} — the link did not deliver ${dropped[0].url}`);
    } else {
      report.fail('the storefront app mounts', boot.reason);
    }
    return;
  }
  report.pass('the storefront app mounts', 'flutter-view is attached');

  if (!(await enableSemantics(page))) {
    report.fail('the semantics tree can be enabled', 'the app exposes no inspectable control');
    return;
  }
  report.pass('the semantics tree can be enabled', 'the controls are inspectable');

  const products = await catalogueSize(page);
  if (products === 0) {
    report.unmeasured('a category control is present', 'the catalogue is empty — there is no category to filter');
    return;
  }
  report.note('the catalogue holds', `${products ?? 'an unknown number of'} product(s)`);

  const category = await findControl(page, CATEGORY, ['button', 'tab', 'link']);
  if (!category) {
    report.fail('a category control is present', 'no control labelled with a category');
    return;
  }
  report.pass('a category control is present', `“${category.label}”`);

  const beforeCategory = await signature(page);
  const clickedCategory = await clickControl(page, CATEGORY, ['button', 'tab', 'link'], { settleMs: 3000 });
  const afterCategory = await signature(page);

  if (clickedCategory.error) {
    report.fail('clicking the category changes the catalogue', clickedCategory.error);
    return;
  }
  if (afterCategory !== beforeCategory) {
    report.pass(
      'clicking the category changes the catalogue',
      `${productLines(beforeCategory)} → ${productLines(afterCategory)} product lines`,
    );
  } else {
    report.fail('clicking the category changes the catalogue', 'the click changed nothing');
    return;
  }

  const all = await findControl(page, ALL_PRODUCTS, ['button', 'tab', 'link']);
  if (!all) {
    report.fail('the all-products control is present', 'no control labelled with the whole catalogue');
    return;
  }
  report.pass('the all-products control is present', `“${all.label}”`);

  const beforeAll = await signature(page);
  const clickedAll = await clickControl(page, ALL_PRODUCTS, ['button', 'tab', 'link'], { settleMs: 3000 });
  const afterAll = await signature(page);

  if (clickedAll.error) {
    report.fail('the all-products control works once a category is selected', clickedAll.error);
  } else if (afterAll !== beforeAll) {
    report.pass(
      'the all-products control works once a category is selected',
      `${productLines(beforeAll)} → ${productLines(afterAll)} product lines — it is state-dependent, not dead`,
    );
  } else {
    report.fail(
      'the all-products control works once a category is selected',
      'the click changed nothing, in a state where it had to change something — the control is dead',
    );
  }
}

if (isMain(import.meta.url)) {
  console.log(`probing ${baseUrl()}\n`);
  process.exit(await standalone({ title, run }));
}
