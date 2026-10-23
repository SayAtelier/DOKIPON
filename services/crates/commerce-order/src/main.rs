//! Standalone binary for the commerce-order service.
//!
//! The service's logic lives in `lib.rs` so that the merged `commerce-edge`
//! deployment — one Vercel function serving every service — can mount the
//! same router alongside the others. This binary is the local and k3s
//! deployment: it binds a port and serves the router on its own.

use commerce_order::*;
use commerce_core::idempotency;
use commerce_core::price_authority::PriceAuthority;
use std::sync::Arc;
use std::sync::RwLock;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    // Where the authoritative prices come from. This is a hard dependency:
    // without it the service cannot price a checkout, and it refuses to
    // rather than falling back to the client's numbers.
    let catalog_url = std::env::var("CATALOG_URL")
        .unwrap_or_else(|_| "http://commerce-catalog:8082".to_string());
    let store: SharedOrder = Arc::new(RwLock::new(OrderStore::new(idempotency::now_ms())));
    let prices = Arc::new(PriceAuthority::new(catalog_url.clone()));
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind order");
    tracing::info!(
        "order service listening on {}, pricing from {}",
        listener.local_addr().unwrap(),
        catalog_url
    );
    axum::serve(listener, router(store, prices)).await.expect("serve order");
}
