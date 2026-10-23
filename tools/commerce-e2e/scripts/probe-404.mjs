import { chromium } from 'playwright';

const BASE = process.env.DOKIPON_BASE || 'https://dokipon-commerce.vercel.app';
const path = process.argv[2] || '/admin/';

const browser = await chromium.launch({
  channel: 'chrome',
  args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
page.on('response', (r) => { if (r.status() >= 400) console.log(`  ${r.status()} ${r.url()}`); });
page.on('requestfailed', (r) => console.log(`  FAILED ${r.url()} ${(r.failure() || {}).errorText || ''}`));

await page.goto(BASE + path, { waitUntil: 'domcontentloaded' });
await page.waitForSelector('flutter-view', { state: 'attached', timeout: 120000 });
await page.waitForTimeout(4000);
console.log('done');
await browser.close();
