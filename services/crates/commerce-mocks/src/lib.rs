//! > **STATUS (2026-10-23)**: this crate **builds, tests, and runs**. It is a
//! > member of the `services/` workspace and every `bin/` entry point serves
//! > HTTP, so a test can start a mock and hit it over the wire.
//! >
//! > It is a **TEST FIXTURE, not a product dependency**: nothing in
//! > `services/` may depend on it, and it must not depend on a product
//! > service either. It exists so the `[Designed]` cases in
//! > `docs/ecommerce/05-test/テスト設計書.md` §3 can actually be executed.
//!
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
//!    caller explicitly asks for it, and no *hidden* clock. A mock that
//!    issues a token against a frozen past hands out something every running
//!    service refuses — the token is born expired — so a mock that has to
//!    read the time reads the real time, and the test that wants a frozen
//!    one moves it. The determinism lives in the test's hands, not in the
//!    issuer's. A test that cannot be reproduced is not a test.
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
pub mod liff_webview;
pub mod line_identity;
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    /// The mock has to answer over its real router, not only when a test
    /// calls it as a Rust method. A mock nobody can reach over HTTP is not a
    /// mock, so this drives the same router the `bin/` entry points serve —
    /// no port, no bind, no flakiness.
    #[tokio::test]
    async fn the_psp_router_answers_an_authorize_request_over_http() {
        let mock = psp::PspMock::new();
        let app = control_router(mock.faults()).merge(psp::router(mock));

        let request = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/psp/authorize")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"order_id":"ord-1","payment_method_ref":"tok_mock","amount_minor":4800,"currency":"JPY"}"#,
            ))
            .expect("request builds");

        let response = app.oneshot(request).await.expect("the router answers");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("body reads")
            .to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("body is json");

        assert_eq!(body["status"], "authorized");
        assert!(
            body["providerRef"]
                .as_str()
                .expect("providerRef is a string")
                .starts_with("pspref_"),
            "the mock mints its own provider reference"
        );
    }

    /// CASE-CHAOS-001's whole capability: a test must be able to make the
    /// dependency fail *mid-scenario*, over HTTP, with no restart.
    #[tokio::test]
    async fn the_control_router_injects_a_fault_over_http() {
        let mock = psp::PspMock::new();
        let app = control_router(mock.faults()).merge(psp::router(mock));

        let request = axum::http::Request::builder()
            .method("POST")
            .uri("/__control/faults")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"target":"psp.authorize","mode":"next_n","count":1,"http_status":503}"#,
            ))
            .expect("request builds");

        let response = app.oneshot(request).await.expect("the router answers");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("body reads")
            .to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("body is json");

        assert_eq!(body[0]["target"], "psp.authorize");
        assert_eq!(body[0]["remaining"], 1);
    }
}
