/**
 * The local "Vercel-alike": the HTTP server the local deployment is
 * published through.
 *
 * ## What it is
 *
 * The deployment serves one origin: the static Flutter bundle at the root
 * and the admin console under `/admin/`, and the commerce contract's paths
 * answered by the Functions. Vercel decides which of those answers a
 * request, and it decides it with `vercel.json` and the `api/` tree.
 *
 * This server makes the same decisions, in the same order:
 *
 *   1. **the filesystem** — a file in the bundle, or a Function's route;
 *   2. **the rewrites** — the first rewrite of `vercel.json` whose source
 *      matches the path; its destination is then resolved the same way;
 *   3. **404** — a path nothing owns.
 *
 * The rewrites and the headers are read from `vercel.json` at run time and
 * the Functions are read from the `api/` tree, so there is no second copy
 * of the routing table here.
 *
 * ## What it is not
 *
 * It is not Vercel. It does not run the Functions in a serverless runtime,
 * it does not scale them, and it does not cache anything. What it does is
 * route a request to the same place the deployment would route it, so a
 * test cannot tell the two apart by the answer it gets.
 */

import { createReadStream, existsSync, statSync } from 'node:fs';
import { createServer, request as httpRequest } from 'node:http';
import { join, resolve, sep } from 'node:path';

import {
  compileHeaders,
  compileRewrites,
  discoverFunctions,
  functionForPath,
  pathForFunction,
} from './rewrites.mjs';

/** The content types the bundle needs. */
const CONTENT_TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.map': 'application/json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.webp': 'image/webp',
  '.ttf': 'font/ttf',
  '.otf': 'font/otf',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.txt': 'text/plain; charset=utf-8',
  '.xml': 'application/xml',
  '.wasm.symbols': 'text/plain; charset=utf-8',
};

/** The headers that belong to one hop and must not be carried to the next. */
const HOP_BY_HOP = new Set([
  'connection',
  'keep-alive',
  'proxy-authenticate',
  'proxy-authorization',
  'te',
  'trailer',
  'transfer-encoding',
  'upgrade',
]);

/**
 * The file a request path serves, or `null`.
 *
 * The bundle's root is the storefront and `admin/` is the admin console, so
 * a directory answers with its `index.html` — which is what Vercel does
 * with a static output directory.
 */
export function resolveStaticFile(bundleDir, path) {
  if (!bundleDir || !existsSync(bundleDir)) return null;
  const root = resolve(bundleDir);
  const target = resolve(root, path.replace(/^\/+/, ''));
  // A path that climbs out of the bundle is not a file of the bundle.
  if (target !== root && !target.startsWith(root + sep)) return null;
  try {
    const stats = statSync(target);
    if (stats.isFile()) return target;
    if (stats.isDirectory()) {
      const index = join(target, 'index.html');
      if (existsSync(index)) return index;
    }
  } catch {
    /* no file at that path */
  }
  return null;
}

/** The content type of a file, from its extension. */
function contentTypeOf(file) {
  const dot = file.lastIndexOf('.');
  if (dot < 0) return 'application/octet-stream';
  const extension = file.slice(dot).toLowerCase();
  if (extension === '.symbols') return 'text/plain; charset=utf-8';
  return CONTENT_TYPES[extension] || 'application/octet-stream';
}

/**
 * The headers `vercel.json` adds to a response.
 *
 * Every rule that matches contributes its headers, in the order of the
 * configuration — the same way Vercel stacks them.
 */
export function headersFor(headerRules, path) {
  const headers = {};
  for (const rule of headerRules) {
    if (!rule.match.test(path)) continue;
    for (const header of rule.headers) headers[header.key] = header.value;
  }
  return headers;
}

/**
 * Start the local deployment's origin.
 *
 * `upstreams` maps a Function's route to the address of the local process
 * that serves it.
 */
