/**
 * The local deployment's launcher.
 *
 * ## What it does
 *
 * `node local/stack.mjs serve`   — start the stack and keep it up.
 * `node local/stack.mjs audit`   — start it, run the checks, tear it down.
 *
 * The stack is the deployment's topology, run locally:
 *
 *   - the **eight commerce services**, one process each, one in-memory
 *     state each — that is what the Function topology is;
 *   - the **gateway**, the same router the Vercel gateway Function serves;
 *   - the **single-process fallback**, the whole contract in one process,
 *     which is what `/api/index` is rewritten to;
 *   - the **local origin**, which applies the rewrites read from
 *     `vercel.json` and serves the static bundle.
 *
 * The Rust processes run in one container, because the deployment's
 * processes are Linux processes and a developer machine is not. The
 * container publishes the ports unchanged, so an address means the same
 * thing inside and outside it.
 *
 * ## What it guarantees
 *
 * - it waits until the stack answers before it hands out the base URL;
 * - it tears the stack down on the way out, including when a check fails;
 * - it leaves no orphan process and no bound port behind.
 */

import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { connect } from 'node:net';
import { resolve } from 'node:path';

import {
  API_DIR,
  BASE_PORT,
  BUNDLE_DIR,
  CARGO_VOLUME,
  CONTAINER_NAME,
  E2E_ROOT,
  IMAGE,
  PORTS,
  REPO_ROOT,
  REPORTS_DIR,
  SERVICE_PROCESSES,
  VERCEL_JSON,
  listDir,
  isDirectory,
  MOCK_PROCESSES,
  resolveCredentials,
  serviceUrls,
} from './config.mjs';
import { discoverFunctions, loadVercelConfig } from './rewrites.mjs';
import { startVercelAlike } from './vercel-alike.mjs';

const REPO_POSIX = REPO_ROOT.replace(/\\/g, '/');

/** The Rust packages the local deployment runs. */
const CARGO_PACKAGES = [
  'commerce-catalog',
  'commerce-cart',
  'commerce-order',
  'commerce-content',
  'commerce-identity',
  'commerce-inventory',
  'commerce-payment-adapter',
  'commerce-admin',
  'commerce-edge',
  // The LINE id-token issuer, so a test can mint a token the deployment's
  // identity service accepts. It is a fixture, not a product dependency —
  // nothing in `services/` points at it.
  'commerce-mocks',
];

/** Run a command, capturing its output. */
function run(command, args, options = {}) {
  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(command, args, {
      cwd: options.cwd || REPO_ROOT,
      env: { ...process.env, ...(options.env || {}) },
      stdio: ['ignore', 'pipe', 'pipe'],
      shell: false,
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
      if (options.echo) process.stdout.write(chunk);
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
      if (options.echo) process.stderr.write(chunk);
    });
    child.on('error', rejectPromise);
    child.on('close', (code) => resolvePromise({ code, stdout, stderr }));
  });
}

/** Whether a TCP port accepts a connection. */
function portOpen(port, host = '127.0.0.1', timeout = 500) {
  return new Promise((resolvePromise) => {
    const socket = connect({ port, host });
    const finish = (open) => {
      socket.destroy();
      resolvePromise(open);
    };
    socket.setTimeout(timeout);
    socket.once('connect', () => finish(true));
    socket.once('timeout', () => finish(false));
    socket.once('error', () => finish(false));
  });
}

/** Wait until a port accepts a connection. */
async function waitForPort(port, what, timeoutMs = 60_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await portOpen(port)) return true;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`${what} did not open port ${port} within ${timeoutMs} ms`);
}

/** The health of the local stack, as a table. */
export async function stackHealth(base) {
  const paths = ['/healthz', '/v1/products', '/admin/v1/session'];
  const out = [];
  for (const path of paths) {
    const response = await fetch(`${base}${path}`);
    const text = await response.text();
    out.push({
      path,
      status: response.status,
      body: text.length > 160 ? `${text.slice(0, 160)}…` : text,
    });
  }
  return out;
}

