# commerce-e2e scripts — the index

Every script in this directory answers **one** question about a DOKIPON
commerce deployment, and the index says which question belongs to which
script, what the script needs, how to run it, what it prints, and what its
exit code means.

If you have a question and do not know which script answers it, the
"Question → script" table below is the answer.

## Run the whole suite

```
npm install                    # once, in tools/commerce-e2e
node scripts/run-all.mjs       # every step, one summary, one exit code
```

`run-all.mjs` is the only entry point needed for a deployment check. It runs
the static checkers, the audit, the end-to-end verification and the browser
probes, then prints one summary:

```
  PASS           0.4s  topology         every contract path is served by both deployment topologies
  PASS           0.3s  secrets          the k3s manifests do not lie about credentials
  FAIL        1234.5s  audit            the contract, and every control on both surfaces
  UNMEASURED     0.2s  verify           the deployment facts the contract cannot see
  PASS          301.0s  cart-click      add to cart, given the time to render
```

Its exit code is the gate:

| code | meaning |
|---|---|
| 0 | every step ran and every step passed |
| 1 | at least one step found a defect in the deployment |
| 2 | no defect was found, but at least one question could not be answered |

The difference between 1 and 2 is the difference between "the deployment is
wrong" and "nobody found out". Both are non-zero: a suite that answers
nothing must not look like a suite that found nothing.

### Environment variables

| variable | default | meaning |
|---|---|---|
| `DOKIPON_BASE` | `https://dokipon-commerce.vercel.app` | the deployment to test |
| `DOKIPON_RUN_ALL_ONLY` | — | comma-separated step ids to run |
| `DOKIPON_RUN_ALL_SKIP` | — | comma-separated step ids to skip |
| `DOKIPON_RUN_ALL_API_ONLY` | — | `1` = the contract half only, no browser at all |
| `DOKIPON_RUN_ALL_STEP_TIMEOUT` | — | seconds after which one step is killed |
| `DOKIPON_AUDIT_API_ONLY` | — | `1` = the audit checks the contract only |
| `DOKIPON_PROBE_PATH` | `/` | the path `probe-page-load.mjs` opens |

The step ids are `topology`, `secrets`, `audit`, `verify`, `cart-click`,
`admin-boot`, `admin-tabs`, `state-dependent`, `page-load`.

## Question → script

| the question | the script |
|---|---|
| does the live API answer the contract? | `audit.mjs` (`DOKIPON_AUDIT_API_ONLY=1`) |
| does every control on the storefront and the admin console work? | `audit.mjs` |
| is a control that did nothing broken, or just state-dependent? | `audit.mjs` + `probe-state-dependent.mjs` |
| does the deployment refuse the development password and a forged LINE token? | `verify-deployment.mjs` |
| can the operator see the order that was just placed? | `verify-deployment.mjs` |
| does the order's line carry a price the client can read? | `verify-deployment.mjs` |
| does "add to cart" work, given the time to render? | `probe-cart-click.mjs` |
| does the admin console boot at all, or is it the link? | `probe-admin-boot.mjs` |
| do the admin tabs switch, and switch back? | `probe-admin-tabs.mjs` |
| does a page load with no failed request and no error response? | `probe-page-load.mjs` |
| is every contract path served by **both** topologies? | `check-topology-coverage.py` |
| do the k3s manifests lie about credentials? | `check-manifest-secrets.py` |
| what do the two topologies actually declare? | `dump-topology.py` |

## The scripts

### `run-all.mjs` — the entry point

- **What it proves**: nothing on its own. It runs every other script and
  reports one summary: what ran, what passed, what failed, what could not
  be measured, and the total.
- **What it needs**: a deployment (`DOKIPON_BASE`). Python for the two
  static checkers; Node and the `playwright` package for everything else.
  No credential.
- **How to run it**: `node scripts/run-all.mjs`.
- **What it prints**: each step's own output as it runs, then one summary
  table, then the reasons behind every step that is not `PASS`.
- **Exit code**: 0 = everything passed, 1 = a defect was found, 2 = a
  question could not be answered. See the table above.

