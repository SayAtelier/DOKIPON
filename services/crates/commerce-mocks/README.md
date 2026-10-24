# commerce-mocks — the test fixtures for the unmocked test-design cases

> **This crate is a TEST FIXTURE, not a product dependency.** Nothing in
> `services/` may depend on it, and it must not depend on a product service
> either. It exists so the `[Designed]` cases in
> `docs/ecommerce/05-test/テスト設計書.md` §3 can actually be executed.

Every mock here answers exactly one question from the test design:

| Mock | Binary | Port | Stands in for | Case | Question it answers |
|---|---|---|---|---|---|
| [`psp`](src/psp.rs) | `mock-psp` | 9001 | the payment provider | CASE-CHAOS-001 | what does checkout do when the provider takes the money and loses the answer? |
| [`broker`](src/broker.rs) | `mock-broker` | 9002 | the event broker | CASE-IT-002, CASE-CHAOS-002 | is Outbox→Inbox at-least-once, and does a redelivery duplicate work? |
| [`postgres`](src/postgres.rs) | `mock-postgres` | 9003 | the database | CASE-IT-001, CASE-CHAOS-003 | do the constraints in the DDL actually hold? |
| [`line_identity`](src/line_identity.rs) | `mock-line-identity` | 9004 | LINE's id-token issuer | CASE-E2E-002, CASE-E2E-003 | what does the server do with a real-shaped `id_token` — and with each broken one? |
| [`liff_webview`](src/liff_webview.rs) | `mock-liff-webview` | 9005 | the LINE WebView host | CASE-E2E-002/003/004 | does the client degrade correctly per WebView host? |
| [`ledger`](src/ledger.rs) | `mock-ledger` | 9006 | the order store's export/restore | CASE-OPS-001 | can the event trail be exported and restored? |

`mock-postgres` is **not** PostgreSQL and `mock-ledger` is **not** the order
store: both are constraint-shaped stand-ins. What they prove is the shape of
the rule, not the behaviour of the real system.

## The design rules

1. **Deterministic by default.** No randomness in a response path unless the
   caller explicitly asks for it. A test that cannot be reproduced is not a
   test.
2. **Fault injection is an API, not a config file.** A test drives
   [`fault::FaultController`](src/fault.rs) over HTTP, so a failure can be
   injected *mid-test* without restarting anything.
3. **A mock never gets a secret.** These fixtures hold no production keys.
   `CASE-SEC-002` (no secret in the bundle) is checked by scanning, not by
   trusting this crate.
4. **Honest evidence.** A response records what the mock did, so a test can
   assert on the mock's own behaviour instead of on the SUT's report of it.

The full control surface (`/__control/faults`, `/__control/reset`,
`/__mock/...`) is documented in `tests/mocks/README.md` at the repository
root; this file does not repeat it.

**The issuer's clock is the wall clock.** `mock-line-identity` mints `exp`
against the real time, because the product verifies `exp` against **its own**
clock. An issuer frozen in the past hands out tokens that every running
service refuses with `identity.expired_token` — a token born expired, usable
only by a test that froze the product's clock to the same instant. A test
that wants a frozen clock calls `LineIdentityMock::set_now()`: the
determinism lives in the test's hands, not in the issuer's.

## How the local run uses it

The inputs of a local run live in one place:
`tools/commerce-e2e/mock_data/local/` — `local.env` (the flat form a
launcher sources) and `local.json` (the manifest: the same credentials, the
port map, and the seeded dataset the services already hold). Every value in
there is a **local development value**, and the set says so in the file
itself.

Two of those values matter to a mock:

- `LINE_ID_TOKEN_KEY` — `mock-line-identity` signs with it;
- `LINE_CHANNEL_ID` — `mock-line-identity` mints for it.

The **same** two values must reach `commerce-identity`, which verifies with
them. A mock that signs with a different key than the product verifies with
makes every test that uses it wrong, so the mock↔product agreement test
(`tools/commerce-e2e/agreement/tests/line_token_agreement.rs`) reads both
values out of `local.json` and uses them on each side.

## What is wired, and what is not — honestly

**No mock is called over HTTP by the product today.** Not one. The product
simulates its dependencies in process, so there is no client anywhere in
`services/` that points at a mock:

| Mock | The product's own stand-in for it | Why the mock is not on the wire |
|---|---|---|
| `mock-psp` | `commerce_payment_adapter::PaymentAdapter` with a `ProviderScript` (`services/crates/commerce-payment-adapter/src/lib.rs:44`) | the adapter *is* the provider in a local run: it signs, it declines, it times out, it loses the answer. The mock models the same provider for a test that wants to drive the provider's own failure modes. |
| `mock-broker` | the order service's outbox is an in-memory `Vec<DomainEvent>` (`services/crates/commerce-order/src/lib.rs:139`) | there is no broker client in the product, so there is nothing for the mock to answer. |
| `mock-postgres` | every store is an in-memory `HashMap` behind an `RwLock`; no crate depends on `sqlx` | the product has no database client at all. The mock proves the DDL's *constraints*, which the product's stores enforce by hand. |
| `mock-line-identity` | `commerce_identity::TokenVerifier` verifies locally, with HS256 over `header.payload` | the mock is the **issuer** and the product is the **verifier** — they are two halves of one handshake, and the product never calls out to an issuer. |
| `mock-liff-webview` | nothing | it is a **host** for the Flutter build, not a dependency of any service: the client asks it which WebView it is running in. |
| `mock-ledger` | the order store is in memory | the product exposes no snapshot/restore route, so the mock's snapshot/restore shape has no product side to answer. |

So the mocks are **driven by the test**, not by the product. Two consequences
worth stating plainly:

- **A mock is not a live dependency.** A green run against `mock-psp` says
  the *mock* behaves that way; it says nothing about the PSP the deployment
  will actually talk to. [证据等级: Observed] for the mock's own behaviour,
  [Hypothetical] for the real provider, the real broker, the real database
  and the real LINE platform.
- **The one place a mock's output meets the product today is the agreement
  test.** `mock-line-identity` mints a token and `commerce_identity`
  verifies it, with the local fixture set's credentials on both sides. That
  is what keeps the mock honest: if the two disagree about the token's
  shape, the test fails instead of every test that uses the mock quietly
  testing a fiction.

## Running the mocks

```bash
cd services
PORT=9001 cargo run -p mock-psp
PORT=9002 cargo run -p mock-broker
PORT=9003 cargo run -p mock-postgres
PORT=9004 cargo run -p mock-line-identity
PORT=9005 cargo run -p mock-liff-webview
PORT=9006 cargo run -p mock-ledger
```

Each binds `0.0.0.0` (not `127.0.0.1`) so an Android emulator can reach the
host at `10.0.2.2` — that is what CASE-E2E-004 needs.

## The tests

```bash
cargo test -p commerce-mocks --lib        # the fixtures' own behaviour
cargo test --manifest-path tools/commerce-e2e/agreement/Cargo.toml   # mock ↔ product
```
