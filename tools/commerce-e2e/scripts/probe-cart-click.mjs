#!/usr/bin/env node
/**
 * probe-cart-click.mjs — does "add to cart" actually work?
 *
 * ## The question
 *
 * The audit reported eight "click failed: Timeout 5000ms exceeded" on the
 * product cards. There are two readings, and they need different fixes:
 *
 *   (a) the button is broken — the control is painted and does nothing;
 *   (b) the button was not there yet — the catalogue had not finished
 *       rendering when the audit clicked it.
 *
 * The audit cannot tell them apart: its per-control loop reloads the page
 * for every control and clicks within five seconds of the page loading, so
 * on a slow link the control may simply not exist yet. This probe waits for
 * the control to appear — up to five minutes — and only then asks whether
 * pressing it does anything.
 *
 * So the question is: **given enough time to render, does the product
 * card's add-to-cart control work?**
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
  waitForControl,
} from './probe-lib.mjs';

export const id = 'probe-cart-click';
export const title = 'probe-cart-click';
export const surface = '/ (storefront)';
export const question = 'given enough time to render, does add-to-cart work?';

/** The control under test. */
const ADD_TO_CART = /カートに追加|add to cart/i;
/** The cart button — which must not be the add-to-cart button itself. */
const CART_BUTTON = /カート|cart/i;

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

  // The whole point of this probe: wait for the control instead of racing it.
  const products = await catalogueSize(page);
  if (products === 0) {
    report.unmeasured('the add-to-cart control appears', 'the catalogue is empty — there is nothing to add to cart');
    return;
  }
  report.note('the catalogue holds', `${products ?? 'an unknown number of'} product(s)`);

  const appeared = await waitForControl(page, ADD_TO_CART, ['button', 'tab', 'link'], 300_000);
  if (!appeared.found) {
    report.fail('the add-to-cart control appears', 'it never rendered, although the app is running');
    return;
  }
  report.pass('the add-to-cart control appears', `“${appeared.found.label}”`);

  // The cart button — which must not be the add-to-cart button itself.
  const cartLabel = async () => {
    const found = await findControl(page, (label) => CART_BUTTON.test(label) && !ADD_TO_CART.test(label));
    return found ? found.label : null;
  };

  const before = await cartLabel();
  const signatureBefore = await signature(page);
  const pageErrorsBefore = state.pageErrors.length;
  const consoleErrorsBefore = state.consoleErrors.length;
  const calls = [];
  const onRequest = (r) => {
    if (/\/v1\/|\/admin\/v1\//.test(r.url())) calls.push(`${r.method()} ${r.url()}`);
  };
  page.on('request', onRequest);

  // The control is looked up again here: the click that follows may change
  // the semantics tree, and a stale index would land on another control.
  const clicked = await clickControl(page, ADD_TO_CART, ['button', 'tab', 'link'], { timeout: 60_000, settleMs: 3000 });
  page.off('request', onRequest);

  const after = await cartLabel();
  const signatureAfter = await signature(page);

  if (clicked.error) {
    report.fail('pressing add-to-cart dispatches', clicked.error);
  } else {
    report.pass('pressing add-to-cart dispatches', 'the click reached the control');
  }

  const reacted = after !== before || signatureAfter !== signatureBefore || calls.length > 0;
  if (reacted) {
    const what = calls.length ? calls[calls.length - 1] : 'the cart label and the screen changed';
    report.pass('pressing add-to-cart changes the cart', what);
  } else {
    report.fail('pressing add-to-cart changes the cart', 'nothing changed: no request, no new label, no change on screen');
  }

  const newErrors = [
    ...state.pageErrors.slice(pageErrorsBefore),
    ...state.consoleErrors.slice(consoleErrorsBefore).filter((t) => !/Failed to load resource/i.test(t)),
  ];
  if (newErrors.length) {
    report.fail('pressing add-to-cart raises no error', newErrors[0].split('\n')[0]);
  } else {
    report.pass('pressing add-to-cart raises no error', 'no uncaught exception, no console error');
  }
}

if (isMain(import.meta.url)) {
  console.log(`probing ${baseUrl()}\n`);
  process.exit(await standalone({ title, run }));
}
