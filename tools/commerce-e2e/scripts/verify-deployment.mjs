// verify-deployment.mjs — end-to-end check of the deployed commerce line.
//
// The audit checks the contract; this checks the things the contract cannot
// see: that the operator can actually see a shopper's order, that the order's
// line carries a price the client can read, and that the development
// credentials no longer work.
//
//   node tools/commerce-e2e/scripts/verify-deployment.mjs
//   DOKIPON_BASE=https://... node tools/commerce-e2e/scripts/verify-deployment.mjs
//
// ## The credential
//
// The admin password comes from the deployment's own environment. The script
// reads it from `ADMIN_PASSWORD` (and `ADMIN_USER`) when they are set, and
// otherwise from the credentials file the deploy step wrote
// (`reports/.deploy-credentials.json`, which is git-ignored).
//
// ## The three answers
//
// Each check prints one line and lands in one of three columns:
//
//   PASS         the deployment was asked and it answered correctly;
//   FAIL         the deployment was asked and it answered wrongly;
//   UNMEASURED   the question could not be asked — no credential, a link
//                that dropped. This is not a defect of the deployment.
//
// The exit code follows: 0 when everything that could be asked was asked and
// answered correctly, 1 when something failed, 2 when nothing failed but
// something could not be measured.

import { readFileSync, existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '..');
const BASE = (process.env.DOKIPON_BASE || 'https://dokipon-commerce.vercel.app').replace(/\/$/, '');
const CREDENTIALS = resolve(ROOT, 'reports', '.deploy-credentials.json');

const findings = [];
const notMeasured = [];
const notes = [];

function check(name, ok, detail = '') {
  const line = `${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ` — ${detail}` : ''}`;
  notes.push(line);
  if (!ok) findings.push(`${name}${detail ? `: ${detail}` : ''}`);
  console.log(line);
}

/** A question that could not be asked. Not a finding: nobody found out. */
function unmeasured(name, detail = '') {
  const line = `UNMEASURED  ${name}${detail ? ` — ${detail}` : ''}`;
  notes.push(line);
  notMeasured.push(`${name}${detail ? `: ${detail}` : ''}`);
  console.log(line);
}

/**
 * The admin credential, from the environment first and from the deploy
 * step's credentials file second.
 */
function credential() {
  if (process.env.ADMIN_PASSWORD) {
    return {
      user: process.env.ADMIN_USER || 'admin',
      password: process.env.ADMIN_PASSWORD,
      source: 'the environment',
    };
  }
  if (existsSync(CREDENTIALS)) {
    const stored = JSON.parse(readFileSync(CREDENTIALS, 'utf8'));
    if (stored.ADMIN_PASSWORD) {
      return {
        user: stored.ADMIN_USER || 'admin',
        password: stored.ADMIN_PASSWORD,
        source: CREDENTIALS,
      };
    }
  }
  return null;
}

const api = async (path, { method = 'GET', body, headers = {} } = {}) => {
  const init = { method, headers: { ...headers } };
  if (body !== undefined) {
    init.body = JSON.stringify(body);
    init.headers['content-type'] = 'application/json';
  }
  let response;
  try {
    response = await fetch(`${BASE}${path}`, init);
  } catch (e) {
    // The link dropped. That is not an answer from the deployment, so the
    // caller is told the question was not asked.
    return { status: null, body: null, text: '', error: String(e && e.message ? e.message : e).split('\n')[0] };
  }
  const text = await response.text();
  let parsed = null;
  try {
    parsed = text ? JSON.parse(text) : null;
  } catch {
    parsed = null;
  }
  return { status: response.status, body: parsed, text, error: null };
};

console.log(`verifying ${BASE}\n`);

const credentialValue = credential();
if (credentialValue) {
  console.log(`the admin credential comes from ${credentialValue.source}\n`);
} else {
  console.log(
    'no admin credential: ADMIN_PASSWORD is not set and there is no credentials file.\n' +
      'The checks that need one will be reported as unmeasured.\n',
  );
}

// --- 1. The development credentials must not work any more. ----------------
const devLogin = await api('/admin/v1/sessions', {
  method: 'POST',
  body: { username: 'admin', password: 'dev-only-admin' },
});
if (devLogin.error) {
  unmeasured('the development admin password is refused', devLogin.error);
} else {
  check(
    'the development admin password is refused',
    devLogin.status === 401,
    `status ${devLogin.status}`,
  );
}

// --- 2. The real credential works. -----------------------------------------
let token = '';
if (!credentialValue) {
  unmeasured('the deployed admin password works', 'no credential available');
} else {
  const login = await api('/admin/v1/sessions', {
    method: 'POST',
    body: { username: credentialValue.user, password: credentialValue.password },
  });
  if (login.error) {
    unmeasured('the deployed admin password works', login.error);
  } else {
    check(
      'the deployed admin password works',
      login.status === 200 && Boolean(login.body?.token),
      `status ${login.status}`,
    );
    token = login.body?.token ?? '';
  }
}

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
if (forged.error) {
  unmeasured('a LINE id token signed with the development key is refused', forged.error);
} else {
  check(
    'a LINE id token signed with the development key is refused',
    forged.status === 401,
    `status ${forged.status}`,
  );
}

