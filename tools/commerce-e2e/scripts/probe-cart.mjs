import { chromium } from 'playwright';

const BASE = process.env.DOKIPON_BASE || 'https://dokipon-commerce.vercel.app';

const browser = await chromium.launch({
  channel: 'chrome',
  args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
const errs = [];
page.on('pageerror', (e) => errs.push('pageerror: ' + e.message));
page.on('console', (m) => { if (m.type() === 'error') errs.push('console: ' + m.text()); });

await page.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
await page.waitForSelector('flutter-view', { state: 'attached', timeout: 45000 });
await page.waitForTimeout(2500);
await page.locator('flt-semantics-placeholder').first().evaluate((el) => el.click()).catch(() => {});
await page.waitForTimeout(800);

const btns = page.locator('flt-semantics[role="button"]');
const labels = [];
for (let i = 0; i < await btns.count(); i++) {
  labels.push(await btns.nth(i).evaluate((n) => n.getAttribute('aria-label') || n.textContent.trim()));
}

for (const want of [/カート \/ Cart/, /アカウント/, /OK/]) {
  const idx = labels.findIndex((l) => want.test(l));
  if (idx < 0) { console.log(`no control matching ${want}`); continue; }
  const before = errs.length;
  const semBefore = await page.locator('flt-semantics').count();
  let clickErr = null;
  try { await btns.nth(idx).click({ force: true, timeout: 6000 }); } catch (e) { clickErr = e.message.split('\n')[0]; }
  await page.waitForTimeout(1500);
  const semAfter = await page.locator('flt-semantics').count();
  const view = await page.evaluate(() => ({
    flutterView: document.querySelectorAll('flutter-view').length,
    glass: document.querySelectorAll('flt-glass-pane').length,
  }));
  console.log(`\n=== ${labels[idx]} ===`);
  console.log(`  clickError: ${clickErr || 'none'}`);
  console.log(`  newErrors: ${JSON.stringify(errs.slice(before))}`);
  console.log(`  semantics: ${semBefore} -> ${semAfter}`);
  console.log(`  dom: ${JSON.stringify(view)}`);
}

await browser.close();