/** The local deployment. */
export class LocalStack {
  constructor(options = {}) {
    this.log = options.log || ((line) => console.log(line));
    this.credentials = resolveCredentials();
    this.containerStarted = false;
    this.origin = null;
    this.functions = discoverFunctions(API_DIR, listDir, isDirectory);
    this.vercelConfig = loadVercelConfig(VERCEL_JSON);
  }

  /** The address the local deployment is published through. */
  get base() {
    return `http://127.0.0.1:${PORTS.public}`;
  }

  /** Build the Rust binaries, if they are not there yet. */
  async build({ force = false } = {}) {
    const binaries = [
      ...Object.values(SERVICE_PROCESSES).map((service) => service.binary),
      ...Object.values(MOCK_PROCESSES).map((mock) => mock.binary),
      'local-gateway',
      'commerce-edge',
    ];
    const built = binaries.every((binary) =>
      existsSync(resolve(REPO_ROOT, 'services', 'target', 'release', binary)),
    );
    if (built && !force && process.env.DOKIPON_LOCAL_SKIP_BUILD === '1') {
      this.log('==> the binaries are already built; skipping the build');
      return;
    }
    if (built && !force && process.env.DOKIPON_LOCAL_NO_BUILD === '1') {
      this.log('==> the binaries are already built; skipping the build');
      return;
    }
    this.log('==> building the commerce binaries (this is the slow part)');
    const result = await run(
      'docker',
      [
        'run',
        '--rm',
        '-v',
        `${REPO_POSIX}:/work`,
        '-v',
        `${CARGO_VOLUME}:/cargo`,
        '-e',
        'CARGO_HOME=/cargo',
        '-e',
        'CARGO_TERM_COLOR=never',
        '-w',
        '/work/services',
        IMAGE,
        'cargo',
        'build',
        '--release',
        ...CARGO_PACKAGES.flatMap((name) => ['-p', name]),
      ],
      { echo: false },
    );
    if (result.code !== 0) {
      throw new Error(`the build failed:\n${result.stdout}\n${result.stderr}`);
    }
    this.log('==> the binaries are built');
  }

  /** The base URLs the services are reached by, inside the container. */
  serviceEnv() {
    const env = {
      ...serviceUrls(),
      // A local run is exactly the case the escape hatch is for: the
      // deployment must never set it, a developer machine may.
      DOKIPON_ALLOW_DEV_SECRETS: process.env.DOKIPON_ALLOW_DEV_SECRETS || '1',
      ADMIN_USER: this.credentials.adminUser,
      ADMIN_PASSWORD: this.credentials.adminPassword,
      ADMIN_SESSION_ID: this.credentials.adminSessionId,
      LINE_CHANNEL_ID: this.credentials.lineChannelId,
      LINE_ID_TOKEN_KEY: this.credentials.lineIdTokenKey,
      PSP_WEBHOOK_SECRET: this.credentials.pspWebhookSecret,
      SERVICE_TOKEN: this.credentials.serviceToken,
      RUST_LOG: process.env.RUST_LOG || 'info',
    };
    return env;
  }

  /** Start the container the Rust processes run in. */
  async startContainer() {
    await this.sweepStaleContainers();
    await run('docker', ['rm', '-f', CONTAINER_NAME], { echo: false });

    const args = ['run', '-d', '--rm', '--name', CONTAINER_NAME];
    args.push('--label', `dokipon.local=true`);
    args.push('--label', `dokipon.local.pid=${process.pid}`);
    args.push('-v', `${REPO_POSIX}:/work`);
    args.push('-w', '/work');
    for (const [name, value] of Object.entries(this.serviceEnv())) {
      args.push('-e', `${name}=${value}`);
    }
    args.push('-e', `LOCAL_BASE_PORT=${BASE_PORT}`);
    // Every port is published unchanged, so an address means the same thing
    // inside the container and outside it.
    for (const port of [
      ...Object.values(SERVICE_PROCESSES).map((service) => service.port),
      ...Object.values(MOCK_PROCESSES).map((mock) => mock.port),
      PORTS.gateway,
      PORTS.edge,
    ]) {
      args.push('-p', `127.0.0.1:${port}:${port}`);
    }
    args.push(IMAGE, 'bash', '/work/tools/commerce-e2e/local/supervise.sh');

    const result = await run('docker', args, { echo: false });
    if (result.code !== 0) {
      throw new Error(`the container did not start:\n${result.stdout}\n${result.stderr}`);
    }
    this.containerStarted = true;
    this.log(`==> the container ${CONTAINER_NAME} is up`);
  }

