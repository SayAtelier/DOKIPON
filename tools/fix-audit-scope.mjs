import { readFileSync, writeFileSync } from 'node:fs';

const a = 'D:/DOKIPON/tools/commerce-e2e/scripts/audit.mjs';
let js = readFileSync(a, 'utf8');
const before = js;

// `active` is used by the second-attempt block, which sits outside the
// `else` branch where the control loop lives. Declare it at function scope.
js = js.replace(
  `      // Controls that changed the page when they were clicked. One of them
      // is used as a primer for the second attempt below.
      const active = [];

`,
  '',
);
js = js.replace(
  `  const loadErrors = {`,
  `  // Controls that changed the page when they were clicked. One of them is
  // used as a primer for the second attempt below.
  const active = [];

  const loadErrors = {`,
);
if (js === before) {
  console.error('no replacement applied');
  process.exit(1);
}
writeFileSync(a, js, 'utf8');
console.log('audit.mjs: active moved to function scope');
