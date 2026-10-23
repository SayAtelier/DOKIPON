import { chromium } from 'playwright';

const BASE = process.env.DOKIPON_BASE || 'https://dokipon-commerce.vercel.app';

const browser = await chromium.launch({
  channel: 'chrome',
  args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
const errs = [];
page.on('pageerror', (e) => errs.push('pageerror: ' + e.message));
page.on('console', (m) => { if (m.type() === 'error') errs.push('console: ' + m.text().split('\n')[0]); });
page.on('response', (r) => { if (r.url().includes('/admin/v1/')) console.log(`  HTTP ${r.status()} ${r.request().method()} ${r.url()}`); });

await page.goto(BASE + '/admin/', { waitUntil: 'domcontentloaded' });
await page.waitForSelector('flutter-view', { state: 'attached', timeout: 120000 });
await page.waitForTimeout(2500);
await page.locator('flt-semantics-placeholder').first().evaluate((el) => el.click()).catch(() => {});
await page.waitForTimeout(1000);

const inputs = page.locator('input, textarea');
console.log('input count =', await inputs.count());

// Real key events instead of fill(): Flutter's web text editing only
// reliably mirrors genuine key input into the TextEditingController.
await inputs.nth(0).click();
await page.keyboard.type('admin', { delay: 40 });
await inputs.nth(1).click();
await page.keyboard.type('dev-only-admin', { delay: 40 });
await page.waitForTimeout(400);

const readValues = () => page.evaluate(() => [...document.querySelectorAll('input, textarea')].map((n) => n.value));
console.log('dom values after typing =', JSON.stringify(await readValues()));

await page.locator('flt-semantics[role="button"]').filter({ hasText: /sign in/i }).first().click({ force: true, timeout: 6000 }).catch((e) => console.log('click err:', e.message.split('\n')[0]));
await page.waitForTimeout(3500);

console.log('dom values after sign-in =', JSON.stringify(await readValues()));
const nodes = await page.evaluate(() => [...document.querySelectorAll('flt-semantics')].map((n) => `${n.getAttribute('role') || '-'}|${(n.getAttribute('aria-label') || n.textContent || '').trim().slice(0, 50)}`));
console.log('nodes after sign-in =', JSON.stringify(nodes.slice(0, 20), null, 1));
console.log('errors:', JSON.stringify(errs.slice(0, 8)));

await browser.close();
