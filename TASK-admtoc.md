You are in the DOKIPON repo, git worktree at .worktrees/wt-admtoc (branch agent/codex/admtoc).

# Task — write the ADMIN API contract, and update the docs that describe the product surface

The repo's product documentation is now wrong in a specific way: it describes a
storefront-only system, and the repo is gaining a back office. This task owns the
contract file and every doc that enumerates the system's surfaces.

## Part 1 — the admin contract

Extend `contracts/openapi.yaml` (OpenAPI 3.1) with the admin surface. The existing 12
paths are the customer contract and **must not be modified** — only additions.

Read `contracts/openapi.yaml` first for its exact conventions: version line, `info`
block, `servers`, `tags`, `components.securitySchemes`, `components.schemas`, the
problem+json error responses, and the idempotency-key convention. Match them.

Add a `securitySchemes` entry for bearer auth if one does not exist, and a `/admin/v1`
path group with:

- `POST /admin/v1/sessions` — login, returns bearer token + role
- `GET  /admin/v1/session` — introspect current token
- `GET  /admin/v1/orders` — list with `state` / `limit` / `cursor` filters
- `GET  /admin/v1/orders/{orderId}` — detail
- `POST /admin/v1/orders/{orderId}/reconcile` — operator recovery for `payment_unknown`
- `GET  /admin/v1/products` — catalog listing for the back office
- `GET  /admin/v1/products/{productId}` — catalog detail
- `GET  /admin/v1/inventory` — stock levels
- `POST /admin/v1/inventory/reservations` — operator-forced reserve
- `POST /admin/v1/inventory/reservations/{reservationId}/release` — release
- `GET  /admin/v1/payments` — payment attempts

Rules the contract must express, because they are the whole point of the back office:
- The three roles `viewer` < `operator` < `admin` and which operations each unlocks.
  Express this in the descriptions AND in a reusable `security` + description pattern.
- `reconcile` returns **409** when the order is not in `payment_unknown`, and **403**
  when the token lacks `operator`. Both must appear in the responses block.
- `reconcile` is **not** idempotent-key protected (it is not a customer write; it is an
  operator recovery action) — but say so explicitly in the description so nobody
  "fixes" it later.
- Money fields are **integer minor units + currency**, never float. Reuse the existing
  Money schema if the contract already has one.
- Every error response is the existing problem+json schema. Reuse, do not invent.

Validate the result: if python + `yaml` are available, parse it and assert the path count.
If `openapi-spec-validator` or similar is available, use it. Report what you ran.

## Part 2 — the documentation that is now stale

These files enumerate the product surface and are wrong once an admin side exists.
Update them. Do NOT rewrite them wholesale; make the minimal accurate change.

- `README.md` — the "Quick map" tree lists `services/` crates and `apps/`. Add the
  admin service and admin console in the same style and format. Update the product table
  at the top only if the admin surface changes what the product *is*.
- `CHANGELOG.md` — add an `## [unreleased]` → `### Added` entry for the admin backend,
  the admin console, and the admin contract, following the existing entry style.
- `CLAUDE.md` and `AGENTS.md` — check whether they enumerate crates, apps, or ports.
  If they do, update those enumerations. If they do not, leave them alone and say so.
  **Do not restructure these two files.**
- `services/deploy/deploy.sh` header comment mentions the service count ("all 7
  commerce services") — if the count changes, update the comment.

Search for every place that hardcodes "7" next to "service", "12" next to "path", or
similar counts, and fix them. Report the list you found.

## Constraints

- Do NOT add or modify any Rust source. The admin service crate is owned by another agent.
- Do NOT add or modify any Dart source. The admin console is owned by another agent.
- Do NOT touch `docs/deploy/**` — the deployment integration is owned by another agent.
- Do NOT invent design claims. If you cannot verify a statement from the code or the
  contract, write it as a proposal, not as a fact.

## Definition of done

- `contracts/openapi.yaml` parses, existing 12 paths byte-identical, admin paths added
- README / CHANGELOG / CLAUDE.md / AGENTS.md / deploy.sh comments updated where stale
- Report: the admin path table you added, every stale count you found and fixed, every
  file you deliberately left alone and why.
