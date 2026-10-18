You are in the DOKIPON repo, git worktree at .worktrees/wt-adminui (branch agent/codex/adminui).

# Task — build the commerce ADMIN console (Dart/Flutter Web app)

The repo has 3 customer-facing Flutter apps (`apps/shop_app`, `apps/shop_web`,
`apps/line_web`) and **no admin console**. An e-commerce system without a back office
is half a system. Your job is the admin frontend.

## Read these first (house style + the design system you must reuse)

- `packages/design_system/` — the shared widget library. **Use it; do not re-style.**
- `packages/api_client/` — the single source of network I/O for all 3 apps
- `packages/domain_models/` — `Money`, `CheckoutStatus`, `ApiProblem`, `StoreChannel`
- `packages/storefront_state/` — controller pattern (ChangeNotifier or equivalent)
- `apps/shop_web/pubspec.yaml` + `apps/shop_web/lib/` — the closest reference app (Flutter Web)
- `packages/domain_models/lib/` — read `CheckoutStatus` carefully; note that
  `paymentUnknown` is a real non-terminal state that the UI must render as
  "needs review", never as a failure and never as success.

## What to build

`apps/admin_web/` — a Flutter Web admin console.

### Screens (minimum viable back office, 4)

1. **Login** — user/password against the admin service, stores the bearer token.
2. **Orders** — the operator's main screen. Table with: order id, state chip, total
   (Money, integer minor units — never a double), created-at. Filters by
   `CheckoutState`. Row action: open detail.
3. **Order detail** — line items, state trail, and the **reconcile action** for
   `payment_unknown` orders (two buttons: "Mark authorized" / "Mark declined").
   The reconcile control must be hidden or disabled for every other state, because the
   server refuses it with 409 and a dead button is a bug.
4. **Catalog** — product list with price and channel eligibility. Read-only in v1;
   say so in the UI rather than showing dead edit buttons.

### Architecture requirements

1. **Do not put network calls in widgets.** Add an `AdminApiClient` to
   `packages/api_client/lib/` (or a clearly separated admin module there) that owns the
   base URL, the bearer token, and the typed admin endpoints. Widgets call controllers
   in `packages/storefront_state/lib/` (or an admin controllers file alongside it).
   This matches the existing 3-layer split: design_system / domain_models / api_client /
   storefront_state.
2. **Base URL from env**, exactly like the existing apps do — find the env var the
   storefront apps read and follow that pattern. Default should work locally.
3. **Money formatting must go through `domain_models.Money`.** No `toDouble()`, no
   manual `/100`.
4. **Every 401 / 403 must be handled explicitly**: 401 → clear token and route back to
   login; 403 → show "insufficient role" rather than a generic error. These are different
   operator experiences.
5. **Loading and error states are part of the deliverable**, not afterthoughts. Every
   screen shows a spinner while fetching and a retry affordance on failure. An operator
   cannot act on a blank table.

### Quality bar

- `dart analyze` clean for the new code.
- Widget tests for the parts where the logic is non-obvious: the reconcile button's
  visibility rule (visible only for `paymentUnknown`), and the 401 vs 403 handling.
  Use the same test style as the existing packages.
- If `flutter` is not on PATH in your environment, still write the tests and say clearly
  in your report that you could not execute them. Do NOT claim you ran what you did not run.

## Constraints

- Do NOT modify `packages/design_system/`, `packages/domain_models/`, or the 3 existing
  apps. If you need a design_system widget that does not exist, add a NEW widget file
  inside `apps/admin_web/lib/` instead of editing the shared package — another agent may
  be working there. Note in your report what is missing from design_system.
- Do NOT touch `contracts/openapi.yaml` — another agent owns the admin contract.
- Do NOT run kubectl / docker / touch any cluster.
- Do NOT run `flutter pub get` against the network if it is unavailable; write the
  pubspec correctly and report that resolution was not executed.

## Definition of done

- `apps/admin_web/` is a complete Flutter Web app: pubspec, 4 screens, api client layer,
  controller layer, widget tests
- `dart analyze` clean (or, if dart is unavailable, say so explicitly)
- Report: the screen list, the layer split you used, the design_system widgets you reused
  vs the ones you had to localise, which tests you ran, and what is missing from
  design_system that a follow-up should add.
