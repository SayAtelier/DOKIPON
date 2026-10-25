//! Standalone binary for the commerce-cart service.
//!
//! The service's logic lives in `lib.rs` so that the merged `commerce-edge`
//! deployment — one Vercel function serving every service — can mount the
//! same router alongside the others. This binary is the local and k3s
//! deployment: it binds a port and serves the router on its own.

use commerce_cart::*;
use commerce_core::price_authority::PriceAuthority;
use std::sync::Arc;
use std::sync::RwLock;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    // Where the authoritative prices come from. Without it the cart cannot
    // price a line and refuses the mutation rather than trusting the client.
    let catalog_url =
        std::env::var("CATALOG_URL").unwrap_or_else(|_| "http://commerce-catalog:8082".to_string());
    let store: SharedCart = Arc::new(RwLock::new(CartStore::new()));
    let prices = Arc::new(PriceAuthority::new(catalog_url.clone()));
    // The carts live in the database when the deployment has one: a cart
    // created on one instance has to be visible to the next. A deployment
    // that has a database and cannot reach it does not start — a store that
    // quietly kept its state in memory would look healthy and lose the cart
    // anyway.
    let repo: Arc<dyn CartRepository> = match commerce_core::db::connect_from_env().await {
        Ok(Some(pool)) => {
            tracing::info!("the cart service persists its carts in the database");
            Arc::new(DbCartRepository::new(pool))
        }
        Ok(None) => Arc::new(MemoryCartRepository::new(store)),
        Err(error) => panic!("the database must be reachable: {error}"),
    };
    let app = router(repo, prices);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind cart");
    tracing::info!(
        "cart service listening on {}, pricing from {}",
        listener.local_addr().unwrap(),
        catalog_url
    );
    axum::serve(listener, app).await.expect("serve cart");
}
