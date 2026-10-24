// probe-admin-boot.mjs — does the admin console actually boot?
//
// The admin surface stopped loading in the browser while its assets are
// served fine over HTTP and the storefront boots normally. That pattern
// smells like the app throwing during startup, not like a slow link. This
// loads the page and prints whatever the console says.
import { chromium } from 'playwright';

const BASE = (process.env.DOKIPON_BASE || 'https://dokipon-commerce.vercel.app').replace(/\/$/, '');

const browser = await chromium.launch({
  channel: 'chrome',
  args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
const page = await context.newPage();

const console_ = [];
const pageErrors = [];
const failed = [];
page.on('console', (m) => console_.push(`${m.type()}: ${m.text()}`));
page.on('pageerror', (e) => pageErrors.push(String(e && e.message ? e.message : e)));
page.on('requestfailed', (r) =>
  failed.push(`${(r.failure() && r.failure().errorText) || '?'} ${r.url()}`),
);

const response = await page.goto(`${BASE}/admin/`, {
  waitUntil: 'domcontentloaded',
  timeout: 300_000,
});
console.log(`the document answered ${response.status()}`);

await page.waitForTimeout(90_000);

const shape = await page.evaluate(() => ({
  flutterView: !!document.querySelector('flutter-view'),
  glassPane: !!document.querySelector('flt-glass-pane'),
  semantics: document.querySelectorAll('flt-semantics').length,
  placeholder: !!document.querySelector('flt-semantics-placeholder'),
  bodyText: (document.body.innerText || '').slice(0, 300),
  scripts: [...document.querySelectorAll('script')].map((s) => s.src).filter(Boolean),
}));

console.log('\n--- the page ---');
console.log(JSON.stringify(shape, null, 2));
console.log('\n--- the console ---');
for (const line of console_.slice(0, 25)) console.log(`  ${line}`);
console.log('\n--- uncaught ---');
for (const line of pageErrors.slice(0, 10)) console.log(`  ${line}`);
console.log('\n--- requests that failed ---');
for (const line of failed.slice(0, 15)) console.log(`  ${line}`);

await browser.close();
