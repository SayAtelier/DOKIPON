/**
 * The deployment's routing, read from the deployment.
 *
 * ## Why this file exists
 *
 * `vercel.json` is the deployment's routing configuration: the rewrites and
 * the headers. The local deployment must apply the **same** ones, or a test
 * run locally would be testing a different surface than the deployment
 * serves — which is exactly the class of defect the local runner exists to
 * catch.
 *
 * So the local runner does not have a routing table of its own. It reads:
 *
 * - the rewrites and the headers from `vercel.json`;
 * - the Functions from the `api/` tree — `api/gateway.rs` is the Function
 *   at `/api/gateway`, `api/svc/catalog.rs` the one at `/api/svc/catalog`.
 *
 * Change either and the local deployment changes with it.
 *
 * ## How a rewrite is matched
 *
 * Vercel matches a rewrite's `source` against the request path. The sources
 * this deployment uses are literal segments with a `(.*)` wildcard, so the
 * compiler below turns `source` into a regular expression by escaping
 * everything but the wildcard. A source with a named parameter
 * (`:id`) is not part of this deployment's configuration and is refused
 * loudly rather than matched wrongly.
 */

import { readFileSync } from 'node:fs';
import { join, relative, sep } from 'node:path';

/** Read `vercel.json`. */
export function loadVercelConfig(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

/** Escape the regular-expression metacharacters of a literal segment. */
function escapeRegExp(literal) {
  return literal.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Compile a rewrite `source` into a matcher for the request path.
 *
 * The path is matched without the query string, which is what Vercel
 * matches on.
 */
export function compileRewriteSource(source) {
  const text = String(source);
  if (text.includes(':')) {
    throw new Error(
      `the rewrite source "${text}" uses a named parameter, which this runner does not ` +
        'compile. Add it to compileRewriteSource() rather than letting it match wrongly.',
    );
  }
  const pattern = text.split('(.*)').map(escapeRegExp).join('(.*)');
  return new RegExp(`^${pattern}$`);
}

/**
 * The deployment's rewrites, compiled.
 *
 * The order is the order of `vercel.json`: Vercel applies the first
 * rewrite that matches, so the order is part of the configuration.
 */
export function compileRewrites(rewrites) {
  return (rewrites || []).map((rewrite) => ({
    source: rewrite.source,
    destination: rewrite.destination,
    match: compileRewriteSource(rewrite.source),
  }));
}

/**
 * The deployment's header rules, compiled.
 *
 * The rules are applied in order, and every rule that matches adds its
 * headers — the same way Vercel stacks them.
 */
export function compileHeaders(headers) {
  return (headers || []).map((rule) => ({
    source: rule.source,
    match: compileRewriteSource(rule.source),
    headers: rule.headers || [],
  }));
}

/**
 * The Functions of the deployment, read from the `api/` tree.
 *
 * `api/gateway.rs` is the Function at `/api/gateway`; `api/svc/cart.rs` is
 * the one at `/api/svc/cart`. The Function's route is the entry point's
 * path with the `.rs` dropped — and the `api/` directory itself is served
 * under `/api`, which is where Vercel serves it.
 */
export function discoverFunctions(apiDir, listDir, isDirectory) {
  const routes = new Map();
  const prefix = `/${apiDir.split(/[\\/]/).filter(Boolean).pop() || 'api'}`;
  const walk = (dir) => {
    for (const entry of listDir(dir)) {
      const full = join(dir, entry);
      if (isDirectory(full)) {
        walk(full);
        continue;
      }
      if (!entry.endsWith('.rs')) continue;
      const relativePath = relative(apiDir, full).split(sep).join('/');
      routes.set(`${prefix}/${relativePath.replace(/\.rs$/, '')}`, full);
    }
  };
  walk(apiDir);
  return routes;
}

/**
 * The Function that owns a path.
 *
 * A path belongs to a Function when it *is* the Function's route or starts
 * with the route and a slash — the same rule the service routers use for
 * their own prefixes. The longest route wins, so `/api/svc/cart` never
 * shadows `/api/svc/catalog`.
 */
export function functionForPath(functions, path) {
  let best = null;
  for (const route of functions.keys()) {
    if (path === route || path.startsWith(`${route}/`)) {
      if (!best || route.length > best.length) best = route;
    }
  }
  return best;
}

/**
 * The path a Function receives.
 *
 * The Function's own prefix is stripped, which is what Vercel does when it
 * hands a request to a Function, and what the service Functions' own mount
 * expects: the service's routes are rooted at `/`, so the service must see
 * the path without the Function's prefix.
 *
 * A path that does not carry the prefix is passed on unchanged — that is
 * the case where the gateway forwards a public path to a service.
 */
export function pathForFunction(route, path) {
  if (path === route) return '/';
  if (path.startsWith(`${route}/`)) return path.slice(route.length);
  return path;
}