  /**
   * Remove the containers a killed run left behind.
   *
   * A run that is killed outright — not stopped, not failed — cannot tear
   * itself down, and its container would keep the ports bound, so the next
   * run would fail to start. Two cases:
   *
   * - a container this runner started carries the label with the pid of
   *   the run that owns it. If that pid is gone, the run was killed, and
   *   the container is removed. A container whose owner is still alive is
   *   left alone: that is a run in progress.
   * - a container named like this one but carrying **no** such label was
   *   started by a runner that did not label it, so there is no owner to
   *   ask about. It cannot be a run of this version, and it holds the
   *   ports, so it goes.
   */
  async sweepStaleContainers() {
    const listed = await run(
      'docker',
      [
        'ps',
        '-a',
        '--filter',
        'label=dokipon.local=true',
        '--format',
        '{{.ID}} {{.Label "dokipon.local.pid"}}',
      ],
      { echo: false },
    );
    if (listed.code !== 0) return;
    for (const line of listed.stdout.split(/\r?\n/)) {
      const [id, pid] = line.trim().split(/\s+/);
      if (!id || !pid) continue;
      if (Number(pid) === process.pid) continue;
      let alive = true;
      try {
        process.kill(Number(pid), 0);
      } catch {
        alive = false;
      }
      if (alive) continue;
      this.log(`==> removing the container ${id} left behind by the run that owned pid ${pid}`);
      await run('docker', ['rm', '-f', id], { echo: false });
    }

    const unlabelled = await run(
      'docker',
      [
        'ps',
        '-a',
        '--filter',
        'name=dokipon-local-',
        '--format',
        '{{.ID}} {{.Label "dokipon.local.pid"}} {{.Names}}',
      ],
      { echo: false },
    );
    if (unlabelled.code !== 0) return;
    for (const line of unlabelled.stdout.split(/\r?\n/)) {
      const [id, pid, name] = line.trim().split(/\s+/);
      if (!id || !name) continue;
      if (pid) continue;
      if (name === CONTAINER_NAME) continue;
      this.log(`==> removing the container ${name} left behind by an earlier runner`);
      await run('docker', ['rm', '-f', id], { echo: false });
    }
  }

  /** The address of the process that serves each Function. */
  upstreams() {
    const upstreams = new Map();
    upstreams.set('/api/gateway', `http://127.0.0.1:${PORTS.gateway}`);
    upstreams.set('/api/index', `http://127.0.0.1:${PORTS.edge}`);
    for (const [name, service] of Object.entries(SERVICE_PROCESSES)) {
      upstreams.set(`/api/svc/${name}`, `http://127.0.0.1:${service.port}`);
    }
    return upstreams;
  }

  /** Start the local origin. */
  async startOrigin() {
    this.origin = await startVercelAlike({
      port: PORTS.public,
      vercelConfig: this.vercelConfig,
      functions: this.functions,
      bundleDir: BUNDLE_DIR,
      upstreams: this.upstreams(),
      log: (line) => {
        if (process.env.DOKIPON_LOCAL_VERBOSE === '1') this.log(`    ${line}`);
      },
    });
    this.log(`==> the local origin is up: ${this.base}`);
    if (!existsSync(BUNDLE_DIR)) {
      this.log(
        `==> no static bundle at ${BUNDLE_DIR}: the API surface works, the Flutter surfaces do not.\n` +
          '    Build it with `flutter build web` (see the README), or point DOKIPON_LOCAL_BUNDLE at a bundle.',
      );
    }
  }

