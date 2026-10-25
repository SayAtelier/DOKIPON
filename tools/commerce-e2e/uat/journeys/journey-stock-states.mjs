/**
 * journey-stock-states.mjs — what the catalogue says about stock, and
 * whether the app believes it.
 *
 * Three states, and each one has to be *right* in two places: the card
 * says it, and the button behaves as it says.
 *
 *   - out of stock   — the add-to-cart button is dead, and both the card
 *                      and the product sheet say why.
 *   - made to order  — no stock level at all, and still purchasable.
 *   - in stock       — purchasable.
 *
 * The button is checked in the **product sheet**, not on the card: the
 * sheet's button is the one the shopper presses, and its label says which
 * state it is in. A card's button cannot be told apart from its
 * neighbours' by name alone, and a check that cannot say which button it
 * looked at is not a check.
 */

import {
  bootFlutter,
  clickProduct,
  enableSemantics,
  findLabel,
  journey,
  openSurface,
  scrollToEnd,
  semanticsLabels,
  waitForLabel,
} from '../lib.mjs';

export const stockStates = journey({
  id: 'stock-states',
  title: 'The catalogue says what the stock is, and the buttons agree',
  async run({ t, page, fixture }) {
    await openSurface(page, '/');
    await bootFlutter(page);
    await enableSemantics(page);
    await scrollToEnd(page);

    const expectations = [
      {
        key: 'outOfStock',
        cardSays: '在庫切れ / Out of stock',
        sheetButton: '在庫切れ / Out of stock',
        purchasable: false,
        why: 'a sold-out product offers a dead button and says why',
      },
      {
        key: 'madeToOrder',
        cardSays: '受注生産 / Made to order',
        sheetButton: 'カートに追加 / Add to cart',
        purchasable: true,
        why: 'a product with no stock level is still purchasable',
      },
      {
        key: 'buy',
        cardSays: '在庫あり / In stock',
        sheetButton: 'カートに追加 / Add to cart',
        purchasable: true,
        why: 'a product with stock on hand is purchasable',
      },
    ];

    for (const expectation of expectations) {
      const product = fixture.journeys[expectation.key];
      const labels = await semanticsLabels(page);
      const card = findLabel(labels, product.title, 'button');
      t.check(
        `the ${expectation.key} product is on the page`,
        card !== null,
        card ? card.label : `${product.title} is not in the tree`,
      );
      if (!card) continue;

      t.check(
        `the ${expectation.key} card says "${expectation.cardSays}"`,
        card.label.includes(expectation.cardSays),
        card.label,
      );

      // The sheet's own button: the one the shopper presses.
      await clickProduct(page, product.title);
      const sheet = await waitForLabel(page, `SKU: ${product.sku}`, { timeout: 20_000 });
      if (!sheet) {
        t.check(`the ${expectation.key} product sheet opens`, false, 'the sheet never appeared');
        continue;
      }
      // The sheet's own button: the one the shopper presses. It is waited
      // for rather than read once: the sheet's content is taller than the
      // screen, and the button is at the bottom of it.
      const button = await waitForLabel(page, expectation.sheetButton, {
        timeout: 15_000,
        role: 'button',
      });
      t.check(
        `the ${expectation.key} product sheet offers "${expectation.sheetButton}"`,
        button !== null,
        button ? button.label : 'the sheet has no such button',
      );
      if (button) {
        t.check(
          `the ${expectation.key} product's button is ${expectation.purchasable ? 'live' : 'dead'}`,
          expectation.purchasable ? !button.disabled : button.disabled,
          `aria-disabled=${button.disabled} — ${expectation.why}`,
        );
      }

      // Close the sheet before the next product.
      await page.keyboard.press('Escape');
      await page.waitForTimeout(900);
    }
  },
});