The browser probes share **one** Chrome and **one** browser context, and
each probe gets a page of its own from it, so the Flutter bundle is fetched
once instead of once per probe. The audit launches its own browser: it is a
self-contained script, and its findings must not depend on how the suite
runs it.

### `audit.mjs` — the centerpiece

- **What it proves**: that the live deployment answers the contract, and
  that every interactive control on the storefront (`/`) and the admin
  console (`/admin/`) actually does something. For each control the app is
  reloaded, the control is clicked, and five things are checked: the click
  dispatches, no console error appears, no request fails, the app is still
  mounted, and something happened (the URL moved, the semantics tree
  changed, or a request was made). A control that passes the first four
  and fails the fifth is **inert** — painted, and dead.
- **What it needs**: a deployment (`DOKIPON_BASE`). A browser for the
  surface half; the contract half (`DOKIPON_AUDIT_API_ONLY=1`) needs
  neither. `ADMIN_USER` / `ADMIN_PASSWORD` for the screens behind the
  admin login.
- **How to run it**: `node scripts/audit.mjs`, or
  `DOKIPON_AUDIT_API_ONLY=1 node scripts/audit.mjs` for the contract half
  alone.
- **What it prints**: the whole report, and then the path of the Markdown
  report it wrote.
- **Exit code**: 0 = no findings, 1 = findings, 2 = the run itself failed.
- **Where the reports go**: `tools/commerce-e2e/reports/audit-<timestamp>.{json,md}`.
  The Markdown file is the one to read; the JSON is for diffing against a
  previous run.
- **What it does not cover**: a control that the semantics tree does not
  expose at the moment the list is taken is never audited. Flutter builds
  the tree lazily, so only part of the catalogue is in the list at that
  moment — the rest comes into the tree when the page is scrolled. A green
  run says every control the app exposed was checked; it does not say every
  control in the app was checked.

### `verify-deployment.mjs` — the facts the contract cannot see

- **What it proves**: that the deployment refuses the published development
  admin password, refuses a LINE id token signed with the development key,
  accepts a checkout that conforms to the contract, stamps the payment
  attempt with a real clock, serves the content document, answers the
  payment list route, and — with the admin credential — lets the operator
  see the order that was just placed and read its line's price.
- **What it needs**: a deployment (`DOKIPON_BASE`). The admin credential:
  `ADMIN_USER` / `ADMIN_PASSWORD` from the environment, or a credentials
  file — `reports/.deploy-credentials.json` for a deployed credential,
  `reports/.local-credentials.json` for the local stack's. The deployed one
  is looked up first and is the only copy of a password a Vercel Secret
  cannot give back; the local runner writes and removes **its own** file
  and never the deployed one. Keep the deployed credential somewhere safe
  as well as on this disk: losing the file means rotating the credential
  and redeploying, because there is nothing to read back. No browser.
- **How to run it**: `node scripts/verify-deployment.mjs`.
- **What it prints**: one line per check — `PASS`, `FAIL` or `UNMEASURED` —
  then a closing line with the totals.
- **Exit code**: 0 = every check was made and passed, 1 = a check failed,
  2 = nothing failed but a check could not be made (no credential, or the
  link dropped).

### `probe-cart-click.mjs` — does "add to cart" work?

- **What it proves**: that the product card's add-to-cart control works,
  given enough time to render. The audit clicks a control within five
  seconds of the page loading and calls a timeout a defect; this probe
  waits for the control to appear — up to five minutes — and only then asks
  whether pressing it changes the cart, makes a request, or raises an
  error.
- **What it needs**: a deployment (`DOKIPON_BASE`) and a browser. No
  credential.
- **How to run it**: `node scripts/probe-cart-click.mjs`.
- **What it prints**: one line per assertion — `PASS`, `FAIL` or
  `UNMEASURED`.
- **Exit code**: 0 = every assertion was measured and passed, 1 = an
  assertion failed, 2 = nothing failed but an assertion could not be
  measured.

### `probe-admin-boot.mjs` — does the admin console boot?