  /** Bring the whole stack up. */
  async start(options = {}) {
    await this.build(options);
    await this.startContainer();

    const ports = [
      ...Object.entries(SERVICE_PROCESSES).map(([name, service]) => [
        `the ${name} service`,
        service.port,
      ]),
      ['the gateway', PORTS.gateway],
      ['the single-process fallback', PORTS.edge],
      ...Object.entries(MOCK_PROCESSES).map(([name, mock]) => [
        `the ${name} mock`,
        mock.port,
      ]),
    ];
    for (const [what, port] of ports) {
      await waitForPort(port, what);
      this.log(`==> ${what} is listening on ${port}`);
    }

    await this.startOrigin();
    await waitForPort(PORTS.public, 'the local origin');

    // The stack is up when the deployment's own health check answers.
    const deadline = Date.now() + 30_000;
    for (;;) {
      try {
        const response = await fetch(`${this.base}/healthz`);
        if (response.status === 200) break;
      } catch {
        /* not up yet */
      }
      if (Date.now() > deadline) throw new Error('the local stack did not become healthy');
      await new Promise((r) => setTimeout(r, 250));
    }
    this.log('==> the local stack is healthy');
  }

  /** The processes of the stack, as the container sees them. */
  async processList() {
    const result = await run('docker', ['top', CONTAINER_NAME, '-o', 'pid,args'], {
      echo: false,
    });
    if (result.code !== 0) return result.stderr;
    return result.stdout;
  }

  /** The containers the local deployment left behind. */
  async leftoverContainers() {
    const result = await run(
      'docker',
      ['ps', '-a', '--filter', `name=${CONTAINER_NAME}`, '--format', '{{.Names}} {{.Status}}'],
      { echo: false },
    );
    return result.stdout.trim();
  }

  /**
   * Write the local stack's own credential, for a standalone verification
   * run against a stack this runner left up.
   *
   * It goes to **its own file**, not to the deploy step's. The deployed
   * credential lives at `reports/.deploy-credentials.json` and is the only
   * copy of a password a Vercel Secret cannot give back; this runner used
   * to write the local credential over it and then delete it, so a local
   * test run destroyed the deployed credential. The two are now separate
   * files, and this one is the only one this runner may touch.
   */
  writeCredentials() {
    mkdirSync(REPORTS_DIR, { recursive: true });
    const file = resolve(REPORTS_DIR, '.local-credentials.json');
    writeFileSync(
      file,
      `${JSON.stringify(
        {
          ADMIN_USER: this.credentials.adminUser,
          ADMIN_PASSWORD: this.credentials.adminPassword,
          SERVICE_TOKEN: this.credentials.serviceToken,
        },
        null,
        2,
      )}\n`,
    );
    return file;
  }

  /** Run the checks against the local stack. */
  async runChecks() {
    const failures = [];

    this.log('');
    // The contract half needs no browser, which keeps the local run quick.
    // The browser half needs the static bundle and a Chrome; it is opt-in.
    const full = process.env.DOKIPON_LOCAL_FULL_AUDIT === '1';
    this.log(
      `==> the audit${full ? '' : ', API half'}, against the local stack`,
    );
    const audit = await run(
      'node',
      [resolve(E2E_ROOT, 'scripts', 'audit.mjs')],
      {
        cwd: E2E_ROOT,
        env: {
          DOKIPON_BASE: this.base,
          DOKIPON_AUDIT_API_ONLY: full ? '' : '1',
          // The audit signs into the operator console with these, so the
          // local run has to hand it the credential the local stack runs
          // with — otherwise it tries the development password and the
          // console's own screens are never audited.
          ADMIN_USER: this.credentials.adminUser,
          ADMIN_PASSWORD: this.credentials.adminPassword,
        },
        echo: true,
      },
    );
    if (audit.code !== 0) failures.push(`the audit reported findings (exit ${audit.code})`);

    this.log('');
    this.log('==> the end-to-end verification against the local stack');
    const credentialsFile = this.writeCredentials();
    const verify = await run(
      'node',
      [resolve(E2E_ROOT, 'scripts', 'verify-deployment.mjs')],
      {
        cwd: E2E_ROOT,
        env: { DOKIPON_BASE: this.base, ADMIN_PASSWORD: this.credentials.adminPassword },
        echo: true,
      },
    );
    if (verify.code !== 0) failures.push(`the verification reported findings (exit ${verify.code})`);
    // Only this runner's own file is removed. The deployed credential is
    // not this runner's to delete: it is the only copy of a password the
    // platform cannot give back.
    rmSync(credentialsFile, { force: true });

    return { failures };
  }

