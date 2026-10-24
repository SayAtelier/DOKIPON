// probe-admin-tabs.mjs — do the two admin tabs switch, and back?
//
// The audit reported "Orders Tab 1 of 2" as inert: the click changed nothing.
// Two readings, and they need different fixes:
//
//   (a) the Orders tab is *already* the selected one, so clicking it is a
//       no-op by design — the audit's instrument gap, because the DOM does
//       not say which tab is selected;
//   (b) the tab really does not switch back — a dead control.
//
// So: click Catalog, then click Orders, and see whether the screen comes
// back. That is the whole question, and it needs one page, not twenty-five.
import { chromium } from 'playwright';

const BASE = (process.env.DOKIPON_BASE || 'https://dokipon-commerce.vercel.app').replace(/\/$/, '');
const USER = process.env.ADMIN_USER || 'admin';
const PASSWORD = process.env.ADMIN_PASSWORD || 'dev-only-admin';

const browser = await chromium.launch({
  channel: 'chrome',
  args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
const page = await context.newPage();

await page.goto(`${BASE}/admin/`, { waitUntil: 'domcontentloaded', timeout: 300_000 });
const placeholder = page.locator('flt-semantics-placeholder').first();
if (await placeholder.count()) {
  await placeholder.evaluate((el) => el.click()).catch(() => {});
}
await page.locator('flt-semantics[role="button"]').first().waitFor({ timeout: 300_000 });
await page.waitForTimeout(1500);

// Sign in.
const inputs = page.locator('input, textarea');
if ((await inputs.count()) >= 2) {
  await inputs.nth(0).click();
  await page.waitForTimeout(300);
  await page.keyboard.type(USER, { delay: 40 });
  await inputs.nth(1).click();
  await page.waitForTimeout(300);
  await page.keyboard.type(PASSWORD, { delay: 40 });
  const signIn = page.locator('flt-semantics[role="button"]', { hasText: /sign in|ログイン/i }).first();
  if ((await signIn.count()) > 0) {
    await signIn.click({ force: true, timeout: 30_000 }).catch(() => {});
    await page.waitForTimeout(4000);
  }
}

const tab = (name) => page.locator('flt-semantics[role="tab"]', { hasText: name }).first();
const title = async () => {
  const t = await page.locator('flt-semantics').first().getAttribute('aria-label');
  return (t || '').replace(/\n/g, ' ');
};

const ordersTab = tab('Orders');
const catalogTab = tab('Catalog');
console.log(`the Orders tab is there:  ${(await ordersTab.count()) > 0}`);
console.log(`the Catalog tab is there: ${(await catalogTab.count()) > 0}`);
console.log(`the screen starts on:     ${JSON.stringify(await title())}`);

await catalogTab.click({ force: true, timeout: 60_000 });
await page.waitForTimeout(3000);
const afterCatalog = await title();
console.log(`after clicking Catalog:   ${JSON.stringify(afterCatalog)}`);

await ordersTab.click({ force: true, timeout: 60_000 });
await page.waitForTimeout(3000);
const afterOrders = await title();
console.log(`after clicking Orders:    ${JSON.stringify(afterOrders)}`);

const switchedOut = afterCatalog !== afterOrders;
console.log(switchedOut ? 'PASS: the tabs switch both ways' : 'FAIL: clicking Orders changed nothing');

await browser.close();
process.exit(switchedOut ? 0 : 1);