- **What it proves**: whether the admin console boots at all, or whether it
  is the link that is broken. The audit can only say "the surface did not
  load"; this probe separates the two from the evidence the page leaves
  behind: the document's status, whether Flutter mounted, whether an
  exception was thrown while the app started, and whether the deployment's
  own requests arrived.
- **What it needs**: a deployment (`DOKIPON_BASE`) and a browser. No
  credential — the login is not part of this question.
- **How to run it**: `node scripts/probe-admin-boot.mjs`.
- **What it prints**: one line per assertion, plus a `NOTE` with the
  shape of the page.
- **Exit code**: 0 / 1 / 2 as above.

### `probe-admin-tabs.mjs` — do the admin tabs switch?

- **What it proves**: that the admin console's two tabs switch, and switch
  **back**. The audit reported "Orders Tab 1 of 2" as inert; the two
  readings are "it was already selected, so clicking it again is a no-op by
  design" and "the tab is dead". This probe asks the round trip directly:
  click Catalog, then click Orders, and see whether the screen comes back.
- **What it needs**: a deployment (`DOKIPON_BASE`), a browser, and the admin
  credential (`ADMIN_USER` / `ADMIN_PASSWORD`) to get past the login. If the
  credential is refused the tabs cannot be reached, and the probe says
  `UNMEASURED` — the credential question belongs to
  `verify-deployment.mjs`.
- **How to run it**: `node scripts/probe-admin-tabs.mjs`.
- **What it prints**: one line per assertion.
- **Exit code**: 0 / 1 / 2 as above.

### `probe-state-dependent.mjs` — is a no-op control dead?

- **What it proves**: that a control which does nothing in the state the app
  starts in is **state-dependent** (correct) rather than **dead**. The
  "all products" control does nothing while the catalogue already shows
  everything; it must change the screen once a category is selected. This
  probe picks the primer itself — a category — and checks the round trip.
- **What it needs**: a deployment (`DOKIPON_BASE`) and a browser. No
  credential.
- **How to run it**: `node scripts/probe-state-dependent.mjs`.
- **What it prints**: one line per assertion.
- **Exit code**: 0 / 1 / 2 as above.

### `probe-page-load.mjs` — does a page load with nothing missing?

- **What it proves**: that a page loads with no failed request and no error
  response of the deployment's own. The audit records what happened while a
  surface loaded but does not judge it; this probe judges it.
- **What it needs**: a deployment (`DOKIPON_BASE`) and a browser. No
  credential.
- **How to run it**: `node scripts/probe-page-load.mjs`, or with a path:
  `node scripts/probe-page-load.mjs /admin/`.
- **What it prints**: one line per assertion.
- **Exit code**: 0 / 1 / 2 as above.

*This script was `probe-404.mjs`. It was renamed because it does not only
look for 404s: it judges the whole load. The old name described a symptom,
the new one describes the question.*

### `probe-lib.mjs` — the shape every probe wears

- **What it proves**: nothing. It is the shared code: the one-line result
  printer, the exit-code rule, the browser helpers (`bootFlutter`,
  `enableSemantics`, `signature`, `findControl`, `signIn`) and the
  classification of a failed request (the link dropped it, the font CDN
  dropped it, or the deployment did).
- **What it needs**: nothing to run it.
- **How to run it**: it is imported, never run.
- **What it prints**: nothing by itself.
- **Exit code**: none.

### `check-topology-coverage.py` — every contract path, both topologies

- **What it proves**: that every path in `contracts/openapi.yaml` is served
  by **both** deployment topologies — the k3s cluster and the Vercel
  functions. For each path it asks whether the owning service is deployed
  in that topology, reachable through that topology's routing, and given
  the environment it needs. It also compares the contract against the paths
  the Dart client calls and against the routes the public origin serves.
- **What it needs**: the repository (no deployment, no network) and PyYAML.
- **How to run it**: `python tools/commerce-e2e/scripts/check-topology-coverage.py`,
  or with a different set of manifests:
  `python tools/commerce-e2e/scripts/check-topology-coverage.py <k3s manifests dir>`.
