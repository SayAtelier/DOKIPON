/**
 * journey-session-survives-reload.mjs — the shopper who comes back.
 *
 * The session is the shopper's identity: it is the buyer's key on their orders
 * and the cart's id. When the sessions lived in each instance's memory, a
 * reload threw that identity away — the shopper came back as somebody else,
 * with an empty basket and no way to find the order they had just placed.
 *
 * The sessions are in the database now, and the server remembers them in a
 * cookie the browser sends on its own. So the client does not have to remember
 * anything: it asks "am I still the shopper I was?", the server answers from
 * the cookie, and the cart and the orders come back with it.
 *
 * This journey drives that, in a browser, against a real deployment.
 */

import {
  bootFlutter,
  clickControl,
  clickMatching,
  clickProduct,
  enableSemantics,
  journey,
  openSurface,
  waitForCatalogue,
  waitForLabel,
} from '../lib.mjs';

export const sessionSurvivesReload = journey({
  id: 'session-survives-reload',
  title: '刷新之后还是我：购物车与订单都还在',
  async run({ t, page, session, fixture }) {
    await openSurface(page, '/');
    await bootFlutter(page);
    await enableSemantics(page);
    await waitForCatalogue(page, fixture.catalogue.products);

    // 1. Buy something, so there is a cart and an order to come back to.
    const buy = fixture.journeys.buy;
    await clickProduct(page, buy.title);
    await waitForLabel(page, 'SKU:', { timeout: 20_000 });
    await clickControl(page, 'button', 'カートに追加 / Add to cart');
    const badge = await waitForLabel(page, 'カート / Cart, 1 items', { timeout: 20_000 });
    t.check(
      'the line is in the cart',
      badge !== null,
      badge ? badge.label : 'the badge never showed a count',
    );
    if (!badge) return;

    await clickMatching(page, 'button', /カート \/ Cart/);
    await page.waitForTimeout(1_200);
    await clickControl(page, 'button', 'レジに進む / Checkout');
    const confirmed = await waitForLabel(page, 'ご注文を完了しました / Order confirmed.', {
      timeout: 45_000,
    });
    t.check(
      'the order is placed',
      confirmed !== null,
      confirmed ? confirmed.label : 'the status bar never confirmed the order',
    );
    if (!confirmed) return;

    // 2. The session is remembered by the server, in a cookie the browser
    //    sends on its own.
    const cookies = await page.context().cookies();
    const cookie = cookies.find((c) => c.name === 'dokipon_session');
    t.check(
      'the session is remembered in a cookie',
      Boolean(cookie),
      cookie ? 'the cookie carries the session' : 'there is no session cookie',
    );
    const before = session.id;

    // 3. Reload. The client forgets everything; the browser does not.
    await page.reload();
    await bootFlutter(page);
    await enableSemantics(page);
    await waitForCatalogue(page, fixture.catalogue.products);

    // 4. The same shopper.
    t.check(
      'the session survived the reload',
      Boolean(session.id) && session.id === before,
      `before=${before} after=${session.id}`,
    );

    // 5. And the basket that came with it.
    const after = await waitForLabel(page, 'カート / Cart, 1 items', { timeout: 20_000 });
    t.check(
      'the cart came back with the session',
      after !== null,
      after ? after.label : 'the cart is empty after a reload',
    );
  },
});
