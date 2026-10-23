# Changelog

All notable changes to GemArc Commerce live here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html) for
the commerce module only (the RGS game server follows its own cadence).

## [unreleased]

### Added
- Proposed GemArc Commerce back-office surface:
  - `services/crates/commerce-admin/` — Rust admin API service.
  - `apps/admin_console/` — Flutter admin console.
  - `contracts/openapi.yaml` — 11 `/admin/v1` operations with bearer auth,
    viewer/operator/admin role requirements, shared problem+json errors, and
    integer-minor-unit money fields. The unspecified implementation payload
    shapes are marked as proposals in the contract.
- `commerce-ci.yml` — GitHub Actions workflow that runs the three
  local-suite commands on every push to `main` for changes under
  `apps/`, `packages/`, `services/`, `contracts/`, `tools/e2e_checkout.sh`,
  `tests/`, or this workflow file. Path-filtered so it does not fire on
  RGS-only changes.
- `tests/` integration scaffolding:
  - `tests/README.md` — map from `docs/ecommerce/05-test/テスト設計書.md`
    §3 [Designed] cases to the bash / dart / rust pieces that drive them
  - `tests/mocks/README.md` — the 6 commerce mocks (PSP / broker /
    Postgres / line-identity / liff-webview / ledger), the `--mocks-up`
    flag in `tests/integration/e2e_checkout.sh`
  - `tests/integration/e2e_checkout.sh` — extended copy with a
    `--mocks-up` switch
- `services/deploy/deploy.sh` — install / uninstall / restart / status
  / logs for the 7 storefront commerce services managed by this script on
  the Multipass VM
- `services/crates/commerce-mocks/src/lib.rs` STATUS banner that
  documents the workspace exclusion

### Changed
- `docs/ecommerce/05-test/テスト設計書.md` §3 now names which
  [Designed] cases the `tests/` scaffolding can drive (IT-002,
  CHAOS-001..003, OPS-001) and which still need real infrastructure.
- Workspace member `services/crates/commerce-mocks` is commented out
  in `services/Cargo.toml`. The crate depends on `std::sync::RwLock`,
  which is not `Send` across await points; axum 0.8's `Handler` bound
  fails to apply. Migrating to `tokio::sync::RwLock` is the
  documented fix; until then the crate is built explicitly:
  `cd services && cargo test -p commerce-mocks`.

### Removed
- `PLAN-update-v8-2.md` — process artifact, contained absolute working
  directory paths. `PLAN-update-v8-2-scan.md` is kept (reproducible
  scan results).

## [8.2.0] — 2026-10-17

Initial extracted implementation. See `docs/ecommerce/README.md` for
the mapping from `docs/gemarc-5d-design-atlas` v8.2 design artifacts
to source code. The full file inventory is captured in the
`f3625c2 8.2版文档更新` commit and the three following commits
(`1756173 前端开发`, `9296765 电商版本设计资料`).
