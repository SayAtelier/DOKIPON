/**
 * The local deployment's configuration.
 *
 * ## What this file is
 *
 * The addresses and the credentials of the local deployment. It is
 * deliberately *not* a routing table: the routing lives in `vercel.json`
 * and in the `api/` tree, and both are read at run time by
 * `rewrites.mjs`, so the local deployment and the deployment cannot drift
 * apart.
 *
 * ## The one mapping that is here
 *
 * `SERVICE_PROCESSES` says which local binary serves which Function, and
 * which base-URL variable that service is reached by. The local deployment
 * runs each service as its own process — that is what the function topology
 * is — so the Function's path and the process's address have to be tied
 * together somewhere. Two of the eight names are not a plain
 * `commerce-<name>`: the payment adapter, and the `PAYMENT_ADAPTER_URL`
 * the admin facade and the gateway read.
 */

import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const HERE = dirname(fileURLToPath(import.meta.url));

/** `tools/commerce-e2e` — the tool's own directory. */
export const E2E_ROOT = resolve(HERE, '..');

/** The repository root. */
export const REPO_ROOT = resolve(E2E_ROOT, '..', '..');

/**
 * The deployment's routing configuration.
 *
 * The local deployment reads its rewrites from here, so a rewrite that
 * changes on the deployment changes here too — there is no second copy.
 */
export const VERCEL_JSON = process.env.DOKIPON_VERCEL_JSON
  ? resolve(process.env.DOKIPON_VERCEL_JSON)
  : resolve(REPO_ROOT, 'vercel.json');

/** The Function entry points. `api/gateway.rs` answers at `/api/gateway`. */
export const API_DIR = resolve(REPO_ROOT, 'api');

/** The Rust workspace. */
export const SERVICES_DIR = resolve(REPO_ROOT, 'services');

/**
 * The static bundle: the storefront at the root, the admin console under
 * `admin/`.
 *
 * It is the `public/` directory the Vercel build assembles. A local run
 * needs it built first (`flutter build web`); the API half does not.
 */
export const BUNDLE_DIR = process.env.DOKIPON_LOCAL_BUNDLE
  ? resolve(process.env.DOKIPON_LOCAL_BUNDLE)
  : resolve(REPO_ROOT, 'public');

/** The first port of the local stack. Every other port is derived from it. */
export const BASE_PORT = Number(process.env.DOKIPON_LOCAL_BASE_PORT || 41800);

/**
 * One port per process.
 *
 * The container publishes these numbers unchanged, so an address means the
 * same thing inside the container and outside it.
 */
export const PORTS = {
  catalog: BASE_PORT + 1,
  cart: BASE_PORT + 2,
  order: BASE_PORT + 3,
  content: BASE_PORT + 4,
  identity: BASE_PORT + 5,
  inventory: BASE_PORT + 6,
  payment: BASE_PORT + 7,
  admin: BASE_PORT + 8,
  gateway: BASE_PORT + 9,
  edge: BASE_PORT + 10,
  public: BASE_PORT + 11,
};

/**
 * Which local process serves which Function.
 *
 * The Function's path is derived from the `api/` tree at run time; this
 * table only says which binary answers it and which base-URL variable
 * points at it.
 */
export const SERVICE_PROCESSES = {
  catalog: { binary: 'commerce-catalog', urlVar: 'CATALOG_URL', port: PORTS.catalog },
  cart: { binary: 'commerce-cart', urlVar: 'CART_URL', port: PORTS.cart },
  order: { binary: 'commerce-order', urlVar: 'ORDER_URL', port: PORTS.order },
  content: { binary: 'commerce-content', urlVar: 'CONTENT_URL', port: PORTS.content },
  identity: { binary: 'commerce-identity', urlVar: 'IDENTITY_URL', port: PORTS.identity },
  inventory: { binary: 'commerce-inventory', urlVar: 'INVENTORY_URL', port: PORTS.inventory },
  payment: {
    binary: 'commerce-payment-adapter',
    urlVar: 'PAYMENT_ADAPTER_URL',
    port: PORTS.payment,
  },
  admin: { binary: 'commerce-admin', urlVar: 'ADMIN_URL', port: PORTS.admin },
};

/** The Docker image the Rust processes are built and run in. */
export const IMAGE = process.env.DOKIPON_LOCAL_IMAGE || 'rust:1.98.1-slim-bookworm';

