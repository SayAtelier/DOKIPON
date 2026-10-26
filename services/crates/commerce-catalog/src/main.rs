//! Standalone binary for the commerce-catalog service.
//!
//! The service's logic lives in `lib.rs` so that the merged `commerce-edge`
//! deployment — one Vercel function serving every service — can mount the
//! same router alongside the others. This binary is the local and k3s
//! deployment: it binds a port and serves the router on its own.

use commerce_catalog::*;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let store: SharedCatalog = Arc::new(CatalogStore::seed_demo());
    // The ledger. The catalogue asks it how much of each SKU is left, so the
    // storefront is told what the ledger says and not what the catalogue
    // copied of it a while ago.
    let inventory_url =
        std::env::var("INVENTORY_URL").unwrap_or_else(|_| "http://commerce-inventory:8084".into());
    let stocks: Arc<dyn commerce_core::StockSource> = Arc::new(
        commerce_core::stock_source::StockAuthority::new(inventory_url),
    );
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind catalog");
    tracing::info!(
        "catalog service listening on {}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, router(store, stocks))
        .await
        .expect("serve catalog");
}
