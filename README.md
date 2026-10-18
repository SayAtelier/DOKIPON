# DOKIPON

Multi-product repository. Two parallel product lines live here:

| Product | Surface | Status |
|---|---|---|
| **GemArc Commerce** | Rust micro-services + Flutter storefront (App / Web / LINE MINI App) and proposed back-office API / console | `v8.3.x`, in active development |
| **RGS — Rust Game Server** | gRPC game-server suite (5 domain services + gm-backend) | production (`0.1.0+`) |

This README covers the repository shape and the entry points. The
`AGENTS.md` and `CLAUDE.md` files at the root document agent / coding
conventions shared by both product lines.

## Quick map

```
apps/                   — Flutter (3 storefront apps + proposed admin console)
  ├ shop_app/           — Android / iOS (storefront)
  ├ shop_web/           — Flutter Web (storefront)
  ├ line_web/           — LINE MINI App (LIFF-bridged storefront)
  └ admin_console/      — proposed Flutter back-office console

packages/               — shared Dart packages for storefront clients
  ├ domain_models/      — Money, CheckoutStatus, ApiProblem, StoreChannel
  ├ api_client/         — REST + LIFF bridge (single source of network I/O)
  ├ storefront_state/   — Catalog/Checkout/Session controllers
  └ design_system/      — shared widgets

services/               — Rust workspace (Axum 0.8 micro-services)
  ├ Cargo.toml          — workspace 2021, 8 current production crates + commerce-mocks (9 with proposed admin crate)
  ├ crates/
  │  ├ commerce-core/             — FSM, Money, Idempotency, AppError
  │  ├ commerce-bff/              — rate-limit + path rewrites
  │  ├ commerce-identity/         — sessions, OAuth
  │  ├ commerce-catalog/          — SKU + authoritative pricing
  │  ├ commerce-cart/             — optimistic concurrency + tamper defence
  │  ├ commerce-inventory/        — stock + reservation TTL
  │  ├ commerce-order/            — checkout state machine
  │  ├ commerce-payment-adapter/  — PSP sandbox
  │  ├ commerce-admin/            — proposed back-office API service
  │  └ commerce-mocks/            — see lib.rs STATUS banner; excluded from workspace
  └ deploy/deploy.sh    — install / uninstall / restart on the Multipass VM

contracts/openapi.yaml  — OpenAPI 3.1, 23 paths, 24 schemas, storefront + proposed admin contract

tools/e2e_checkout.sh   — 22-assertion end-to-end HTTP check
tests/                  — integration scaffolding (see tests/README.md)

docs/ecommerce/         — design documents (Japanese, IPA-style)
docs/gemarc-5d-design-atlas/ — upstream design source-of-truth (v8.2.0)
```

## Working agreements

- **Money is integer minor units + ISO-4217 currency.** `double` may not hold
  business money anywhere. The type is in `commerce-core::money` (Rust) and
  `domain_models.Money` (Dart).
- **`payment_unknown` is a real state.** Clients poll. They do not resubmit.
  The FSM is in `commerce-core::order_state`; the client rule is in
  `storefront_state::CheckoutController`.
- **The server is the only source of truth on price.** Client subtotals
  display; settlement re-computes from catalog prices.
- **Idempotency-Key is mandatory on storefront business writes and admin
  inventory writes.** Scope is `(actor, operation)`. Same key + same payload
  replays; same key + different payload returns 409. Admin login and operator
  `payment_unknown` reconciliation are explicit exceptions.
- **Errors are `application/problem+json`** with extensions `{code, traceId,
  retryable}`. `retryable=true` on transient states only.
- **LIFF channel secret and any PSP credential never live in source.** The
  LIFF bridge (`apps/line_web/web/liff_bridge.js`) is the only place that
  touches the SDK; failures / external browser / signed-out all degrade to
  `GuestSession`. No login loop.
- **The mocks are scaffolding that does not run.** See the STATUS banner in
  `services/crates/commerce-mocks/src/lib.rs`. Migrating to
  `tokio::sync::RwLock` will move them back into the workspace.

## Local three-suite

```
cargo test --workspace                                   99 passed, 0 failed
( cd packages/domain_models && flutter test )            24 passed, 0 failed
( cd services && cargo run -p commerce-order ) && \
    bash tools/e2e_checkout.sh                           22 passed, 0 failed
```

CI runs the same three on every push. See
`.github/workflows/commerce-ci.yml`.

## License

Commercial — see [`COMMERCIAL-LICENSE.md`](./COMMERCIAL-LICENSE.md).
