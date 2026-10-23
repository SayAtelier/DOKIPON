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

const dump = async (label) => {
  const s = await page.evaluate(() => {
    const sem = [...document.querySelectorAll('flt-semantics')];
    return {
      inputs: [...document.querySelectorAll('input, textarea')].map((n) => ({
        tag: n.tagName, type: n.type, aria: n.getAttribute('aria-label'), ph: n.placeholder,
        val: n.value, rect: (() => { const r = n.getBoundingClientRect(); return [Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)]; })(),
      })),
      nodes: sem.map((n) => `${n.getAttribute('role') || '-'}|${n.getAttribute('aria-label') || (n.textContent || '').trim().slice(0, 40)}`).slice(0, 25),
    };
  });
  console.log(`\n===== ${label} =====`);
  console.log('inputs:', JSON.stringify(s.inputs, null, 1));
  console.log('nodes:', JSON.stringify(s.nodes, null, 1));
  return s;
};

await dump('after semantics');

// fill the credential fields
const inputs = page.locator('input, textarea');
const n = await inputs.count();
console.log('\ninput count =', n);
if (n >= 2) {
  await inputs.nth(0).fill('admin');
  await inputs.nth(1).fill('dev-only-admin');
  await page.waitForTimeout(300);
  await dump('after fill');

  const btn = page.locator('flt-semantics[role="button"]').filter({ hasText: /sign in/i }).first();
  console.log('\nsign-in button count =', await page.locator('flt-semantics[role="button"]').filter({ hasText: /sign in/i }).count());
  await btn.click({ force: true, timeout: 6000 }).catch((e) => console.log('click err:', e.message.split('\n')[0]));
  await page.waitForTimeout(3000);
  await dump('after sign-in');
  console.log('\nerrors:', JSON.stringify(errs.slice(0, 8)));
  console.log('inputs still present =', await page.locator('input, textarea').count());
}

await browser.close();

