/**
 * journey-payment-refused.mjs — what happens when the payment provider
 * says no.
 *
 * The product's payment adapter reads `x-simulated-payment` and declines
 * the authorization, which is the provider refusing. The order must not be
 * confirmed, and the shopper must be told — silently confirming an order
 * that was never paid for is the worst thing a shop can do, and the
 * `payment_unknown` state exists precisely so that "we do not know" and
 * "it worked" are never confused.
 *
 * The header is set on the page, so the refusal is injected the way the
 * provider's own answer would arrive — in the request, not in a test-only
 * branch of the client.
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
  semanticsLabels,
  waitForLabel,
} from '../lib.mjs';

export const paymentRefused = journey({
  id: 'payment-refused',
  title: 'A refused payment does not become a confirmed order',
  async run({ t, page, fixture }) {
    // The provider's answer, injected into the request.
    await page.setExtraHTTPHeaders({ 'x-simulated-payment': 'declined' });

    await openSurface(page, '/');
    await bootFlutter(page);
    await enableSemantics(page);

    const buy = fixture.journeys.buy;
    await clickProduct(page, buy.title);
    await waitForLabel(page, 'SKU:', { timeout: 20_000 });
    await clickControl(page, 'button', 'カートに追加 / Add to cart');
    const badge = await waitForLabel(page, 'カート / Cart, 1 items', { timeout: 20_000 });
    if (!badge) {
      t.check('the line went into the cart', false, 'the badge never showed a count');
      return;
    }

    await clickMatching(page, 'button', /カート \/ Cart/);
    await page.waitForTimeout(1200);
    await clickControl(page, 'button', 'レジに進む / Checkout');

    // The answer arrives on the status bar at the bottom of the catalogue.
    const confirmed = await waitForLabel(page, 'ご注文を完了しました / Order confirmed.', { timeout: 30_000 });
    t.check(
      'a refused payment is never reported as a confirmed order',
      confirmed === null,
      confirmed ? 'the status bar said the order was confirmed' : 'the status bar did not confirm the order',
    );

    const refused = await waitForLabel(
      page,
      'ご注文を処理できませんでした / We could not process this order.',
      { timeout: 15_000 },
    );
    t.check(
      'the shopper is told the order could not be processed',
      refused !== null,
      refused ? refused.label : 'the status bar never said the order could not be processed',
    );

    const labels = await semanticsLabels(page);
    const pending = findLabel(labels, 'お支払いを確認中です');
    t.check(
      'the shopper is not told the payment is "being confirmed" either',
      pending === null,
      pending ? pending.label : 'no "we are confirming the payment" message',
    );
  },
});
