//! Standalone binary for the commerce-payment-adapter service.
//!
//! The service's logic lives in `lib.rs` so that the merged `commerce-edge`
//! deployment — one Vercel function serving every service — can mount the
//! same router alongside the others. This binary is the local and k3s
//! deployment: it binds a port and serves the router on its own.

use commerce_payment_adapter::*;
use std::sync::Arc;
use std::sync::RwLock;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    // The webhook secret is a credential, not configuration: a deployment
    // that keeps the development value accepts a webhook signed by anyone.
    // The service refuses to start without a real one.
    let secret = commerce_core::secrets::require(
        "PSP_WEBHOOK_SECRET",
        commerce_core::secrets::DEV_PSP_WEBHOOK_SECRET,
    )
    .expect("PSP_WEBHOOK_SECRET must be set to a real webhook secret");
    let store: SharedPayment = Arc::new(RwLock::new(PaymentAdapter::new(secret.clone())));
    // The attempts live in the database when the deployment has one: a
    // provider event that arrives at one instance has to be refused at the
    // next. A deployment that has a database and cannot reach it does not
    // start — a store that quietly kept its state in memory would look
    // healthy and lose the attempt anyway.
    let repo: Arc<dyn PaymentRepository> = match commerce_core::db::connect_from_env().await {
        Ok(Some(pool)) => {
            tracing::info!("the payment adapter persists its attempts in the database");
            Arc::new(DbPaymentRepository::new(pool, secret))
        }
        Ok(None) => Arc::new(MemoryPaymentRepository::new(store)),
        Err(error) => panic!("the database must be reachable: {error}"),
    };
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind payment");
    tracing::info!(
        "payment adapter listening on {}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, router(repo))
        .await
        .expect("serve payment");
}
