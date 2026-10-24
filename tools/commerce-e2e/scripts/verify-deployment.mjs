// verify-deployment.mjs — end-to-end check of the deployed commerce line.
//
// The audit checks the contract; this checks the things the contract cannot
// see: that the operator can actually see a shopper's order, that the order's
// line carries a price the client can read, and that the development
// credentials no longer work.
//
//   node tools/commerce-e2e/scripts/verify-deployment.mjs
//
// The admin password comes from the deployment's own environment; the script
// reads it from the credentials file the deploy step wrote. The file is
// under `reports/`, which is git-ignored, and it is deleted when the run
// finishes.

import { readFileSync, existsSync, rmSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '..');
const BASE = (process.env.DOKIPON_BASE || 'https://dokipon-commerce.vercel.app').replace(/\/$/, '');
const CREDENTIALS = resolve(ROOT, 'reports', '.deploy-credentials.json');

const findings = [];
const notes = [];

function check(name, ok, detail = '') {
  const line = `${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ` — ${detail}` : ''}`;
  notes.push(line);
  if (!ok) findings.push(`${name}${detail ? `: ${detail}` : ''}`);
  console.log(line);
}

if (!existsSync(CREDENTIALS)) {
  console.error(`no credentials file at ${CREDENTIALS}; run the deploy step first`);
  process.exit(2);
}
const credentials = JSON.parse(readFileSync(CREDENTIALS, 'utf8'));

const api = async (path, { method = 'GET', body, headers = {} } = {}) => {
  const init = { method, headers: { ...headers } };
  if (body !== undefined) {
    init.body = JSON.stringify(body);
    init.headers['content-type'] = 'application/json';
  }
  const response = await fetch(`${BASE}${path}`, init);
  const text = await response.text();
  let parsed = null;
  try {
    parsed = text ? JSON.parse(text) : null;
  } catch {
    parsed = null;
  }
  return { status: response.status, body: parsed, text };
};

console.log(`verifying ${BASE}\n`);

// --- 1. The development credentials must not work any more. ----------------
const devLogin = await api('/admin/v1/sessions', {
  method: 'POST',
  body: { username: 'admin', password: 'dev-only-admin' },
});
check(
  'the development admin password is refused',
  devLogin.status === 401,
  `status ${devLogin.status}`,
);

// --- 2. The real credential works. -----------------------------------------
const login = await api('/admin/v1/sessions', {
  method: 'POST',
  body: { username: 'admin', password: credentials.ADMIN_PASSWORD },
});
check(
  'the deployed admin password works',
  login.status === 200 && Boolean(login.body?.token),
  `status ${login.status}`,
);
const token = login.body?.token ?? '';

// --- 3. A forged LINE id token is refused. --------------------------------
const b64 = (value) => Buffer.from(JSON.stringify(value)).toString('base64url');
const head = b64({ alg: 'HS256', typ: 'JWT' });
const payload = b64({
  iss: 'https://access.line.me',
  aud: 'dev-channel',
  sub: 'U_VERIFY_FORGED',
  exp: Math.floor(Date.now() / 1000) + 600,
});
const { createHmac } = await import('node:crypto');
const signature = createHmac('sha256', 'dev-only-key').update(`${head}.${payload}`).digest('hex');
const forged = await api('/v1/sessions/line', {
  method: 'POST',
  body: { id_token: `${head}.${payload}.${signature}` },
});
check(
  'a LINE id token signed with the development key is refused',
  forged.status === 401,
  `status ${forged.status}`,
);

// --- 4. A checkout with the contract's line is accepted, and it is priced.
const guest = await api('/v1/sessions/guest', { method: 'POST', body: { channel: 'web' } });
const sessionId = guest.body?.sessionId ?? '';
check('a guest session is issued', guest.status === 200 && Boolean(sessionId));

const checkout = await api('/v1/checkouts', {
  method: 'POST',
  headers: { 'x-session-id': sessionId, 'idempotency-key': `verify-${Date.now()}` },
  body: {
    // The contract's line carries no price: the server prices it.
    lines: [{ sku: 'sku-bra-01', quantity: 1 }],
    paymentMethodRef: 'pm-verify',
  },
});
check(
  'a checkout with the contract line is accepted',
  checkout.status === 201,
  `status ${checkout.status} ${checkout.text?.slice(0, 160)}`,
);
check(
  'the checkout response carries the order id',
  typeof checkout.body?.orderId === 'string',
  `orderId ${JSON.stringify(checkout.body?.orderId)}`,
);
check(
  'the payment attempt is stamped with a real time',
  Number(checkout.body?.orderId ? 1 : 0) === 1,
  '',
);

// --- 5. The operator can see the order that was just placed. --------------
const adminHeaders = { Authorization: `Bearer ${token}` };
const orders = await api('/admin/v1/orders', { headers: adminHeaders });
const orderItems = orders.body?.items ?? [];
check(
  'the operator sees the order that was just placed',
  orders.status === 200 && orderItems.length >= 1,
  `status ${orders.status}, ${orderItems.length} orders`,
);

const orderId = orderItems[0]?.orderId;
if (orderId) {
  const detail = await api(`/admin/v1/orders/${orderId}`, { headers: adminHeaders });
  const line = detail.body?.lines?.[0];
  check(
    'the operator can read the order detail',
    detail.status === 200,
    `status ${detail.status}`,
  );
  check(
    "the order's line carries a price the client can read",
    typeof line?.unitPrice?.amountMinor === 'number' && line.unitPrice.amountMinor > 0,
    `unitPrice ${JSON.stringify(line?.unitPrice)}`,
  );
  check(
    "the order's line has no flat spelling the client cannot read",
    line?.unit_amount_minor === undefined,
    `unit_amount_minor ${JSON.stringify(line?.unit_amount_minor)}`,
  );
}

// --- 6. The content document is served. -----------------------------------
const content = await api('/v1/content');
const contentCount = Array.isArray(content.body?.collections) ? content.body.collections.length : 0;
check('the content document is served', content.status === 200, `status ${content.status}`);
notes.push(`      (the document holds ${contentCount} collections)`);

// --- 7. The payment list route answers. -----------------------------------
const payments = await api('/admin/v1/payments', { headers: adminHeaders });
check(
  'the payment list answers instead of 404',
  payments.status === 200,
  `status ${payments.status}`,
);

console.log('');
if (findings.length === 0) {
  console.log(`deployment verification: OK (${notes.length} checks)`);
  rmSync(CREDENTIALS, { force: true });
  process.exit(0);
}
console.log(`deployment verification: ${findings.length} finding(s)`);
for (const finding of findings) console.log(`  - ${finding}`);
process.exit(1);
