import { readFileSync, writeFileSync } from 'node:fs';

const a = 'D:/DOKIPON/tools/commerce-e2e/scripts/audit.mjs';
let js = readFileSync(a, 'utf8');
const before = js;

// 1. Remember which controls changed the page — one of them becomes the
//    primer for the second attempt.
js = js.replace(
  '          report.controls.push(await auditControl(page, control, state));',
  `          const outcome = await auditControl(page, control, state);
          report.controls.push(outcome);
          if (!outcome.problems.length) active.push(control);`,
);

js = js.replace(
  '      for (const control of controls) {',
  `      // Controls that changed the page when they were clicked. One of them
      // is used as a primer for the second attempt below.
      const active = [];

      for (const control of controls) {`,
);

// 2. The second attempt: a control that does nothing in the initial state
//    may still be correct — "show the whole collection" has nothing to clear
//    until a category is picked, and a tab that is already selected is a
//    no-op by design. Retry it after a primer has changed the page.
js = js.replace(
  '  report.consoleErrors = loadErrors.consoleErrors;',
  `  // Second attempt for the controls that did nothing: retry them after a
  // primer has changed the page. A control that reacts then is a
  // state-dependent no-op (correct behaviour), not a dead button.
  const noOps = report.controls.filter(
    (c) => c.problems && c.problems.length === 1 && c.problems[0].startsWith('inert:'),
  );
  if (noOps.length > 0 && active.length > 0) {
    const primer = active[0];
    const ctx2 = await browser.newContext({ viewport: VIEWPORT });
    const p2 = await ctx2.newPage();
    try {
      for (const c of noOps) {
        await p2.goto(\`\${BASE}\${path}\`, { waitUntil: 'domcontentloaded', timeout: BOOT_TIMEOUT });
        await waitForFlutter(p2);
        await enableSemantics(p2);
        if (name === 'admin' && report.login && report.login.ok) await tryAdminLogin(p2);
        // Change the page first.
        await p2.locator(CONTROL_SELECTOR).nth(primer.index).click({ force: true, timeout: 8000 }).catch(() => {});
        await p2.waitForTimeout(SETTLE_MS);
        // Then the control that did nothing.
        const target = p2.locator(CONTROL_SELECTOR).nth(c.index);
        const before2 = await probe(p2);
        let clickError2 = null;
        try {
          await target.scrollIntoViewIfNeeded({ timeout: 2000 }).catch(() => {});
          await target.click({ timeout: 5000, force: true });
        } catch (e) {
          clickError2 = String(e && e.message ? e.message : e).split('\\n')[0];
        }
        await p2.waitForTimeout(SETTLE_MS);
        const after2 = await probe(p2);
        const reacted = after2.url !== before2.url || after2.signature !== before2.signature;
        if (reacted && !clickError2) {
          c.problems = [];
          c.note = 'state-dependent no-op: nothing happens in the initial state and it reacts once the page has changed — correct behaviour, not a dead button';
        } else {
          c.problems = ['inert: the click changed nothing, in the initial state or after another control changed the page'];
        }
      }
    } finally {
      await ctx2.close();
    }
  }

  report.consoleErrors = loadErrors.consoleErrors;`,
);

if (js === before) {
  console.error('no replacement applied');
  process.exit(1);
}
writeFileSync(a, js, 'utf8');
console.log('audit.mjs: state-dependent no-op retry added');
