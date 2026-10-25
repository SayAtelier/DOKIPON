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
    // The ledger lives in the database when the deployment has one: the stock
    // levels have to be the same on every instance, and a reservation taken
    // on one instance has to be visible to the next. A deployment that has a
    // database and cannot reach it does not start — a store that quietly kept
    // its state in memory would look healthy and lose the reservation anyway.
    let repo: Arc<dyn InventoryRepository> = match commerce_core::db::connect_from_env().await {
        Ok(Some(pool)) => {
            tracing::info!("the inventory service persists its ledger in the database");
            let repo = DbInventoryRepository::new(pool);
            repo.seed(DEMO_STOCK_SEED)
                .await
                .expect("the stock levels must be seeded");
            Arc::new(repo)
        }
        Ok(None) => Arc::new(MemoryInventoryRepository::new(store)),
        Err(error) => panic!("the database must be reachable: {error}"),
    };
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind inventory");
    tracing::info!(
        "inventory service listening on {}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, router(repo))
        .await
        .expect("serve inventory");
}