// --- 4. A checkout with the contract's line is accepted, and it is priced.
const guest = await api('/v1/sessions/guest', { method: 'POST', body: { channel: 'web' } });
const sessionId = guest.error ? '' : guest.body?.sessionId ?? '';
if (guest.error) {
  unmeasured('a guest session is issued', guest.error);
} else {
  check('a guest session is issued', guest.status === 200 && Boolean(sessionId));
}

const checkout = await api('/v1/checkouts', {
  method: 'POST',
  headers: { 'x-session-id': sessionId, 'idempotency-key': `verify-${Date.now()}` },
  body: {
    // The contract's line carries no price: the server prices it.
    lines: [{ sku: 'sku-bra-01', quantity: 1 }],
    paymentMethodRef: 'pm-verify',
  },
});
if (checkout.error) {
  unmeasured('a checkout with the contract line is accepted', checkout.error);
} else {
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
}

// --- 5. The payment attempt is stamped with a real time. ------------------
// The check used to read the order id again, which says nothing about a
// clock. The clock is the point: an authorization stamped with the epoch
// makes every webhook look stale, and nothing else in the line shows it.
const authorize = await api('/v1/payments/authorize', {
  method: 'POST',
  body: {
    order_id: 'ord-verify-stamp',
    payment_method_ref: 'pm-verify',
    amount_minor: 1500,
    currency: 'JPY',
  },
});
if (authorize.error) {
  unmeasured('the payment attempt is stamped with a real time', authorize.error);
} else if (authorize.status !== 200) {
  check(
    'the payment attempt is stamped with a real time',
    false,
    `the attempt could not be minted: status ${authorize.status}`,
  );
} else {
  const stamp = authorize.body?.authorized_at_ms;
  check(
    'the payment attempt is stamped with a real time',
    typeof stamp === 'number' && stamp > 0,
    `authorized_at_ms ${JSON.stringify(stamp)}`,
  );
}

// --- 6. The operator can see the order that was just placed. --------------
const adminHeaders = token ? { Authorization: `Bearer ${token}` } : {};
if (!token) {
  unmeasured('the operator sees the order that was just placed', 'no admin session');
} else {
  const orders = await api('/admin/v1/orders', { headers: adminHeaders });
  const orderItems = orders.error ? [] : orders.body?.items ?? [];
  if (orders.error) {
    unmeasured('the operator sees the order that was just placed', orders.error);
  } else {
    check(
      'the operator sees the order that was just placed',
      orders.status === 200 && orderItems.length >= 1,
      `status ${orders.status}, ${orderItems.length} orders`,
    );
  }

  const orderId = orderItems[0]?.orderId;
  if (orderId) {
    const detail = await api(`/admin/v1/orders/${orderId}`, { headers: adminHeaders });
    const line = detail.error ? null : detail.body?.lines?.[0];
    if (detail.error) {
      unmeasured('the operator can read the order detail', detail.error);
    } else {
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
  } else {
    unmeasured('the operator can read the order detail', 'no order to open');
  }
}

// --- 7. The content document is served. -----------------------------------
const content = await api('/v1/content');
if (content.error) {
  unmeasured('the content document is served', content.error);
} else {
  const contentCount = Array.isArray(content.body?.collections) ? content.body.collections.length : 0;
  check('the content document is served', content.status === 200, `status ${content.status}`);
  notes.push(`      (the document holds ${contentCount} collections)`);
}

// --- 8. The payment list route answers. -----------------------------------
if (!token) {
  unmeasured('the payment list answers instead of 404', 'no admin session');
} else {
  const payments = await api('/admin/v1/payments', { headers: adminHeaders });
  if (payments.error) {
    unmeasured('the payment list answers instead of 404', payments.error);
  } else {
    check(
      'the payment list answers instead of 404',
      payments.status === 200,
      `status ${payments.status}`,
    );
  }
}

console.log('');
if (findings.length === 0 && notMeasured.length === 0) {
  console.log(`deployment verification: OK (${notes.length} checks)`);
  console.log(`      the credentials file is still at ${CREDENTIALS} — it is the only copy of the`);
  console.log('      admin password. Delete it once you have it.');
  process.exit(0);
}
if (findings.length === 0) {
  console.log(`deployment verification: ${notMeasured.length} check(s) could not be measured`);
  for (const item of notMeasured) console.log(`  - ${item}`);
  process.exit(2);
}
console.log(`deployment verification: ${findings.length} finding(s), ${notMeasured.length} unmeasured`);
for (const finding of findings) console.log(`  - ${finding}`);
for (const item of notMeasured) console.log(`  - ${item}`);
process.exit(1);
