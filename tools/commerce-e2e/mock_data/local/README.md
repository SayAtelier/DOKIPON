# local/ — the local run's inputs, in one place

The commerce stack needs six credentials and a port map before it can start.
Until this directory existed, those inputs lived in three different places:
the reader's memory, the deployment's environment
(`services/deploy/k3s/00-namespace-configmap.yaml`), and each service's own
seed code. A local run therefore was not reproducible — two runs of "the same
stack" could disagree about the LINE channel id, and every test that used the
LINE mock was testing a fiction.

This directory is that one place.

| file | what it is | who reads it |
|---|---|---|
| `local.env` | the flat `KEY=VALUE` form: the six credentials, the guard switch | a launcher (`docker run --env-file`, `set -a; . local.env; set +a`) |
| `local.json` | the manifest: the same credentials with their variable names and the service that reads them, the port map, and the seeded dataset the services already hold | a machine (the launcher, and the mock↔product agreement test) |
| `check-fixture-set.ps1` | the self-check: starts the mock and the identity service **from this set** and proves the product accepts the mock's token and refuses the four broken flavours | whoever wants to know whether the set still wires up |

## Every value here is a local development value

Not one of these is a credential. They are published on purpose so a local run
is reproducible, and that is exactly why they must never reach a deployment.

The values are deliberately **not** the published development values
(`dev-only-admin`, `dev-only-key`, `dev-channel`, `dev-only-secret`,
`dev-only-service-token`). The guard in
`services/crates/commerce-core/src/secrets.rs` refuses those unless
`DOKIPON_ALLOW_DEV_SECRETS=1` is set; a local run sets that switch, and the
switch is set here. The values themselves are local-only names, so this
directory cannot be mistaken for a deployment's credential set — and so a
deployment that somehow read this file would still be refused by the guard
rather than quietly running with a known password.

**Never copy `local.env` into a deployment.** A deployment fills the k3s
Secret with real values (`kubectl -n commerce create secret generic
commerce-service-credentials --from-literal=...`, see the comment in
`services/deploy/k3s/00-namespace-configmap.yaml`).

## How the mock and the product agree on the credentials

`LINE_ID_TOKEN_KEY` and `LINE_CHANNEL_ID` must reach **both** sides:

- `mock-line-identity` reads them and signs the id tokens it mints with them;
- `commerce-identity` reads them and verifies with them.

A mock that signs with a different key than the service verifies with makes
every test that uses it wrong. The agreement test
(`tools/commerce-e2e/agreement/tests/line_token_agreement.rs`) reads the
values out of `local.json` and uses them on both sides, so the fixture set
cannot drift away from the mock.

`check-fixture-set.ps1` is the same thing over HTTP: it starts
`mock-line-identity` and `commerce-identity` from this set, mints a token at
the mock, and posts it to the product's `/v1/sessions/line`.

```bash
pwsh -File tools/commerce-e2e/mock_data/local/check-fixture-set.ps1
```

## The seeded dataset is not here, on purpose

The catalogue, the inventory ledger and the content document are seeded by the
services themselves — `local.json`'s `seed` section points at exactly where
each seed lives and what a test should expect from it. Copying them into this
directory would create a second truth about what the catalogue holds, and the
second truth is the one that drifts.

The one thing a local run has to supply is `CONTENT_PATH`: the content
document starts empty (revision 0) and is persisted to that path once the
operator writes to it.
