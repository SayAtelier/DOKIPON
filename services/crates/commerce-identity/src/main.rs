//! Standalone binary for the commerce-identity service.
//!
//! The service's logic lives in `lib.rs` so that the merged `commerce-edge`
//! deployment — one Vercel function serving every service — can mount the
//! same router alongside the others. This binary is the local and k3s
//! deployment: it binds a port and serves the router on its own.

use commerce_core;
use commerce_core::idempotency;
use commerce_identity::*;
use std::sync::Arc;
use std::sync::RwLock;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    // The signing key and the channel id are credentials, not configuration:
    // a deployment that keeps the development values accepts an id token
    // signed by anyone. The service refuses to start without real ones.
    let expected_audience = commerce_core::secrets::require(
        "LINE_CHANNEL_ID",
        commerce_core::secrets::DEV_LINE_CHANNEL_ID,
    )
    .expect("LINE_CHANNEL_ID must be set to a real channel id");
    let signing_key = commerce_core::secrets::require(
        "LINE_ID_TOKEN_KEY",
        commerce_core::secrets::DEV_LINE_ID_TOKEN_KEY,
    )
    .expect("LINE_ID_TOKEN_KEY must be set to a real signing key");
    let verifier = Arc::new(TokenVerifier {
        expected_audience,
        expected_issuer: "https://access.line.me".to_string(),
        signing_key,
    });
    let store: SharedIdentity = Arc::new(RwLock::new(IdentityStore::new(
        commerce_core::idempotency::now_ms(),
    )));
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind identity");
    tracing::info!(
        "identity service listening on {}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, router(store, verifier))
        .await
        .expect("serve identity");
}
