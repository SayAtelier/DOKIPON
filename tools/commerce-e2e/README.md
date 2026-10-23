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
