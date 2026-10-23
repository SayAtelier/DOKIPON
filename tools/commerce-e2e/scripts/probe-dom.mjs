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
await page.waitForSelector('flt-scene-host, flt-glass-pane, canvas', { state: 'attached', timeout: 45000 });
await page.waitForTimeout(2500);

const dump = async (label) => {
  const s = await page.evaluate(() => ({
    host: !!document.querySelector('flt-scene-host'),
    glass: !!document.querySelector('flt-glass-pane'),
    canvas: document.querySelectorAll('canvas').length,
    flutterView: document.querySelectorAll('flutter-view').length,
    semantics: document.querySelectorAll('flt-semantics').length,
    bodyChildren: [...document.body.children].map((e) => e.tagName.toLowerCase() + (e.id ? '#' + e.id : '')),
  }));
  console.log(label, JSON.stringify(s));
  return s;
};

await dump('before-semantics:');
await page.locator('flt-semantics-placeholder').first().evaluate((el) => el.click()).catch((e) => console.log('ph err', e.message));
await page.waitForTimeout(800);
await dump('after-semantics: ');

const btns = page.locator('flt-semantics[role="button"]');
console.log('buttons =', await btns.count());
const labels = [];
for (let i = 0; i < Math.min(await btns.count(), 6); i++) {
  labels.push(await btns.nth(i).evaluate((n) => n.getAttribute('aria-label') || n.textContent.trim()));
}
console.log('labels =', JSON.stringify(labels));

// Click a category button (a pure in-page interaction, no navigation).
const idx = labels.findIndex((l) => /ランジェリー|LINGERIE/.test(l));
console.log('clicking index', idx, labels[idx]);
if (idx >= 0) {
  try { await btns.nth(idx).click({ force: true, timeout: 5000 }); } catch (e) { console.log('click err:', e.message.split('\n')[0]); }
  await page.waitForTimeout(1200);
  await dump('after-click: ');
  console.log('errors:', JSON.stringify(errs.slice(0, 6)));
  console.log('html head:', (await page.content()).slice(0, 400).replace(/\s+/g, ' '));
}

await browser.close();
