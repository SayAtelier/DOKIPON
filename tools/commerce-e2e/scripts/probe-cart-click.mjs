// probe-cart-click.mjs — does "add to cart" actually work?
//
// The audit reported eight "click failed: Timeout 5000ms exceeded" on the
// product cards. That is what a control looks like when the app has not
// finished rendering the catalogue yet: the button does not exist, so the
// click has nothing to click. The audit's per-control loop reloads the page
// for every control, and on a slow link the catalogue can take longer than
// the click's timeout.
//
// So this waits for the control to be there — and only then asks whether
// pressing it does anything. That is the difference between "the button is
// broken" and "the button was not there yet".
import { chromium } from 'playwright';

const BASE = (process.env.DOKIPON_BASE || 'https://dokipon-commerce.vercel.app').replace(/\/$/, '');

const browser = await chromium.launch({
  channel: 'chrome',
  args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
const page = await context.newPage();

const apiCalls = [];
page.on('request', (r) => {
  if (/\/v1\/|\/admin\/v1\//.test(r.url())) apiCalls.push(`${r.method()} ${r.url()}`);
});

await page.goto(`${BASE}/`, { waitUntil: 'domcontentloaded', timeout: 300_000 });
const placeholder = page.locator('flt-semantics-placeholder').first();
if (await placeholder.count()) {
  await placeholder.evaluate((el) => el.click()).catch(() => {});
}

// The control under test: "add to cart" on a product card.
const addToCart = page.locator('flt-semantics[role="button"]', { hasText: 'カートに追加' }).first();
await addToCart.waitFor({ timeout: 300_000 });
console.log('the add-to-cart control is on screen');

const cartBefore = await page
  .locator('flt-semantics[role="button"]', { hasText: 'カート' })
  .first()
  .getAttribute('aria-label');
console.log(`the cart before the click: ${JSON.stringify(cartBefore)}`);

const started = Date.now();
await addToCart.click({ timeout: 60_000 });
console.log(`the click took ${Date.now() - started}ms`);
await page.waitForTimeout(3000);

const cartAfter = await page
  .locator('flt-semantics[role="button"]', { hasText: 'カート' })
  .first()
  .getAttribute('aria-label');
console.log(`the cart after the click:  ${JSON.stringify(cartAfter)}`);
console.log('the API calls the click caused:');
for (const call of apiCalls.slice(-6)) console.log(`  ${call}`);

const changed = cartBefore !== cartAfter;
console.log(changed ? 'PASS: the control changed the cart' : 'FAIL: the control changed nothing');

await browser.close();
process.exit(changed ? 0 : 1);