  /**
   * Run the UAT against the local stack.
   *
   * The UAT is the only thing here that needs the **mock**: the LINE
   * journey signs in with a token the mock mints, and the identity service
   * verifies it with the same key. That is the mock↔product handshake, live,
   * and it is why the local stack runs the issuer.
   */
  async runUat() {
    const failures = [];
    this.log('');
    this.log('==> the UAT, against the local stack');
    const uat = await run(
      'node',
      [resolve(E2E_ROOT, 'uat', 'run.mjs')],
      {
        cwd: E2E_ROOT,
        env: {
          DOKIPON_BASE: this.base,
          ADMIN_USER: this.credentials.adminUser,
          ADMIN_PASSWORD: this.credentials.adminPassword,
        },
        echo: true,
      },
    );
    if (uat.code !== 0) failures.push(`the UAT reported findings (exit ${uat.code})`);
    return { failures };
  }

  /** Tear the stack down. */
  async stop() {
    if (this.origin) {
      const { server } = this.origin;
      // A browser that is still holding a keep-alive connection would keep
      // `close()` waiting for it; the connections are cut so the teardown
      // cannot hang on a socket nobody is going to close.
      if (typeof server.closeAllConnections === 'function') server.closeAllConnections();
      await new Promise((resolvePromise) => server.close(resolvePromise));
      this.origin = null;
    }
    if (this.containerStarted) {
      await run('docker', ['stop', '--timeout', '5', CONTAINER_NAME], { echo: false });
      // `--rm` removes the container, but a container that was still starting
      // up would not be removed by it; this makes sure nothing is left.
      await run('docker', ['rm', '-f', CONTAINER_NAME], { echo: false });
      this.containerStarted = false;
    }
  }

  /** The ports the stack used, and whether they are free now. */
  async portState() {
    const ports = [
      ...Object.values(SERVICE_PROCESSES).map((service) => service.port),
      PORTS.gateway,
      PORTS.edge,
      PORTS.public,
    ];
    const state = [];
    for (const port of ports) {
      state.push({ port, open: await portOpen(port) });
    }
    return state;
  }
}

/** The launcher. */
async function main() {
  const mode = process.argv[2] || 'serve';
  const stack = new LocalStack();
  let code = 0;
  try {
    await stack.start();
    stack.log(`\nbase URL: ${stack.base}\n`);
    stack.log(await stackHealth(stack.base));
    stack.log('');
    stack.log('==> the processes of the local stack (one per service):');
    stack.log(await stack.processList());
    stack.log('');

    if (mode === 'audit') {
      const { failures } = await stack.runChecks();
      if (failures.length) {
        for (const failure of failures) stack.log(`FAIL  ${failure}`);
        code = 1;
      } else {
        stack.log('PASS  the local stack answered every check');
      }
    } else if (mode === 'uat') {
      const { failures } = await stack.runUat();
      if (failures.length) {
        for (const failure of failures) stack.log(`FAIL  ${failure}`);
        code = 1;
      } else {
        stack.log('PASS  every journey held');
      }
    } else {
      await new Promise((resolvePromise) => {
        const stop = () => resolvePromise();
        process.on('SIGINT', stop);
        process.on('SIGTERM', stop);
        stack.log('==> press Ctrl-C to stop the local stack');
      });
    }
  } catch (error) {
    console.error(`the local stack failed: ${error && error.message ? error.message : error}`);
    code = 2;
  } finally {
    stack.log('==> tearing the local stack down');
    await stack.stop();
    const state = await stack.portState();
    const stillOpen = state.filter((entry) => entry.open);
    if (stillOpen.length) {
      console.error(`these ports are still bound: ${stillOpen.map((e) => e.port).join(', ')}`);
      code = code || 3;
    } else {
      stack.log(`==> every port is free again (${state.map((e) => e.port).join(', ')})`);
    }
    const leftover = await stack.leftoverContainers();
    if (leftover) console.error(`a container is left behind: ${leftover}`);
  }
  process.exit(code);
}

if (process.argv[1] && process.argv[1].endsWith('stack.mjs')) {
  main();
}
