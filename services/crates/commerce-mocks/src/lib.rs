//! Deterministic mocks for the test-design cases that could not otherwise be
//! executed.
//!
//! Every mock here exists to answer one specific question from
//! `docs/ecommerce/05-test/テスト設計書.md` §3:
//!
//! | Mock | Case | Question it answers |
//! |---|---|---|
//! | [`psp`] | CASE-CHAOS-001 | What does checkout do when the payment provider is down? |
//! | [`broker`] | CASE-IT-002 | Is Outbox→Inbox at-least-once, and does replay duplicate work? |
//! | [`postgres`] | CASE-IT-001 | Do the constraints in the DDL actually hold? |
//! | [`line_identity`] | CASE-E2E-002/003 | What does the server do with a real-shaped LINE id_token? |
//! | [`liff_webview`] | CASE-E2E-002/003/004 | Does the client degrade correctly per WebView host? |
//! | [`ledger`] | CASE-OPS-001 | Can the event trail be exported and restored? |
//!
//! # Design rules
//!
//! 1. **Deterministic by default.** No randomness in a response path unless the
//!    caller explicitly asks for it. A test that cannot be reproduced is not a
//!    test.
//! 2. **Fault injection is an API, not a config file.** Tests drive
//!    [`fault::FaultController`] over HTTP so a failure can be injected *mid
//!    test* without restarting anything.
//! 3. **A mock never gets a secret.** These fixtures hold no production keys;
//!    `CASE-SEC-002` (no secret in the bundle) is checked by scanning, not by
//!    trusting this crate.
//! 4. **Honest evidence.** Responses record what the mock did, so a test can
//!    assert on the mock's own behaviour rather than on the SUT's report of it.

pub mod broker;
pub mod fault;
pub mod ledger;
pub mod line_identity;
pub mod liff_webview;
pub mod postgres;
pub mod psp;

pub use fault::{Fault, FaultController, FaultMode};

/// Shared helper: every mock binds 0.0.0.0 on `$PORT` (default `fallback`).
///
/// Binding to `0.0.0.0` rather than `127.0.0.1` matters for
/// `CASE-E2E-004`: an Android emulator reaches the host at `10.0.2.2`.
pub async fn serve_forever(router: axum::Router, fallback_port: u16, label: &'static str) {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(fallback_port);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| panic!("{label}: bind {addr}: {e}"));
    tracing::info!("{label} listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router)
        .await
        .unwrap_or_else(|e| panic!("{label}: serve: {e}"));
}

/// Standard control routes mounted by every mock so tests drive them uniformly.
pub fn control_router(controller: FaultController) -> axum::Router {
    use axum::{routing::get, Json};
    // Each route closure needs its own clone; `controller` itself stays owned
    // by this function until the Router is built.
    let list = controller.clone();
    let inject = controller.clone();
    let clear = controller;
    axum::Router::new()
        .route("/__control/health", get(|| async { "ok" }))
        .route(
            "/__control/faults",
            get(move || {
                let c = list.clone();
                async move { Json(c.list()) }
            }),
        )
        .route(
            "/__control/faults",
            axum::routing::post(move |Json(f): Json<Fault>| {
                let c = inject.clone();
                async move {
                    c.inject(f);
                    Json(c.list())
                }
            }),
        )
        .route(
            "/__control/reset",
            axum::routing::post(move || {
                let c = clear.clone();
                async move {
                    c.clear();
                    Json(c.list())
                }
            }),
        )
}
