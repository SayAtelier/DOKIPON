/**
 * journey-guest-browse.mjs — the journey a first-time visitor takes.
 *
 * Browse the catalogue, narrow it with the category strip, open a product,
 * put it in the cart, and see what the cart says.
 *
 * The journey deliberately stops at the cart. Whether a **guest** may
 * check out is a product rule the codebase states in three places and does
 * not enforce in a fourth, and the UAT does not assert a rule it cannot
 * show to be true — the `line-purchase` journey is where a purchase is
 * made, with a session that is allowed to make one.
 */

import {
  bootFlutter,
  clickControl,
  clickMatching,
  clickProduct,
  enableSemantics,
  findLabel,
  journey,
  openSurface,
  scrollToEnd,
  scrollToTop,
  semanticsLabels,
  waitForCatalogue,
  waitForLabel,
} from '../lib.mjs';

export const guestBrowse = journey({
  id: 'guest-browse',
  title: 'A guest browses the catalogue, filters it, and fills a cart',
  async run({ t, page, fixture, api }) {
    await openSurface(page, '/');
    await bootFlutter(page);
    await enableSemantics(page);

    // 0. The catalogue has to be on the page before anything is judged.
    //    The app is still booting when the tree is first read, and an empty
    //    tree says nothing about the app.
    const ready = await waitForCatalogue(page, fixture.catalogue.products);
    t.check(
      'the catalogue is on the page',
      ready !== null,
      ready ? ready.title : 'the catalogue never appeared',
    );
    if (!ready) return;

    // 1. "SHOP COLLECTION" is dead while no category is chosen, and live
    //    once one is. A button that is dead for a reason is fine; a button
    //    that is dead for no reason is the defect the audit looks for.
    //
    //    This is checked at the top of the page, before any scroll: the
    //    semantics tree is built lazily, and what is off the screen is not
    //    in it.
    const beforeFilter = findLabel(await semanticsLabels(page), 'SHOP COLLECTION', 'button');
    t.check(
      '"SHOP COLLECTION" is disabled while no category is chosen',
      beforeFilter !== null && beforeFilter.disabled,
      beforeFilter ? `aria-disabled=${beforeFilter.disabled}` : 'the button is not in the tree',
    );

    // 2. The category strip is a filter, not decoration.
    const nightwear = fixture.categories.entries.find((e) => e.categoryId === 'cat-nightwear');
    const other = fixture.catalogue.products.find((p) => p.category !== nightwear.categoryId);
    const before = await semanticsLabels(page);
    t.check(
      'a product of another category is on the page before the filter',
      findLabel(before, other.title, 'button') !== null,
      other.title,
    );

    await clickControl(page, 'button', nightwear.label);
    await page.waitForTimeout(1500);
    const filtered = await semanticsLabels(page);
    const inside = fixture.catalogue.products.filter(
      (p) => p.category === nightwear.categoryId && findLabel(filtered, p.title, 'button'),
    );
    const outside = fixture.catalogue.products.filter(
      (p) => p.category !== nightwear.categoryId && findLabel(filtered, p.title, 'button'),
    );
    t.check(
      'the category strip narrows the catalogue to that category',
      inside.length === nightwear.webProducts && outside.length === 0,
      `${inside.length} in the category, ${outside.length} outside it`,
    );

    // 3. With a filter applied, "SHOP COLLECTION" comes alive and clears it.
    const afterFilter = findLabel(await semanticsLabels(page), 'SHOP COLLECTION', 'button');
    t.check(
      '"SHOP COLLECTION" is enabled once a category is chosen',
      afterFilter !== null && !afterFilter.disabled,
      afterFilter ? `aria-disabled=${afterFilter.disabled}` : 'the button is not in the tree',
    );
    await clickControl(page, 'button', 'SHOP COLLECTION');
    await page.waitForTimeout(1500);
    await scrollToEnd(page);
    const cleared = await semanticsLabels(page);
    const stillAll = fixture.catalogue.products.filter((p) => findLabel(cleared, p.title, 'button'));
    t.check(
      '"SHOP COLLECTION" puts the whole catalogue back',
      stillAll.length === fixture.catalogue.webCount,
      `${stillAll.length} of ${fixture.catalogue.webCount} products on the page`,
    );

    // 4. The whole catalogue is on the page once it has been scrolled
    //    through — the tree is built lazily, so the products further down
    //    are only in it after the canvas has been scrolled there.
    await scrollToEnd(page);
    const loaded = await semanticsLabels(page);
    const shown = fixture.catalogue.products.filter((p) => findLabel(loaded, p.title, 'button'));
    t.check(
      'the catalogue shows every product the fixture set expects',
      shown.length === fixture.catalogue.webCount,
      `${shown.length} of ${fixture.catalogue.webCount} products on the page`,
    );

    // 5. Back to the top: the cart lives in the top bar, and the top bar is
    //    not in the tree while the page is scrolled down. The tree is built
    //    lazily in both directions.
    await scrollToTop(page);

    // 5. The product sheet shows the product, its price and its stock.
    const buy = fixture.journeys.buy;
    await clickProduct(page, buy.title);
    const sheet = await waitForLabel(page, 'SKU:', { timeout: 20_000 });
    t.check('the product sheet opens', sheet !== null, sheet ? 'the sheet shows the SKU' : 'the sheet never appeared');
    const sheetLabels = await semanticsLabels(page);
    const priceLabel = findLabel(sheetLabels, `${buy.priceMinor} JPY`);
    t.check(
      'the product sheet says the price in the currency\'s own major unit',
      priceLabel !== null,
      priceLabel ? priceLabel.label : `no label carries ${buy.priceMinor} JPY`,
    );

    // 6. Adding to the cart puts the line in it.
    await clickControl(page, 'button', 'カートに追加 / Add to cart');
    const badge = await waitForLabel(page, 'カート / Cart, 1 items', { timeout: 20_000 });
    t.check('the cart badge counts the line', badge !== null, badge ? badge.label : 'the badge never showed a count');
    if (!badge) {
      const after = await semanticsLabels(page);
      const error = findLabel(after, 'カートに追加できませんでした');
      t.note(
        error
          ? `the sheet said: ${error.label}`
          : 'the sheet said nothing — the click did not reach the server',
      );
    }

    // 7. The cart says what the server priced, and says so.
    await clickMatching(page, 'button', /カート \/ Cart/);
    await page.waitForTimeout(1200);
    const cartLabels = await semanticsLabels(page);
    const subtotal = findLabel(cartLabels, `${buy.priceMinor} JPY`);
    t.check(
      'the cart shows the price the server owns',
      subtotal !== null,
      subtotal ? subtotal.label : `no label carries ${buy.priceMinor} JPY`,
    );
    t.check(
      'the cart says the amount is recomputed server-side',
      findLabel(cartLabels, 'recalculated server-side') !== null,
      'the drawer carries the note about the server-side amount',
    );

    // 8. And the guest is told what a guest may do.
    await clickControl(page, 'button', '閉じる / Close');
    await page.waitForTimeout(600);
    await clickControl(page, 'button', 'アカウント / Account');
    await page.waitForTimeout(1000);
    const accountLabels = await semanticsLabels(page);
    const guest = findLabel(accountLabels, 'Session: ゲスト / Guest');
    t.check(
      'the console tells a guest it is browsing as a guest',
      guest !== null,
      guest ? guest.label : 'the session line never appeared',
    );
    t.note(
      'The account sheet says a guest can browse only, and the cart drawer offers a live ' +
        'Checkout button. Whether a guest may check out is stated in three places ' +
        '(docs/ecommerce/04-detailed/セキュリティ設計.md SEC-17, the client model, and the ' +
        'prototype) and enforced in none: the order service takes any x-session-id it is ' +
        'given. This journey stops at the cart, so it asserts neither.',
    );

    // The evidence, taken: what the server actually answers a guest session.
    const guestSessionResponse = await api.fetch('/v1/sessions/guest', {
      method: 'POST',
      body: { channel: 'web' },
    });
    const guestSession = guestSessionResponse.json?.sessionId;
    if (guestSession) {
      const checkout = await api.fetch('/v1/checkouts', {
        method: 'POST',
        headers: { 'x-session-id': guestSession, 'idempotency-key': `uat-guest-${Date.now()}` },
        body: {
          lines: [{ sku: fixture.journeys.buy.sku, quantity: 1 }],
          paymentMethodRef: 'tok_uat_guest',
        },
      });
      t.note(
        `A guest session's checkout answered **${checkout.status}** ` +
          `(${checkout.json?.orderId ? `order ${checkout.json.orderId}` : checkout.json?.code || 'no order'}). ` +
          'SEC-17 says the server must refuse this with 401; it does not.',
      );
    }
  },
});
