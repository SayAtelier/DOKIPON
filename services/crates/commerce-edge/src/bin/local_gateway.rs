//! The gateway as a plain HTTP process.
//!
//! The Vercel deployment serves the gateway as a Function
//! (`src/bin/vercel.rs`): the same router, wrapped in the Vercel runtime's
//! bridge. This binary serves **the same router** over plain HTTP, with no
//! serverless runtime in between, so the function topology can be run — and
//! tested — on a developer machine without a deployment.
//!
//! ## Why this is not a re-implementation
//!
//! The router is `commerce_edge::msa::gateway::router(Gateway::from_env())`.
//! That is the same expression the Vercel gateway Function builds
//! (`services/deploy/vercel/src/gateway.rs`), so the routing table, the
//! upstream addresses and the BFF that answers locally are the deployed
//! ones. Only the transport differs: TCP instead of the Vercel runtime's
//! request events.
//!
//! ## What it still needs
//!
//! The upstream addresses. `Gateway::from_env()` reads `CATALOG_URL`,
//! `CART_URL`, `ORDER_URL`, `CONTENT_URL`, `IDENTITY_URL`, `INVENTORY_URL`,
//! `PAYMENT_ADAPTER_URL` and `ADMIN_URL`, and forwards to them over HTTP —
//! the same way the deployed gateway forwards to the services' Functions.
//! A local run therefore points those variables at the local service
//! processes.
//!
//! The gateway needs no credential of its own: it forwards, it does not
//! authenticate.

use std::net::SocketAddr;

use commerce_edge::msa::gateway::{router, Gateway};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080);

    // Every service process binds 0.0.0.0, so the gateway does too: a
    // container that publishes the port needs the listener on all interfaces.
    let listener =
        tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await?;
    tracing::info!("local gateway listening on {}", listener.local_addr()?);

    let app = router(Gateway::from_env());
    axum::serve(listener, app).await?;
    Ok(())
}
