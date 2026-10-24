#!/usr/bin/env node
/**
 * probe-admin-tabs.mjs — do the two admin tabs switch, and switch back?
 *
 * ## The question
 *
 * The audit reported "Orders Tab 1 of 2" as inert: the click changed
 * nothing. Two readings, and they need different fixes:
 *
 *   (a) the Orders tab is *already* the selected one, so clicking it is a
 *       no-op by design — an artefact of the instrument, because the DOM
 *       does not say which tab is selected;
 *   (b) the tab really does not switch back — a dead control.
 *
 * The audit answers this only approximately: it re-clicks a control that
 * did nothing after some *other* control changed the page, and it picks
 * the primer itself. This probe asks the round trip directly: click
 * Catalog, then click Orders, and see whether the screen comes back.
 *
 * So the question is: **do the admin tabs switch both ways?**
 *
 * ## What it needs
 *
 * A deployment to look at (`DOKIPON_BASE`), a browser, and the admin
 * credential (`ADMIN_USER` / `ADMIN_PASSWORD`) to get past the login. If
 * the credential is refused, the tabs cannot be reached at all — that is
 * reported as UNMEASURED, because the credential question belongs to
 * `verify-deployment.mjs` and this probe has nothing to say about it.
 */

import {
  baseUrl,
  bootFlutter,
  collectors,
  controlLocator,
  enableSemantics,
  findControl,
  isMain,
  openSurface,
  signIn,
  signature,
  standalone,
} from './probe-lib.mjs';

export const id = 'probe-admin-tabs';
export const title = 'probe-admin-tabs';
export const surface = '/admin/';
export const question = 'do the admin tabs switch, and switch back?';

const ORDERS = /orders|注文/i;
const CATALOG = /catalog|カタログ/i;

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
    const dropped = state.failedRequests.filter((r) => !/fonts\.(googleapis|gstatic)\.com/.test(r.url));
    if (dropped.length) {
      report.unmeasured('the admin app mounts', `${dropped[0].reason} — the link did not deliver ${dropped[0].url}`);
    } else {
      report.fail('the admin app mounts', boot.reason);
    }
    return;
  }
  report.pass('the admin app mounts', 'flutter-view is attached');

  if (!(await enableSemantics(page))) {
    report.fail('the semantics tree can be enabled', 'the app exposes no inspectable control');
    return;
  }
  report.pass('the semantics tree can be enabled', 'the controls are inspectable');

  const login = await signIn(page);
  if (!login.ok) {
    report.unmeasured(
      'the tabs can be reached',
      `the admin console refused the credential (${login.reason}) — the credential question belongs to verify-deployment.mjs`,
    );
    return;
  }
  report.pass('the admin console accepts the credential', 'the credential fields are gone');

  const orders = await findControl(page, ORDERS, ['tab', 'button']);
  const catalog = await findControl(page, CATALOG, ['tab', 'button']);
  if (!orders) {
    report.fail('the Orders tab is present', 'no control labelled Orders');
    return;
  }
  if (!catalog) {
    report.fail('the Catalog tab is present', 'no control labelled Catalog');
    return;
  }
  report.pass('both tabs are present', `“${orders.label}” and “${catalog.label}”`);

  const initial = await signature(page);
  report.note('the screen starts on', (await page.locator('flt-semantics').first().getAttribute('aria-label')) || '(no label)');

  const click = async (control) => {
    let error = null;
    try {
      await controlLocator(page, control.role, control.index).click({ force: true, timeout: 30_000 });
    } catch (e) {
      error = String(e && e.message ? e.message : e).split('\n')[0];
    }
    await page.waitForTimeout(3000);
    return error;
  };

  const afterCatalogClick = await click(catalog);
  const afterCatalog = await signature(page);
  if (afterCatalogClick) {
    report.fail('clicking Catalog switches the screen', afterCatalogClick);
  } else if (afterCatalog !== initial) {
    report.pass('clicking Catalog switches the screen', 'the screen changed');
  } else {
    report.fail('clicking Catalog switches the screen', 'the click changed nothing');
  }

  const afterOrdersClick = await click(orders);
  const afterOrders = await signature(page);
  if (afterOrdersClick) {
    report.fail('clicking Orders brings the screen back', afterOrdersClick);
  } else if (afterOrders !== afterCatalog) {
    const back = afterOrders === initial;
    report.pass('clicking Orders brings the screen back', back ? 'the screen is back where it started' : 'the screen changed');
  } else {
    report.fail('clicking Orders brings the screen back', 'the click changed nothing — the tab does not switch back');
  }
}

if (isMain(import.meta.url)) {
  console.log(`probing ${baseUrl()}\n`);
  process.exit(await standalone({ title, run }));
}
