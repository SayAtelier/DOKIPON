//! The DOKIPON edge as a Vercel function.
//!
//! The Vercel Rust runtime bridges a function to a tower service. This binary
//! wraps the same edge router the standalone binary serves, so the deployed
//! function and the local process answer exactly the same requests.
//!
//! ## What the deployment is
//!
//! A **preview and demonstration** deployment. The commerce services keep
//! their state in memory, and Vercel may run several instances of the same
//! function at once. A cart created on one instance is invisible to another,
//! so a shopper's cart can appear to empty itself. The catalogue, the
//! content document and the admin console are unaffected — they are either
//! static or idempotent reads.
//!
//! Making this a production deployment means moving the mutable stores
//! (cart, order, inventory) into a shared database. That is a change to the
//! stores themselves; the edge does not have to change.

use commerce_edge::build;
use tower::ServiceBuilder;
use vercel_runtime::axum::VercelLayer;
use vercel_runtime::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // This Function serves every service in one process, so it needs every
    // service's credentials. A Function that kept a development credential
    // would authenticate nobody it should not, so it refuses to start.
    commerce_edge::secrets::require_for(commerce_edge::secrets::Service::All)
        .map_err(|message| Box::<dyn std::error::Error + Send + Sync>::from(message))?;

    // The admin facade reads its credentials and its upstream URLs from the
    // environment. On Vercel those are the project's environment variables.
    // `vercel_runtime::Error` is `Box<dyn std::error::Error + Send + Sync>`,
    // so a configuration failure is a plain boxed error.
    let admin = commerce_admin::AdminState::from_env()
        .map_err(|message| Box::<dyn std::error::Error + Send + Sync>::from(message))?;

    let (edge, _services) = build(admin);

    let app = ServiceBuilder::new()
        .layer(VercelLayer::new())
        .service(edge);

    vercel_runtime::run(app).await
}
