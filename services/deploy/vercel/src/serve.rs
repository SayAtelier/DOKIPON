//! Serving a router as a Vercel function.
//!
//! The Vercel Rust runtime bridges a function to a tower service: the layer
//! turns the runtime's request event into an Axum request, and the Axum
//! response back into the runtime's response event. Every function in this
//! deployment is the same three lines — build the router, wrap it in the
//! layer, run it — so they live here.

use axum::Router;
use tower::ServiceBuilder;
use vercel_runtime::axum::VercelLayer;

/// Serve `router` as a Vercel function.
///
/// The router is the whole function: the Function's path is stripped by
/// Vercel's routing before the request arrives, and the service's own routes
/// are rooted at `/`.
pub fn function(router: Router) -> Result<(), vercel_runtime::Error> {
    // The admin facade reads its credentials and its upstream URLs from the
    // environment, and so does every service that needs a secret. The logs
    // are the only way to see what a Function did, so they are always on.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let runtime = tokio::runtime::Runtime::new().map_err(|error| -> vercel_runtime::Error {
        Box::new(error)
    })?;

    runtime.block_on(async {
        let app = ServiceBuilder::new()
            .layer(VercelLayer::new())
            .service(router);
        vercel_runtime::run(app).await
    })
}