export function startVercelAlike({ port, host = '127.0.0.1', vercelConfig, functions, bundleDir, upstreams, log = () => {} }) {
  const rewrites = compileRewrites(vercelConfig.rewrites);
  const headerRules = compileHeaders(vercelConfig.headers);

  const server = createServer((req, res) => {
    handle(req, res).catch((error) => {
      log(`error: ${error && error.stack ? error.stack : error}`);
      if (!res.headersSent) res.writeHead(500, { 'content-type': 'text/plain' });
      res.end('local deployment error\n');
    });
  });

  async function handle(req, res) {
    const rawUrl = req.url || '/';
    const questionMark = rawUrl.indexOf('?');
    const rawPath = questionMark < 0 ? rawUrl : rawUrl.slice(0, questionMark);
    const query = questionMark < 0 ? '' : rawUrl.slice(questionMark + 1);
    let path;
    try {
      path = decodeURIComponent(rawPath);
    } catch {
      path = rawPath;
    }

    const extraHeaders = headersFor(headerRules, path);

    // 1. the filesystem: the bundle, or a Function's route.
    const staticFile = resolveStaticFile(bundleDir, path);
    if (staticFile) {
      log(`static ${path} -> ${staticFile}`);
      res.writeHead(200, { 'content-type': contentTypeOf(staticFile), ...extraHeaders });
      createReadStream(staticFile).pipe(res);
      return;
    }

    const functionRoute = functionForPath(functions, path);
    if (functionRoute) {
      await forward(req, res, functionRoute, path, query, extraHeaders);
      return;
    }

    // 2. the rewrites, in the order `vercel.json` lists them.
    for (const rewrite of rewrites) {
      if (!rewrite.match.test(path)) continue;
      const destination = rewrite.destination;
      const destinationFile = resolveStaticFile(bundleDir, destination);
      if (destinationFile) {
        log(`rewrite ${path} -> ${destination} (static)`);
        res.writeHead(200, { 'content-type': contentTypeOf(destinationFile), ...extraHeaders });
        createReadStream(destinationFile).pipe(res);
        return;
      }
      if (functions.has(destination)) {
        log(`rewrite ${path} -> ${destination}`);
        await forward(req, res, destination, path, query, extraHeaders);
        return;
      }
      log(`rewrite ${path} -> ${destination} (no such file or Function)`);
      res.writeHead(404, { 'content-type': 'text/plain; charset=utf-8', ...extraHeaders });
      res.end('Not Found\n');
      return;
    }

    // 3. nothing owns the path.
    log(`404 ${path}`);
    res.writeHead(404, { 'content-type': 'text/plain; charset=utf-8', ...extraHeaders });
    res.end('Not Found\n');
  }

  /** Hand the request to the process that serves the Function. */
  async function forward(req, res, route, originalPath, query, extraHeaders) {
    const upstream = upstreams.get(route);
    if (!upstream) {
      res.writeHead(502, { 'content-type': 'application/problem+json', ...extraHeaders });
      res.end(
        JSON.stringify({
          type: 'https://api.gemarc.example/problems/no-upstream',
          title: 'Bad Gateway',
          status: 502,
          code: 'local.no_upstream',
          detail: `no local process serves ${route}`,
          traceId: '',
          retryable: false,
        }),
      );
      return;
    }

    const forwardPath = pathForFunction(route, originalPath);
    const target = new URL(upstream);
    const headers = {};
    for (const [name, value] of Object.entries(req.headers)) {
      if (HOP_BY_HOP.has(name) || name === 'host') continue;
      headers[name] = value;
    }
    headers.host = target.host;

    await new Promise((done) => {
      const upstreamRequest = httpRequest(
        {
          hostname: target.hostname,
          port: target.port || 80,
          path: query ? `${forwardPath}?${query}` : forwardPath,
          method: req.method,
          headers,
        },
        (upstreamResponse) => {
          const responseHeaders = {};
          for (const [name, value] of Object.entries(upstreamResponse.headers)) {
            if (HOP_BY_HOP.has(name)) continue;
            responseHeaders[name] = value;
          }
          res.writeHead(upstreamResponse.statusCode || 502, { ...responseHeaders, ...extraHeaders });
          upstreamResponse.pipe(res);
          res.on('finish', done);
        },
      );
      upstreamRequest.on('error', (error) => {
        log(`upstream ${route} unreachable: ${error.message}`);
        if (!res.headersSent) {
          res.writeHead(502, { 'content-type': 'application/problem+json', ...extraHeaders });
          res.end(
            JSON.stringify({
              type: 'https://api.gemarc.example/problems/upstream-unreachable',
              title: 'Bad Gateway',
              status: 502,
              code: 'local.upstream_unreachable',
              detail: `the local process serving ${route} is unreachable`,
              traceId: '',
              retryable: true,
            }),
          );
        }
        res.on('finish', done);
      });
      req.pipe(upstreamRequest);
    });
  }

  return new Promise((resolvePromise, rejectPromise) => {
    server.on('error', rejectPromise);
    server.listen(port, host, () => resolvePromise({ server, port, host }));
  });
}

export { discoverFunctions };
