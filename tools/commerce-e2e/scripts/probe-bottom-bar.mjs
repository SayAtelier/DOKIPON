import { chromium } from 'playwright';

const BASE = process.env.DOKIPON_BASE || 'https://dokipon-commerce.vercel.app';
const browser = await chromium.launch({
  channel: 'chrome',
  args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
await page.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
await page.waitForSelector('flutter-view', { state: 'attached', timeout: 240000 });
await page.waitForTimeout(3000);
await page.locator('flt-semantics-placeholder').first().evaluate((el) => el.click()).catch(() => {});
await page.waitForTimeout(1200);

const info = await page.evaluate(() => {
  const vh = window.innerHeight;
  const out = [];
  for (const n of document.querySelectorAll('flt-semantics')) {
    const r = n.getBoundingClientRect();
    const label = (n.getAttribute('aria-label') || n.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 48);
    if (!label) continue;
    out.push({
      role: n.getAttribute('role') || '-',
      label,
      top: Math.round(r.top),
      bottom: Math.round(r.bottom),
      h: Math.round(r.height),
      w: Math.round(r.width),
      inView: r.bottom > 0 && r.top < vh,
      belowFold: r.top > vh,
    });
  }
  return { vh, nodes: out };
});

console.log('viewport height =', info.vh);
const bottom = info.nodes.filter((n) => n.bottom > info.vh - 120).sort((a, b) => a.top - b.top);
console.log('\n靠近视口底部的节点（最可能被底部固定栏挡住）:');
for (const n of bottom) {
  console.log(`  ${n.role.padEnd(8)} top=${String(n.top).padStart(5)} bottom=${String(n.bottom).padStart(5)} h=${String(n.h).padStart(4)} inView=${n.inView} belowFold=${n.belowFold}  ${n.label}`);
}
await browser.close();
