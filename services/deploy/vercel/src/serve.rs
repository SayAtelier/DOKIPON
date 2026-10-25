//! Serving a router as a Vercel function.
//!
//! The Vercel Rust runtime bridges a function to a tower service: the layer
//! turns the runtime's request event into an Axum request, and the Axum
//! response back into the runtime's response event. Every function in this
//! deployment is the same three lines — build the router, wrap it in the
//! layer, run it — so they live here.
//!
//! ## One runtime per Function
//!
//! The runtime belongs to this module, and to nothing else. A function that
//! made a runtime of its own — a `#[tokio::main]` entry point, say — and then
//! called into here would start a runtime *inside* the one it was already
//! running on. Tokio refuses that: `block_on` cannot block a thread that is
//! already driving an executor, so it panics with "Cannot start a runtime
//! from within a runtime", and the Function dies on its first request.
//!
//! So a function whose router needs something asynchronous — the identity
//! Function has to reach its database before it has a router at all — builds
//! that router *inside* the runtime, through [`function_from`], and never
//! outside it.

use std::future::Future;

use axum::Router;
use tower::ServiceBuilder;
use vercel_runtime::axum::VercelLayer;

/// A configuration failure, as the Function's error type.
///
/// `vercel_runtime::Error` is `Box<dyn std::error::Error + Send + Sync>`, so
/// a configuration failure is a plain boxed error. The Function then exits
/// non-zero instead of serving traffic it cannot authenticate.
///
/// Every Function includes this module, and only the ones that read a
/// credential need this helper, so it is allowed to go unused in the rest.
#[allow(dead_code)]
pub fn config_error(message: String) -> vercel_runtime::Error {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        message,
    ))
}

/// Serve `router` as a Vercel function.
///
/// The router is the whole function: the Function's path is stripped by
/// Vercel's routing before the request arrives, and the service's own routes
/// are rooted at `/`.
///
/// The identity Function does not use this one — it builds its router inside
/// the runtime, through [`function_from`], because it has to reach its
/// database first. Every Function includes this module, so this one is
/// allowed to go unused in that binary.
#[allow(dead_code)]
pub fn function(router: Router) -> Result<(), vercel_runtime::Error> {
    function_from(|| async move { Ok(router) })
}

/// Serve a router that has to be built inside the runtime that serves it.
///
/// `build` runs on the runtime this Function serves on, so anything it awaits
/// — a database connection, a migration — is awaited on that runtime and not
/// on one of its own. A failure to build the router stops the Function: it
/// exits non-zero rather than serve traffic it cannot authenticate.
pub fn function_from<F, Fut>(build: F) -> Result<(), vercel_runtime::Error>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Router, vercel_runtime::Error>>,
{
    // The admin facade reads its credentials and its upstream URLs from the
    // environment, and so does every service that needs a secret. The logs
    // are the only way to see what a Function did, so they are always on.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|error| -> vercel_runtime::Error { Box::new(error) })?;

    runtime.block_on(async {
        let router = build().await?;
        let app = ServiceBuilder::new()
            .layer(VercelLayer::new())
            .service(router);
        vercel_runtime::run(app).await
    })
}
