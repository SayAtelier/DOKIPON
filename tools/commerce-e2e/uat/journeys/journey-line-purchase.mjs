/**
 * journey-line-purchase.mjs — buying something, with a LINE session.
 *
 * This is the journey where the **mock** earns its place. The product never
 * calls an id-token issuer: it verifies tokens itself. The mock is the
 * other half of that handshake — it *issues* them, with the same key and
 * for the same channel the identity service verifies with, so a token the
 * mock mints is a token the deployment accepts.
 *
 * The bridge (`window.__gemarcLiff`) is how the app learns there is a LINE
 * session at all. It is the one place the LIFF SDK is touched, and it is
 * read once, exchanged for a server session, and discarded. The harness
 * publishes the bridge before the app boots, with a token the mock just
 * minted — which is exactly what the LIFF SDK would have done inside the
 * LINE app.
 *
 * A deployment whose `LINE_ID_TOKEN_KEY` is a platform secret cannot be
 * given a token by a test, because the key cannot be read back. This
 * journey is therefore `UNMEASURED` against such a deployment, and it says
 * so rather than passing quietly.
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

export const linePurchase = journey({
  id: 'line-purchase',
  title: 'A LINE user signs in, buys a product, and the order exists',
  needs: { mock: true },
  async run({ t, page, fixture, mock, api, session }) {
    // 1. The mock mints a token the deployment's identity service accepts.
    const line = fixture.line;
    const mintUrl = `${mock}${line.mintPath}?flavour=${line.flavour}` +
      `&sub=${encodeURIComponent(line.subject)}&nonce=${encodeURIComponent(line.nonce)}`;
    const minted = await fetch(mintUrl, { signal: AbortSignal.timeout(20_000) });
    const token = await minted.json();
    t.check(
      'the mock mints a LINE-shaped id_token',
      minted.ok && typeof token.idToken === 'string' && token.idToken.length > 20,
      token.idToken ? `minted a token for ${token.sub}` : `the mock answered ${minted.status}`,
    );
    if (!token.idToken) return;

    // 2. The bridge the app reads, published before it boots.
    await page.addInitScript((idToken) => {
      window.__gemarcLiff = {
        ready: true,
        isInClient: true,
        isLoggedIn: true,
        idToken,
        errorCode: null,
      };
    }, token.idToken);

    // The bridge has to be in place before the app boots, so the navigation
    // comes after it.
    await openSurface(page, '/');
    await bootFlutter(page);
    await enableSemantics(page);

    // 3. The app exchanged the token for a server session.
    await clickControl(page, 'button', 'アカウント / Account');
    await page.waitForTimeout(1200);
    const account = await semanticsLabels(page);
    const signedIn = findLabel(account, 'Session: LINE ユーザー / LINE user');
    t.check(
      'the app signed the LINE user in with the mock\'s token',
      signedIn !== null,
      signedIn
        ? signedIn.label
        : `the exchange answered ${session.exchange ? session.exchange.status : 'nothing'} ` +
          `${session.exchange ? JSON.stringify(session.exchange.body) : ''}`,
    );
    t.check(
      'the app says this session may check out',
      findLabel(account, '購入できます / You can check out') !== null,
      'the account sheet carries the "you can check out" line',
    );
    t.check(
      'the app got a server session for the token',
      Boolean(session.id),
      session.id
        ? `the identity service answered a ${session.kind} session`
        : 'no session answer was captured',
    );

    // 4. The catalogue. The **web-only** product is not in the LINE
    //    channel's catalogue — which is the catalogue's job, not the
    //    client's, so it is asked of the API.
    const lineCatalogue = await api.fetch('/v1/products?channel=line');
    const lineItems = lineCatalogue.json?.items || [];
    t.check(
      'the LINE channel\'s catalogue leaves the web-only product out',
      lineCatalogue.status === 200 && lineItems.length === fixture.catalogue.lineCount,
      `channel=line answered ${lineItems.length} of ${fixture.catalogue.webCount}`,
    );
    t.check(
      'the web-only product is not in the LINE channel\'s catalogue',
      !lineItems.some((item) => item.productId === fixture.journeys.restrictedFromLine.productId),
      `${fixture.journeys.restrictedFromLine.productId} is web-only`,
    );

    const buy = fixture.journeys.buy;
    // The account sheet is a modal: it has to be closed before the catalogue
    // behind it can be clicked.
    await page.keyboard.press('Escape');
    await page.waitForTimeout(900);
    await clickProduct(page, buy.title);
    await waitForLabel(page, 'SKU:', { timeout: 20_000 });
    await clickControl(page, 'button', 'カートに追加 / Add to cart');
    const badge = await waitForLabel(page, 'カート / Cart, 1 items', { timeout: 20_000 });
    t.check('the cart holds the line', badge !== null, badge ? badge.label : 'the badge never showed a count');

    // 5. Check out.
    await clickMatching(page, 'button', /カート \/ Cart/);
    await page.waitForTimeout(1200);
    await clickControl(page, 'button', 'レジに進む / Checkout');
    const confirmed = await waitForLabel(page, 'ご注文を完了しました / Order confirmed.', { timeout: 45_000 });
    t.check(
      'the checkout is confirmed',
      confirmed !== null,
      confirmed ? confirmed.label : 'the status bar never said the order was confirmed',
    );

    // 6. And the order is really there, for that session.
    const orders = await api.fetch('/v1/orders', {
      headers: session.id ? { 'x-session-id': session.id } : {},
    });
    t.check(
      'the order is in the session\'s own order list',
      orders.status === 200 && Array.isArray(orders.json?.items) && orders.json.items.length >= 1,
      orders.status === 200
        ? `${orders.json.items.length} order(s) for the session`
        : `the order list answered ${orders.status}`,
    );
  },
});
