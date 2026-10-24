# commerce-e2e — Playwright audit of the deployed DOKIPON commerce site

This directory holds the browser-level regression suite for the **deployed**
DOKIPON commerce deployment: the Flutter Web storefront at `/`, the Flutter
Web admin console at `/admin/`, and the Rust edge that serves the commerce
contract underneath both.

It lives next to the other mock-backed test assets rather than in a
top-level `tests/` directory, for the same reason `tools/rgs-flash-mock/`
does: a test that only exists in one place is a test nobody re-runs.

## What it does

```
npm install
npm run audit
```

The audit drives a real Chrome through every interactive control the app
exposes, and reports the ones that do not work.

**The contract half needs no browser and no `npm install`.** It talks to the
deployment over plain HTTP:

```
node scripts/audit.mjs            # the whole thing (browser + contract)
DOKIPON_AUDIT_API_ONLY=1 node scripts/audit.mjs   # the contract half only
```

So a machine — a CI job, the local deployment runner — can check the
contract without installing a browser library.

### Driving a Flutter Web app

Flutter renders to a canvas, so the buttons are not DOM elements and a
DOM-based test sees an empty page. The audit turns on Flutter's **semantics
tree** — the parallel DOM representation Flutter keeps for screen readers,
which stays off until something asks for it. Clicking the "enable
accessibility" placeholder is that ask, and afterwards every button, link
and field is a real element with a role and a label.

### What counts as a broken control

For each control the app is reloaded, the control is clicked, and five
things are checked:

| # | check |
|---|---|
| 1 | the click dispatches without an uncaught exception |
| 2 | no new console error appears |
| 3 | no request to the API fails |
| 4 | the app is still mounted afterwards |
| 5 | something actually happened — the URL moved, the semantics tree changed, or a request was made |

A control that passes 1–4 but fails 5 is reported as **inert**: it is
painted, it does not throw, and it does nothing. A dead button is invisible
to a screenshot, and that is the class of defect this audit exists to find.

### The API check

`mock_data/contract-expectations.json` is derived from
`contracts/openapi.yaml`, the contract of record. The audit checks the live
endpoints against it: status codes, required fields, and — the one that
matters most — that a product detail carries the **camelCase** field names
the Dart client reads (`productId`, `stockPolicy`, `channelEligibility`,
`titleI18n`, `sessionId`). A server that emits snake_case there is the exact
defect the HTML prototype documented: the client reads camelCase only, so the
field arrives empty and the product renders without a title.

## Reports

`reports/audit-<timestamp>.json` and `reports/audit-<timestamp>.md`. The
Markdown file is the one to read; the JSON is for diffing against a previous
run.

The audit exits non-zero when it finds anything, so it can be wired into CI
as a post-deploy gate.

## Environment

| variable | default | meaning |
|---|---|---|
| `DOKIPON_BASE` | `https://dokipon-commerce.vercel.app` | the deployment to audit |

## Note on the browser

The audit launches **system Chrome** (`channel: 'chrome'`), not Playwright's
bundled Chromium. The bundled browser build number does not match the
`ms-playwright` cache on this machine, so the bundled Chromium fails with
"Executable doesn't exist" while the system Chrome always works.

## Honest boundaries

- The commerce services keep their state **in memory**, and Vercel may run
  several instances of the same function at once. A cart created on one
  instance is invisible to another, so a cart can appear to empty itself.
  The catalogue, the content document and the admin console are unaffected —
  they are static or idempotent reads. This is a preview deployment;
  production means moving the mutable stores into a shared database, which is
  a change to the stores and not to the edge.
- The admin facade runs with its development credentials unless
  `ADMIN_USER` / `ADMIN_PASSWORD` are set as project environment variables.
  The audit therefore expects `/admin/v1/*` to answer **401** without a
  session, not 200.

## The local deployment — the same code, no deploy

The audit above runs against the **deployed** instance. A deploy takes about
sixteen minutes and the link to it is flaky, so the same code can also be run
**locally**, against the same public surface, without deploying anything.

```
npm run local          # start the stack, print the base URL
npm run local:audit    # start it, run the checks, tear it down, exit non-zero on a finding
```

### What it is

The deployment's **function topology**, run locally:

| the deployment | the local deployment |
|---|---|
| one Vercel Function per commerce service | one process per commerce service, each with its own in-memory state |
| the gateway Function (`/api/gateway`) | the `local-gateway` binary, serving **the same router** (`commerce_edge::msa::gateway::router(Gateway::from_env())`) over plain HTTP |
| the single-function fallback (`/api/index`) | the `commerce-edge` binary, which is the same fallback in one process |
| `vercel.json`'s rewrites and headers | **the same ones, read from `vercel.json` at run time** |
| the static bundle in `public/` | the same directory, served by the local origin |

The eight services are eight processes, because that is what the function
topology is: the cart, the order store and the inventory ledger live in the
service's own memory, and a test that ran them in one process would be
testing a topology the deployment does not have. The launcher prints the
process list while the stack is up.

The routing is **not** copied into the runner. The rewrites and the headers
come from `vercel.json`, and the Functions come from the `api/` tree
(`api/gateway.rs` is the Function at `/api/gateway`, `api/svc/cart.rs` the
one at `/api/svc/cart`). Change either and the local deployment changes
with it.

### Running it

