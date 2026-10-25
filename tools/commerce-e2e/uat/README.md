# uat/ — the user acceptance test

> A UAT asks: **can a person do the things a person has to do?**
> The audit (`../scripts/audit.mjs`) asks a different question: **does every
> control do something?**

Both are needed, and they do not overlap. The audit reloads the page before
each control and clicks it on its own, which is the right way to find a dead
button and the wrong way to find out whether a shop works. A UAT drives the
journeys — browse, filter, open a product, fill the cart, pay, and have the
order turn up on the operator's screen — and checks the **outcome**.

## Running it

```bash
cd tools/commerce-e2e
npm run uat                       # against the live deployment
DOKIPON_BASE=https://... npm run uat
npm run local:uat                 # against the local stack (the Vercel topology)
```

| variable | meaning |
|---|---|
| `DOKIPON_BASE` | the deployment under test. The live one by default. |
| `ADMIN_USER` / `ADMIN_PASSWORD` | the operator credential. The credentials file the deploy step wrote is used when they are not set. |
| `DOKIPON_LOCAL_BASE_PORT` | where the local stack's **mock** is. The local stack publishes the mock's port, and the UAT finds it from here. |

The report is written to `../reports/uat-<timestamp>.md`.

## The three answers

| answer | what it means |
|---|---|
| `PASS` | the journey ran and every check held. |
| `FAIL` | a check did not hold, or the journey threw. A finding. The report says which check, and with what evidence. |
| `UNMEASURED` | the journey could not be asked. **Not** a finding, and **not** a pass either. |

The usual reason for `UNMEASURED` is the mock: the LINE journey signs in
with a token the **mock** mints, and a deployment whose `LINE_ID_TOKEN_KEY`
is a platform secret cannot be given one — the key cannot be read back, so
there is nothing a test can sign with. The journey says so instead of
passing quietly.

## The journeys

| journey | what it is | needs |
|---|---|---|
| `guest-browse` | a guest browses the catalogue, narrows it with the category strip, opens a product, and fills the cart | — |
| `stock-states` | what the catalogue says about stock, and whether the buttons agree | — |
| `payment-refused` | a refused payment does not become a confirmed order | — |
| `line-purchase` | a LINE user signs in, buys a product, and the order exists | the mock |
| `operator-view` | the operator sees the order, its price and its trail | the credential |

They run in that order, and the order matters: the LINE journey places an
order, and the operator journey is the other side of that same order.

## The mock, and why the mock is here

The product never calls an id-token issuer: it **verifies** tokens itself.
`mock-line-identity` is the other half of that handshake — it **issues**
them, with the same `LINE_ID_TOKEN_KEY` and for the same `LINE_CHANNEL_ID`
the identity service verifies with. A token the mock mints is therefore a
token the deployment accepts, and that is the mock↔product agreement
exercised live, over HTTP, through a browser.

The local stack runs the issuer as a process of its own
(`local/supervise.sh`), on `base port + 12`. The UAT finds it through
`MOCK_PROCESSES` in `../local/config.mjs`, so there is one address and not
two.

**"The mock answers" is not enough — the target has to accept its token.**
The mock on the machine signs with the local stack's key, and a deployment
whose key is a platform secret refuses every token it mints. So the UAT
proves the handshake before it claims the journey can run: the mock mints,
the target exchanges the token for a session, and only then is the LINE
journey attempted. A target that refuses the token gets that journey marked
`UNMEASURED` — the question was not askable — rather than a failed one.

The bridge (`window.__gemarcLiff`) is how the app learns there is a LINE
session at all. It is the one place the LIFF SDK is touched: the token is
read once, exchanged for a server session, and discarded. The harness
publishes the bridge before the app boots, with a token the mock just
minted — which is what the LIFF SDK would have done inside the LINE app.

## The expectations live in the fixture set

The journeys read `../mock_data/local/local.json` → `uat`, never a literal.
The catalogue's contents are listed there product by product, because a
journey has to name a product, its price and its stock state — and a price
that quietly changes must not quietly change what the UAT expects too. A
fixture that disagrees with the catalogue is a finding in itself.

## What a journey does not assert

A journey asserts what the product **does**, and reports what the product
**says about itself** when the two disagree. It does not pick a side.

The clearest case is the guest checkout. The account sheet tells a guest
that buying needs a sign-in, `Session.canCheckout` is false for a guest,
and `docs/ecommerce/04-detailed/セキュリティ設計.md` SEC-17 says the server
must refuse a guest checkout too. The order service takes any
`x-session-id` it is given, and answers `201`. The `guest-browse` journey
therefore stops at the cart: it asserts the journey a guest can actually
take, and it says in a note that the rule is stated in three places and
enforced in none.
