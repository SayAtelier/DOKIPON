# mocks/ — 6 Rust mocks in `services/crates/commerce-mocks/`

The mocks are *not* in this directory. They are a Rust workspace member at
`services/crates/commerce-mocks/`. This file documents how to start them
and what control endpoints exist.

## Honest disclaimer

The mocks cover the **v5.4 → v8.2 design intent**. The v8.2 atlas itself was
judged **DEGRADED / NOT FULLY ACCEPTED** (`docs/gemarc-5d-design-atlas/QA_FINAL.md`).
The mocks prove the implementation matches the v5.4 invariants; those
invariants are also the foundation v8.2 builds on, so they remain useful,
but they are not a stamp of approval on the v8.2 design.

If `docs/gemarc-5d-design-atlas/QA_FINAL.md` ever moves to "ACCEPTED" status,
this file's disclaimer should be reviewed.

## The 6 mocks

Each mock is a separate binary, a separate process, on its own port. They
are designed to be started independently so a test can stand up only the
ones it needs.

| Binary | Crate | Port | What it mocks | Test case it serves |
|---|---|---|---|---|
| `mock-psp` | `commerce-mocks::psp` | 9001 | Payment provider (authorize / capture / refund / reconcile) | `CASE-CHAOS-001` (PSP outage) |
| `mock-broker` | `commerce-mocks::broker` | 9002 | At-least-once event broker with redelivery control | `CASE-IT-002` (Outbox→Inbox redelivery) |
| `mock-postgres` | `commerce-mocks::postgres` | 9003 | Constraint-enforcing in-memory store. **Not** Postgres. | `CASE-IT-001` (DDL constraints) and `CASE-CHAOS-003` (DB outage) |
| `mock-line-identity` | `commerce-mocks::line_identity` | 9004 | Issuer of LINE-shaped `id_token`s, including invalid flavours | `CASE-E2E-002` / `CASE-E2E-003` |
| `mock-liff-webview` | `commerce-mocks::liff_webview` | 9005 | The host that runs the Flutter build, parameterized by WebView profile | `CASE-E2E-002` / `CASE-E2E-003` / `CASE-E2E-004` |
| `mock-ledger` | `commerce-mocks::ledger` | 9006 | Order ledger with snapshot / restore. **Not** the real order store. | `CASE-OPS-001` (backup / restore shape) |

## Starting them

```bash
cd D:/DOKIPON

# Pick a port for each (they have defaults but PORT overrides).
PORT=9001 cargo run -p mock-psp
PORT=9002 cargo run -p mock-broker
PORT=9003 cargo run -p mock-postgres
PORT=9004 cargo run -p mock-line-identity
PORT=9005 cargo run -p mock-liff-webview
PORT=9006 cargo run -p mock-ledger
```

Each one prints `… listening on 0.0.0.0:PORT`. They serve on `0.0.0.0` not
`127.0.0.1` so they work from inside emulators (`10.0.2.2` on Android).

## Control endpoints

Every mock exposes the same control surface so a test can drive them
uniformly. The mock's own data is at `/__mock/...`; the cross-cutting
fault-injection lives at `/__control/...`.

```
GET  /healthz
GET  /__control/faults                    # list active faults
POST /__control/faults   { target, ... }  # inject a fault
POST /__control/reset                     # clear all faults
```

Plus mock-specific:

```
mock-psp        POST /v1/psp/authorize /v1/psp/reconcile /v1/psp/refund
                GET  /__mock/charges
                POST /__mock/reset
mock-broker     POST /v1/broker/publish /v1/broker/consume /v1/broker/redeliver
                GET  /__mock/inbox
                POST /__mock/reset
mock-postgres   POST /mockdb/stock /mockdb/reserve /mockdb/settle
                       /mockdb/idempotency /mockdb/cart_item
                GET  /__mock/snapshot
                POST /__mock/reset
mock-line-identity
                GET  /mockline/mint?flavour=…&nonce=…
                GET  /mockline/claims  (POST { idToken })
                POST /mockline/reset
mock-liff-webview
                GET  /__mock/host?profile=line_ios|line_android|…
                GET  /__mock/profiles
mock-ledger     GET  /mockledger/snapshot
                POST /mockledger/restore
                GET  /mockledger/dump
                POST /mockledger/seed
```

## Fault injection shape

`/__control/faults` accepts:

```json
{
  "target": "psp.authorize",
  "mode": "always" | "next_n",
  "count": 3,
  "latencyMs": 250,
  "httpStatus": 503,
  "dropResponse": false
}
```

`dropResponse: true` is the **money-taken-but-answer-lost** scenario for
`mock-psp`: the side effect is recorded, the HTTP answer is never sent. This
is the case the whole `payment_unknown` design exists for.

`mode: "next_n"` is the **transient outage** mode: it fails the next `count`
calls, then behaves normally. This is what proves the system actually
*recovers*, rather than merely failing forever.
