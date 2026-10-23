//! Standalone binary for the commerce-identity service.
//!
//! The service's logic lives in `lib.rs` so that the merged `commerce-edge`
//! deployment — one Vercel function serving every service — can mount the
//! same router alongside the others. This binary is the local and k3s
//! deployment: it binds a port and serves the router on its own.

use commerce_identity::*;
use commerce_core;
use commerce_core::idempotency;
use std::sync::Arc;
use std::sync::RwLock;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let verifier = Arc::new(TokenVerifier {
        expected_audience: std::env::var("LINE_CHANNEL_ID").unwrap_or_else(|_| "dev-channel".into()),
        expected_issuer: "https://access.line.me".to_string(),
        signing_key: std::env::var("LINE_ID_TOKEN_KEY").unwrap_or_else(|_| "dev-only-key".into()),
    });
    let store: SharedIdentity = Arc::new(RwLock::new(IdentityStore::new(commerce_core::idempotency::now_ms())));
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind identity");
    tracing::info!("identity service listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router(store, verifier)).await.expect("serve identity");
}
