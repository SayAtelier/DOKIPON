You are in the DOKIPON repo, git worktree at .worktrees/wt-adminsvc (branch agent/codex/adminsvc).

# Task — build the commerce ADMIN service (Rust/Axum), a 9th service in the workspace

The repo has a complete storefront (3 Flutter apps + 7 Axum services) and **no admin
side at all**: `contracts/openapi.yaml` has 12 paths, every one of them customer-facing.
An e-commerce system without a back office is half a system. Your job is the admin
backend.

## Read these first (they define the house style and the domain vocabulary)

- `services/Cargo.toml` — the workspace. Add your crate to `members`.
- `services/crates/commerce-core/src/order_state.rs` — `CheckoutState` FSM, `MAX_TRANSITIONS`
- `services/crates/commerce-core/src/money.rs` — `Money` / `Currency` integer minor units
- `services/crates/commerce-core/src/error.rs` — `AppError`, `status()`, problem+json shape
- `services/crates/commerce-order/src/main.rs` — the reference service: router shape,
  `Store` behind `Arc<RwLock<..>>`, tracing init, `PORT` env, graceful bind
- `services/crates/commerce-catalog/src/main.rs` — `Sku` / `Product` / `ProductListItem`
- `services/crates/commerce-inventory/src/main.rs` — `StockLevel` / `Reservation`
- `services/crates/commerce-payment-adapter/src/main.rs` — `PaymentAttempt` / `WebhookEvent`
- `services/crates/commerce-bff/src/main.rs` — rate limit + correlation-id middleware pattern
- `contracts/openapi.yaml` — the customer contract you are deliberately NOT changing
- `services/deploy/deploy.sh` — the systemd installer; note the port table

## What to build

`services/crates/commerce-admin/` — a new Axum 0.8 service on **port 8087**.

It is a **facade over the other services over cluster DNS**, not a second source of
truth. Upstream URLs come from env exactly like the BFF does:

```
ORDER_URL            http://commerce-order:8085
CATALOG_URL          http://commerce-catalog:8082
INVENTORY_URL        http://commerce-inventory:8084
PAYMENT_ADAPTER_URL  http://commerce-payment-adapter:8086
IDENTITY_URL         http://commerce-identity:8081
```

### Endpoints (all under `/admin/v1`)

**Auth — every other route requires an admin role.**
- `POST /admin/v1/sessions` — body `{ "user": "...", "password": "..." }`.
  Dev-mode credentials from env `ADMIN_USER` / `ADMIN_PASSWORD` (default `admin` / `dev-only-admin`).
  Returns `200 { "token": "...", "role": "admin"|"operator"|"viewer", "expires_in_ms": ... }`
  or `401` problem+json. Token is an opaque random hex stored in the service's
  `Arc<RwLock<..>>` session map. **Never log the password or the token.**
- `GET /admin/v1/session` — introspect the bearer token, returns role + expiry.

**Authorization** — a middleware that reads `Authorization: Bearer <token>`, resolves
the role, and rejects with `403` when the route's required role outranks the token's.
Roles, lowest to highest: `viewer` < `operator` < `admin`.
Read-only routes need `viewer`; mutating routes need `operator`; session/role routes
need `admin`. Make the required role per-route explicit at the call site.

**Read routes (need `viewer`)**
- `GET /admin/v1/orders?state=<CheckoutState>&limit=&cursor=` — proxy to order service,
  expose the full `CheckoutState` filter. This is the operator's main screen.
- `GET /admin/v1/orders/{order_id}` — proxy order detail.
- `GET /admin/v1/products?limit=&cursor=` — proxy catalog listing.
- `GET /admin/v1/products/{product_id}` — proxy catalog detail.
- `GET /admin/v1/inventory` — proxy inventory stock levels.
- `GET /admin/v1/payments?limit=` — proxy payment attempts.

**Mutating routes (need `operator`)**
- `POST /admin/v1/orders/{order_id}/reconcile` — body `{ "authorized": bool }`.
  Proxies the order service's reconcile endpoint. **This is the admin's recovery tool for
  `payment_unknown` checkouts and it must refuse non-`payment_unknown` orders** — the
  order service already enforces that (returns 409); propagate that status unchanged, do
  not paper over it.
- `POST /admin/v1/inventory/reservations` — reserve stock for an operator-forced order.
- `POST /admin/v1/inventory/reservations/{id}/release` — release a stuck reservation.

### Hard requirements

1. **Every upstream failure must be surfaced, not swallowed.** Map connection errors and
   non-2xx upstream responses into `AppError` / problem+json with the upstream status
   preserved. An admin screen that silently shows stale data is worse than an error.
2. **Money must never become a float.** All amounts crossing the admin boundary stay in
   `Money` (integer minor units + ISO-4217). If you parse or re-emit an amount, use
   `commerce_core::money`.
3. **`payment_unknown` stays non-terminal.** Never auto-retry, never auto-reconcile on
   the customer's behalf. The admin explicitly chooses.
4. **No new dependencies beyond what the other services already use.** The workspace
   dependency table is in `services/Cargo.toml`; `reqwest` is NOT in it. If you need an
   HTTP client, add it to `[workspace.dependencies]` in a way consistent with the existing
   style (note: `sqlx` is already there with `runtime-tokio-rustls`; pick the smallest
   addition and say why in your report).
5. Tests: the other crates have `#[cfg(test)]` unit tests inline and integration tests
   under `tests/`. Write real tests that can FAIL — at minimum auth/role matrix, upstream
   error propagation, and the reconcile-on-non-payment_unknown refusal. A test that passes
   against a deliberately broken implementation is worse than no test.
6. Update `services/Cargo.toml` members, and update `services/deploy/deploy.sh` port table
   + SERVICES list so the new service is installable there too (port 8087).

## Constraints

- Do NOT touch `contracts/openapi.yaml` — another agent owns the admin contract file.
- Do NOT touch the other 8 crates except `services/Cargo.toml` and
  `services/deploy/deploy.sh`.
- Do NOT run kubectl, docker, or touch any cluster.
- Do NOT try to build the whole workspace if it takes too long: `cargo check -p commerce-admin`
  and `cargo test -p commerce-admin` are the acceptance gate. If the workspace has
  pre-existing failures on other crates, report them, do not fix them.
- Code comments in this repo are English. Doc comments are `//!` / `///` and explain WHY,
  referencing the atlas section when relevant (e.g. `per atlas §B1`).

## Definition of done

- `cargo check -p commerce-admin` clean
- `cargo test -p commerce-admin` passes, and you can name a mutation that makes it fail
- `services/Cargo.toml` and `services/deploy/deploy.sh` updated
- Report: endpoint table, the role matrix, the dependency you added and why, the tests
  you wrote and which mutation kills each, and anything you deliberately left out.