/** The Docker volume that holds the warm cargo registry cache. */
export const CARGO_VOLUME = process.env.DOKIPON_LOCAL_CARGO_VOLUME || 'dokipon-cargo-home';

/** The name of the container the local stack runs in. */
export const CONTAINER_NAME =
  process.env.DOKIPON_LOCAL_CONTAINER || `dokipon-local-${process.pid}`;

/**
 * The credentials the local stack runs with.
 *
 * Every one of them is overridable from the environment, and every one of
 * them has a value that is **not** its development value: the audit refuses
 * a deployment that still accepts the development credential, so a local
 * run that used the development values would fail the very checks it is
 * meant to run.
 */
export const CREDENTIALS = {
  adminUser: process.env.ADMIN_USER || 'admin',
  adminPassword: process.env.ADMIN_PASSWORD || 'local-admin-password',
  adminSessionId: process.env.ADMIN_SESSION_ID || 'admin-console',
  lineChannelId: process.env.LINE_CHANNEL_ID || 'local-channel',
  lineIdTokenKey: process.env.LINE_ID_TOKEN_KEY || 'local-line-signing-key',
  pspWebhookSecret: process.env.PSP_WEBHOOK_SECRET || 'local-psp-webhook-secret',
  serviceToken: process.env.SERVICE_TOKEN || 'local-service-token',
};

/**
 * The credentials, read from the fixture directory when it is there.
 *
 * The fixture files live in `tools/commerce-e2e/mock_data/local/`, which
 * another agent owns. Two shapes are understood: a `.env` file of
 * `KEY=VALUE` lines, and a `credentials.json` object. When neither is
 * there the defaults above are used, so the local stack works either way.
 */
export function loadCredentialFixtures() {
  const dir = resolve(E2E_ROOT, 'mock_data', 'local');
  if (!existsSync(dir)) return {};
  const out = {};

  const envFile = resolve(dir, '.env');
  if (existsSync(envFile)) {
    for (const line of readFileSync(envFile, 'utf8').split(/\r?\n/)) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) continue;
      const at = trimmed.indexOf('=');
      if (at < 1) continue;
      out[trimmed.slice(0, at).trim()] = trimmed.slice(at + 1).trim();
    }
  }

  const jsonFile = resolve(dir, 'credentials.json');
  if (existsSync(jsonFile)) {
    try {
      Object.assign(out, JSON.parse(readFileSync(jsonFile, 'utf8')));
    } catch {
      /* a fixture that does not parse is not worth failing the run over */
    }
  }
  return out;
}

/**
 * The credentials the stack is started with: the environment first, then
 * the fixtures, then the defaults.
 */
export function resolveCredentials() {
  const fixtures = loadCredentialFixtures();
  const pick = (name, fallback) => process.env[name] || fixtures[name] || fallback;
  return {
    adminUser: pick('ADMIN_USER', CREDENTIALS.adminUser),
    adminPassword: pick('ADMIN_PASSWORD', CREDENTIALS.adminPassword),
    adminSessionId: pick('ADMIN_SESSION_ID', CREDENTIALS.adminSessionId),
    lineChannelId: pick('LINE_CHANNEL_ID', CREDENTIALS.lineChannelId),
    lineIdTokenKey: pick('LINE_ID_TOKEN_KEY', CREDENTIALS.lineIdTokenKey),
    pspWebhookSecret: pick('PSP_WEBHOOK_SECRET', CREDENTIALS.pspWebhookSecret),
    serviceToken: pick('SERVICE_TOKEN', CREDENTIALS.serviceToken),
  };
}

/** The reports directory the audit and the verification write into. */
export const REPORTS_DIR = resolve(E2E_ROOT, 'reports');

/**
 * The base URLs every service is reached by, inside the container.
 *
 * They are the same addresses the gateway and the admin facade read from
 * the environment, and they are the addresses the local processes listen
 * on: the container publishes these ports unchanged.
 */
export function serviceUrls() {
  const urls = {};
  for (const [name, service] of Object.entries(SERVICE_PROCESSES)) {
    urls[service.urlVar] = `http://127.0.0.1:${service.port}`;
  }
  return urls;
}

/** Whether a directory exists and is a directory. */
export function isDirectory(path) {
  try {
    return statSync(path).isDirectory();
  } catch {
    return false;
  }
}

/** The entries of a directory, or an empty list when it is not there. */
export function listDir(path) {
  try {
    return readdirSync(path);
  } catch {
    return [];
  }
}
