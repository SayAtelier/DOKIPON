//! Standalone binary for the commerce-order service.
//!
//! The service's logic lives in `lib.rs` so that the merged `commerce-edge`
//! deployment — one Vercel function serving every service — can mount the
//! same router alongside the others. This binary is the local and k3s
//! deployment: it binds a port and serves the router on its own.

use commerce_core::idempotency;
use commerce_core::price_authority::PriceAuthority;
use commerce_core::ReservationSource;
use commerce_order::*;
use std::sync::Arc;
use std::sync::RwLock;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    // Where the authoritative prices come from. This is a hard dependency:
    // without it the service cannot price a checkout, and it refuses to
    // rather than falling back to the client's numbers.
    let catalog_url =
        std::env::var("CATALOG_URL").unwrap_or_else(|_| "http://commerce-catalog:8082".to_string());
    let store: SharedOrder = Arc::new(RwLock::new(OrderStore::new(idempotency::now_ms())));
    let prices = Arc::new(PriceAuthority::new(catalog_url.clone()));
    // The orders live in the database when the deployment has one: an order
    // created on one instance has to be visible to the next. A deployment
    // that has a database and cannot reach it does not start — a store that
    // quietly kept its state in memory would look healthy and lose the order
    // anyway.
    let repo: Arc<dyn OrderRepository> = match commerce_core::db::connect_from_env().await {
        Ok(Some(pool)) => {
            tracing::info!("the order service persists its orders in the database");
            Arc::new(DbOrderRepository::new(pool))
        }
        Ok(None) => Arc::new(MemoryOrderRepository::new(store)),
        Err(error) => panic!("the database must be reachable: {error}"),
    };
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind order");
    tracing::info!(
        "order service listening on {}, pricing from {}",
        listener.local_addr().unwrap(),
        catalog_url
    );
    // The ledger. The order service holds the units before it asks for the
    // payment, so a checkout that cannot reserve its stock is refused rather
    // than selling what the warehouse does not have.
    let inventory_url =
        std::env::var("INVENTORY_URL").unwrap_or_else(|_| "http://commerce-inventory:8084".into());
    let reservations: Arc<dyn ReservationSource> = Arc::new(
        commerce_core::inventory_authority::InventoryAuthority::new(inventory_url),
    );
    axum::serve(listener, router(repo, prices, reservations))
        .await
        .expect("serve order");
}
