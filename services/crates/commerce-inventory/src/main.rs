//! Standalone binary for the commerce-inventory service.
//!
//! The service's logic lives in `lib.rs` so that the merged `commerce-edge`
//! deployment — one Vercel function serving every service — can mount the
//! same router alongside the others. This binary is the local and k3s
//! deployment: it binds a port and serves the router on its own.

use commerce_inventory::*;
use std::sync::Arc;
use std::sync::RwLock;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let store: SharedInventory = Arc::new(RwLock::new(InventoryStore::new(
        // These SKUs must match commerce-catalog's seed: the checkout path
        // looks stock up by sku, and a sku that exists in one service but not
        // the other makes a purchasable product fail at the last step.
        DEMO_STOCK_SEED,
    )));
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind inventory");
    tracing::info!(
        "inventory service listening on {}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, router(store))
        .await
        .expect("serve inventory");
}
