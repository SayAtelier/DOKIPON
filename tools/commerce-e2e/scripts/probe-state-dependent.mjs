import { chromium } from 'playwright';

const BASE = process.env.DOKIPON_BASE || 'https://dokipon-commerce.vercel.app';
const browser = await chromium.launch({
  channel: 'chrome',
  args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
const ctx = await browser.newContext({ viewport: { width: 1280, height: 900 } });
const page = await ctx.newPage();
const errs = [];
page.on('pageerror', (e) => errs.push('pageerror: ' + e.message));
page.on('console', (m) => { if (m.type() === 'error') errs.push('console: ' + m.text().split('\n')[0]); });

const sig = () => page.evaluate(() =>
  [...document.querySelectorAll('flt-semantics')]
    .map((n) => `${n.getAttribute('role') || '-'}|${(n.getAttribute('aria-label') || n.textContent || '').trim().slice(0, 60)}`)
    .join('\n'),
);
const clickText = async (re) => {
  const btns = page.locator('flt-semantics[role="button"]');
  const n = await btns.count();
  for (let i = 0; i < n; i++) {
    const l = await btns.nth(i).evaluate((x) => (x.getAttribute('aria-label') || x.textContent || '').trim());
    if (re.test(l)) { await btns.nth(i).click({ force: true, timeout: 8000 }); return l; }
  }
  return null;
};

// ---------- storefront ----------
await page.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
await page.waitForSelector('flutter-view', { state: 'attached', timeout: 240000 });
await page.waitForTimeout(3000);
await page.locator('flt-semantics-placeholder').first().evaluate((el) => el.click()).catch(() => {});
await page.waitForTimeout(1000);

const s0 = await sig();
console.log('storefront initial semantics lines =', s0.split('\n').length);
console.log('  has SHOP COLLECTION      :', /SHOP COLLECTION/.test(s0));
console.log('  has a LINGERIE category  :', /ランジェリー/.test(s0));

const before = await sig();
const hit = await clickText(/ランジェリー|LINGERIE/);
await page.waitForTimeout(2500);
const afterCat = await sig();
console.log(`\nclicked category "${hit}"`);
console.log('  semantics changed        :', before !== afterCat);
const lines = (s) => s.split('\n').filter((l) => /DOKIPON/.test(l)).length;
console.log('  product lines before/after:', lines(before), '/', lines(afterCat));

// now SHOP COLLECTION has something to clear
const beforeAll = await sig();
const hit2 = await clickText(/SHOP COLLECTION/);
await page.waitForTimeout(2500);
const afterAll = await sig();
console.log(`\nclicked "${hit2}" (after a category was selected)`);
console.log('  semantics changed        :', beforeAll !== afterAll);
console.log('  product lines before/after:', lines(beforeAll), '/', lines(afterAll));

// ---------- admin ----------
await page.goto(BASE + '/admin/', { waitUntil: 'domcontentloaded' });
await page.waitForSelector('flutter-view', { state: 'attached', timeout: 240000 });
await page.waitForTimeout(3000);
await page.locator('flt-semantics-placeholder').first().evaluate((el) => el.click()).catch(() => {});
await page.waitForTimeout(1000);
const inputs = page.locator('input, textarea');
if ((await inputs.count()) >= 2) {
  await inputs.nth(0).click(); await page.waitForTimeout(300);
  await page.keyboard.type('admin', { delay: 40 });
  await inputs.nth(1).click(); await page.waitForTimeout(300);
  await page.keyboard.type('dev-only-admin', { delay: 40 });
  await clickText(/Sign in/i);
  await page.waitForTimeout(3000);
}
const a0 = await sig();
console.log('\nadmin after login, semantics lines =', a0.split('\n').length);
const aBefore = await sig();
const tab = await clickText(/Catalog/i);
await page.waitForTimeout(2500);
const aAfter = await sig();
console.log(`clicked admin tab "${tab}"`);
console.log('  semantics changed        :', aBefore !== aAfter);
const aBefore2 = await sig();
const tab2 = await clickText(/Orders/i);
await page.waitForTimeout(2500);
const aAfter2 = await sig();
console.log(`clicked admin tab "${tab2}"`);
console.log('  semantics changed        :', aBefore2 !== aAfter2);

console.log('\nerrors:', JSON.stringify(errs.slice(0, 8)));
await browser.close();