| what | how |
|---|---|
| the Rust processes | Docker — the image carries the toolchain and a warm cargo cache. `npm run local` builds them the first time (a few minutes) and reuses them afterwards. |
| the local origin | Node, in this directory. No dependency beyond the ones the audit already needs. |
| the static bundle | **`flutter build web` first.** The Flutter bundle is not in the repository; see below. |

The API half of the audit works without the bundle. The browser half needs
it, because it drives the Flutter surfaces.

Building the bundle (the storefront at the root, the admin console under
`/admin/`), which is what `services/deploy/vercel/build-vercel.sh` does on
the Vercel builder:

```powershell
$env:PATH = 'C:\DevTools\flutter\bin;' + $env:PATH
cd apps/shop_web
flutter pub get
flutter build web --release --base-href / --no-web-resources-cdn `
  --dart-define=GEMARC_CHANNEL=web --dart-define=GEMARC_API_BASE=/
cd ..\admin_web
flutter pub get
flutter build web --release --base-href /admin/ --no-web-resources-cdn `
  --dart-define=GEMARC_CHANNEL=web --dart-define=GEMARC_API_BASE=/
```

Then assemble `public/` the way the deploy script does — the storefront at
the root, the admin console under `admin/`:

```powershell
New-Item -ItemType Directory -Force public\admin | Out-Null
Copy-Item apps\shop_web\build\web\* public\ -Recurse -Force
Copy-Item apps\admin_web\build\web\* public\admin\ -Recurse -Force
Remove-Item public\admin\flutter_service_worker.js -ErrorAction SilentlyContinue
```

A bundle that is already there can be pointed at with
`DOKIPON_LOCAL_BUNDLE=<dir>`.

### The credentials

The commerce services refuse to start while they hold their **development**
credentials, and they check that at the entry point. A local run is exactly
the case the escape hatch exists for, so the launcher sets
`DOKIPON_ALLOW_DEV_SECRETS=1` — a deployment never sets it.

The escape hatch only says "the development values are allowed"; the values
the stack actually runs with are **not** the development ones, because the
audit refuses a deployment that still accepts the development credential:

| variable | the local default | what it decides |
|---|---|---|
| `ADMIN_USER` | `admin` | who signs into the operator console |
| `ADMIN_PASSWORD` | `local-admin-password` | the operator's password — `dev-only-admin` would make the audit fail |
| `LINE_CHANNEL_ID` | `local-channel` | the audience a LINE id token is checked against |
| `LINE_ID_TOKEN_KEY` | `local-line-signing-key` | which id tokens are real |
| `PSP_WEBHOOK_SECRET` | `local-psp-webhook-secret` | which PSP webhook is real |
| `SERVICE_TOKEN` | `local-service-token` | how the admin facade proves it is an internal call |

Every one of them can be overridden from the environment. The launcher also
reads `tools/commerce-e2e/mock_data/local/` (a `.env` file or a
`credentials.json`) when it is there, and falls back to the defaults above
when it is not.

### The runner's own variables

| variable | default | meaning |
|---|---|---|
| `DOKIPON_LOCAL_BASE_PORT` | `41800` | the first port of the stack; the public origin is `base + 11` |
| `DOKIPON_LOCAL_BUNDLE` | `<repo>/public` | the static bundle's directory |
| `DOKIPON_VERCEL_JSON` | `<repo>/vercel.json` | the routing configuration to apply |
| `DOKIPON_LOCAL_NO_BUILD` | — | set it to skip the Docker build |
| `DOKIPON_LOCAL_VERBOSE` | — | set it to log every request the origin routes |
| `DOKIPON_LOCAL_IMAGE` | `rust:1.98.1-slim-bookworm` | the image the processes are built and run in |

### What it is faithful about

- the **rewrites and the headers** are the deployment's own, read from
  `vercel.json`; a rewrite that changes on the deployment changes here;
- the **gateway** is the same router the deployed gateway Function serves —
  the same routing table, the same upstream variables, the same BFF;
- the **eight services** are eight processes with eight in-memory states, so
  the topology is the deployment's;
- the **contract** is the same one: the same paths, the same status codes,
  the same ProblemDetails;
- the **credentials are checked the same way**: the development admin
  password, the development LINE key and the development PSP secret are all
  refused, which is what the audit's own assertions check.

### What it is not

- **One gateway *process*, not a Function.** A Vercel Function is isolated
  per request; the local gateway is a long-lived process. The routes, the
  routing table and the answers are the same, but the isolation is not.
- **One instance per service.** The deployment may run several instances of
  the same Function at once, which is why a cart can appear to empty itself
  there. Locally there is exactly one instance per service, so that hazard
  does not appear — the local run cannot reproduce it.
- **No CDN, no TLS, no caching.** The deployment terminates TLS and serves
  the bundle from Vercel's edge; the local origin serves plain HTTP from a
  directory. The `headers` of `vercel.json` are applied, the platform's own
  behaviour is not.
- **The bundle has to be built.** `flutter build web` is not part of the
  runner: it takes minutes and it is the same step the deploy does. Without
  a bundle the API surface works and the Flutter surfaces do not.
- **Vercel's smaller routing details are not modelled**: the trailing-slash
  redirect, `cleanUrls`, and the rewrite matcher supports the `(.*)`
  wildcard this deployment uses — not the named parameters Vercel also
  accepts.

