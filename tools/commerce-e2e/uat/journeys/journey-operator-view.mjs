/**
 * journey-operator-view.mjs — the operator's side of the same story.
 *
 * An order that a shopper placed has to be visible to the person who has
 * to act on it, with the line's price readable and the trail readable
 * too. This journey signs into the operator console, finds the order, and
 * reads it.
 *
 * The console's order detail screen used to crash on the line's price and
 * used to read the trail as "unknown → unknown" — two quiet defects that a
 * screenshot cannot show and this journey can.
 */

import {
  bootFlutter,
  clickControl,
  clickMatching,
  enableSemantics,
  findLabel,
  journey,
  openSurface,
  semanticsLabels,
  waitForLabel,
} from '../lib.mjs';

export const operatorView = journey({
  id: 'operator-view',
  title: 'The operator sees the order, its price and its trail',
  needs: { admin: true },
  async run({ t, page, fixture, admin, api, state }) {
    await openSurface(page, '/admin/');
    await bootFlutter(page);
    await enableSemantics(page);

    // 1. Sign in. Flutter's web text editing mirrors genuine key input into
    //    its controller, so the credential is typed, not filled.
    const inputs = page.locator('input, textarea');
    if ((await inputs.count()) < 2) {
      t.check('the operator console asks for a credential', false, 'no credential fields were found');
      return;
    }
    await inputs.nth(0).click({ timeout: 15_000 });
    await page.waitForTimeout(300);
    await page.keyboard.type(admin.user, { delay: 40 });
    await inputs.nth(1).click({ timeout: 15_000 });
    await page.waitForTimeout(300);
    await page.keyboard.type(admin.password, { delay: 40 });
    await clickMatching(page, 'button', /sign in|ログイン/i);
    await page.waitForTimeout(4000);

    const loggedIn = (await inputs.count()) === 0;
    t.check('the operator console accepted the credential', loggedIn, loggedIn ? 'the login screen is gone' : 'the console still shows the credential fields');
    if (!loggedIn) return;

    // 2. The orders list. The order the LINE journey placed is one of them.
    const rows = await semanticsLabels(page);
    const orderRows = rows.filter((node) => /ord-\d+/.test(node.label));
    t.check(
      'the operator sees the orders that exist',
      orderRows.length > 0,
      `${orderRows.length} label(s) carry an order id on the list`,
    );

    // 3. The order detail: the line's price, and the trail. The list opens
    //    an order with a button on the row, so that is what is pressed.
    const orderId = (rows.map((node) => (node.label.match(/ord-\d+/) || [])[0]).find(Boolean)) || null;
    if (!orderId) {
      t.check('the operator can open an order', false, 'no order id was on the list');
      return;
    }
    await clickControl(page, 'button', 'Open');

    // The detail is loaded, not painted: the price is waited for, because
    // a screen that is still loading carries no price and would be
    // reported as a screen without one.
    const price = await waitForLabel(page, /\d+ JPY/, { timeout: 20_000 });
    t.check(
      'the order detail shows the line\'s price in the currency\'s own major unit',
      price !== null,
      price ? price.label : 'the detail carries no price',
    );

    const detail = await semanticsLabels(page);
    const bad = state.badResponses.filter((r) => /\/admin\/v1\//.test(r));
    t.check(
      'the order detail shows the line\'s price in the currency\'s own major unit',
      price !== null,
      price
        ? price.label
        : [
            bad.length ? `the console got ${bad.join(' | ')}` : 'the console made no failing request',
            `on the screen: ${detail.map((node) => node.label.slice(0, 60)).join(' | ').slice(0, 400)}`,
          ].join(' — '),
    );
    const unknown = detail.filter((node) => /unknown/.test(node.label));
    t.check(
      'the order trail reads as transitions, not as "unknown → unknown"',
      unknown.length === 0,
      unknown.length ? `the trail still says: ${unknown[0].label.slice(0, 80)}` : 'no label says "unknown"',
    );
    t.check(
      'the order detail screen was reached, not the list',
      detail.some((node) => /Payment state|Unit price|Back to orders/.test(node.label)),
      detail.length
        ? `${detail.length} labels on the screen: ${detail.map((node) => node.label.slice(0, 30)).join(' | ').slice(0, 300)}`
        : 'the screen is empty',
    );

    // 4. And the same order over the API, which is what the operator's
    //    screen is a view of.
    const login = await api.fetch('/admin/v1/sessions', {
      method: 'POST',
      body: { username: admin.user, password: admin.password },
    });
    const token = login.json?.token || '';
    t.check(
      'the operator can sign in over the API',
      login.status === 200 && Boolean(token),
      `status ${login.status}`,
    );
    if (!token) return;
    const apiDetail = await api.fetch(`/admin/v1/orders/${orderId}`, {
      headers: { authorization: `Bearer ${token}` },
    });
    t.check(
      'the operator can read the same order over the API',
      apiDetail.status === 200,
      apiDetail.status === 200 ? 'the order detail answered 200' : `the order detail answered ${apiDetail.status}`,
    );
  },
});
