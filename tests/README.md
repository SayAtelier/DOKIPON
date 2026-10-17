# tests/ — integration test scaffolding

> Scaffolding only. Not run in CI. The v8.2 atlas explicitly says
> **DEGRADED / NOT FULLY ACCEPTED** (`docs/gemarc-5d-design-atlas/QA_FINAL.md`).
> Anything here is **best-effort, not production**.

## What's here

| Path | Role |
|---|---|
| `mocks/README.md` | Documents the 6 Rust mocks at `services/crates/commerce-mocks/`. How to start each one, what control endpoints exist. |
| `integration/e2e_checkout.sh` | A copy of `tools/e2e_checkout.sh` extended with a `--mocks-up` flag that auto-starts the mocks before running the assertions. |

## What's NOT here

- **Unit tests.** Those live with the code: `cargo test -p commerce-X` for the Rust services, `flutter test` in each Dart package. Run those first; the integration layer assumes they pass.
- **Production E2E against a real PSP / DB / LINE.** The v8.2 design acknowledges that the only env that exists is the scripted-mock env. Real wiring would require a Postgres instance, a LINE business account, a real payment provider's sandbox, and a deployable TLS endpoint — none of which this scaffolding can fabricate.
- **CI wiring.** A future PR can call `bash tests/integration/e2e_checkout.sh --mocks-up` from a job that builds `services/` first, but only after someone has actually run it locally and signed off.

## How this maps to the test design book

`docs/ecommerce/05-test/テスト設計書.md` §3 lists the cases that are `[Designed]`
and not yet executed. They are:

- `CASE-E2E-001`..`004` — three-terminal real-device checkout
- `CASE-IT-001`..`003` — Postgres DDL, Outbox→Inbox redelivery, consumer-driven contract tests
- `CASE-CHAOS-001`..`003` — PSP / broker / DB outages
- `CASE-SEC-001`..`002` — SAST/DAST and "no secret in the LINE bundle"
- `CASE-OPS-001` — backup/restore drill (RPO 24h / RTO 4h)

This directory's scaffolding can drive `CASE-IT-002`, `CASE-CHAOS-001..003`, and
`CASE-OPS-001` once the mocks are running. The rest need real infrastructure
that this repo does not have. The mocks are **not** substitutes for the
missing infrastructure — they are placeholders showing the *shape* of what
the eventual test would look like.
