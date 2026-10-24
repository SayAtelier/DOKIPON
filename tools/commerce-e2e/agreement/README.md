# agreement/ — the mock ↔ product agreement test

The mock issues LINE-shaped `id_token`s; the identity service verifies them.
If the two disagree about the token's shape, **every test that uses the mock
is testing a fiction** — the mock would be minting tokens the product would
never accept, and a green test would prove nothing about the product.

This crate is where that disagreement is caught. It mints with
`commerce-mocks::line_identity` and verifies with
`commerce_identity::TokenVerifier`, and it uses the credentials from the
local fixture set (`../mock_data/local/local.json`) on **both** sides — so the
fixture set cannot drift away from the mock.

## Why it is here and not in the mock crate

The mock crate is a test fixture: nothing in `services/` may depend on it,
and it must not depend on a product service either. An agreement test needs
both sides, so it cannot live in either crate. This crate is outside
`services/` and outside the mock crate, and reaches both by path.

## Running it

```bash
cargo test --manifest-path tools/commerce-e2e/agreement/Cargo.toml
```

It is a standalone cargo package (its own workspace root), so it is built and
run separately from the `services/` workspace:

```bash
docker run --rm -v "D:/dokipon-wt/mock-svc:/work" -v dokipon-cargo-home:/cargo \
  -e CARGO_HOME=/cargo -e CARGO_TERM_COLOR=never \
  -w /work/tools/commerce-e2e/agreement rust:1.98.1-slim-bookworm \
  cargo test --message-format short
```

## What it asserts

| the mock issues | the product must answer |
|---|---|
| a well-formed token | accepts it, and the claims come back |
| the token the mock **serves over HTTP** | accepts it too |
| a token signed with another key | `identity.bad_signature` |
| a token whose `exp` is in the past | `identity.expired_token` |
| a token for another channel | `identity.bad_audience` |
| a token from another issuer | `identity.bad_issuer` |
| a token whose nonce is not the one presented | `identity.nonce_mismatch` |

The problem codes are the **product's own** — the ones `commerce_identity`
returns — not strings this test invented.