- **What it prints**: the path-by-path coverage matrix, then the problems.
- **Exit code**: 0 = no gaps, 1 = gaps, 2 = PyYAML is missing.

### `check-manifest-secrets.py` — the manifests do not lie

- **What it proves**: that the k3s manifests do not claim credentials they
  do not have, and do not ship a published development credential. A
  `secretKeyRef` to a key that does not exist stops the container from
  starting at all.
- **What it needs**: the repository (no deployment, no network) and PyYAML.
- **How to run it**: `python tools/commerce-e2e/scripts/check-manifest-secrets.py`,
  or with a different set of manifests:
  `python tools/commerce-e2e/scripts/check-manifest-secrets.py <manifest dir>`.
- **What it prints**: the list of problems, or a one-line summary.
- **Exit code**: 0 = no problems, 1 = problems, 2 = PyYAML is missing.

### `dump-topology.py` — what the topologies declare

- **What it proves**: nothing on its own. It prints the raw facts the
  coverage matrix is built from: every k3s Deployment and Service with its
  port and its environment, and every Vercel Function with the project's
  rewrites and documented environment variables.
- **What it needs**: the repository and PyYAML.
- **How to run it**: `python tools/commerce-e2e/scripts/dump-topology.py`,
  or `python tools/commerce-e2e/scripts/dump-topology.py k3s` /
  `... vercel` for one topology.
- **What it prints**: JSON.
- **Exit code**: 0 = it printed, 2 = PyYAML is missing.

## What was deleted, and what covers it

Six scripts were removed. Each one answered a question that a surviving
script answers better, or printed free-form output and always exited 0.

| deleted | why | what covers the question now |
|---|---|---|
| `probe-cart.mjs` | clicked three storefront controls and dumped diagnostics; that is the audit's job, one control at a time, with a verdict per control | `audit.mjs` (every control, with a verdict) and `probe-cart-click.mjs` (the add-to-cart question, waited for properly) |
| `probe-dom.mjs` | dumped the DOM shape and clicked a category; the same question as the audit's surface half | `audit.mjs` (the surface half) and `probe-admin-boot.mjs` (the page's shape, with a verdict) |
| `probe-admin.mjs` | signed in with `fill()` and dumped the nodes. `fill()` does not work: Flutter's web text editing only mirrors genuine key input into its controller, so this probe was measuring its own technique | `audit.mjs` — its `tryAdminLogin` types the credential, checks the field received it, and reports `admin login: ok` or the reason it failed |
| `probe-admin2.mjs` | the same question as `probe-admin.mjs`, with the correct typing technique — which is what `audit.mjs` now does | `audit.mjs` — the login result is in the report, per surface |
| `probe-bottom-bar.mjs` | printed the semantics nodes near the viewport's bottom edge, free-form, always exiting 0 | `audit.mjs` — a control the app will not scroll into view is reported as `skipped: the app will not scroll it into view`, and the click error keeps Playwright's whole message, which names the element that intercepts pointer events |
| `probe-404.mjs` | printed every 4xx and every failed request of a page load, free-form, always exiting 0 | `probe-page-load.mjs` — the same evidence, judged: the deployment's own error status is a finding, a dropped request is `UNMEASURED` |

## The environment, and why it is not a defect

The deployment is reached over a slow, flaky link, and
`fonts.googleapis.com` is unreachable from here. The suite is built so that
those facts never look like a defect of the product:

- a control that could not be measured is reported as **unmeasured**, not as
  a broken control — the audit already makes this distinction and the
  probes keep it;
- a request that never arrived (a network-level error) is `UNMEASURED`: the
  deployment never got the chance to answer;
- a request to the font CDN is a `NOTE`, not a finding: the deployment does
  not own that host;
- a response the deployment itself sent (a 4xx or a 5xx) is a fact about
  the deployment, and is judged as one.

## A note on the browser

Every browser script launches **system Chrome** (`channel: 'chrome'`), not
Playwright's bundled Chromium: the bundled browser build does not match the
`ms-playwright` cache on this machine, so the bundled Chromium fails with
"Executable doesn't exist" while the system Chrome always works.
