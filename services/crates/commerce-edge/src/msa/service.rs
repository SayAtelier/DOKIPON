//! Each service's Function: one service's router, mounted under its own path.
//!
//! ## What a service's Function is
//!
//! The same router the service's own binary serves, mounted under the
//! Function's path. The service's routes are rooted at `/` — the service does
//! not know it is behind a Function — and the mount strips the Function's
//! prefix before the request reaches the router, so the service sees exactly
//! the paths it registered.
//!
//! ## The price authority
//!
//! The cart and the order must not trust a client-supplied price. In the
//! in-process edge the catalogue's store is in the same process, so the
//! price is read from it. Here the catalogue is a Function of its own, so the
//! only way to ask for a price is over HTTP — which is what the cluster
//! deployment does, with the same
//! [`commerce_core::price_authority::PriceAuthority`]. One implementation of
//! the money rule, not two.

use std::sync::Arc;

use axum::{
    extract::Request,
    http::{StatusCode, Uri},
    response::Response,
    Router,
};
use tower::ServiceExt;

use super::routing::{service_prefix, ServiceId};
use super::upstreams::Upstreams;
use crate::stores;

/// The catalogue Function. It serves `/v1/products`.
pub fn catalog() -> Router {
    mount(
        ServiceId::Catalog,
        commerce_catalog::router(stores::catalog()),
    )
}

/// The cart Function. It serves `/v1/carts`.
///
/// The pool is the database the carts live in, when the deployment has one.
/// Without one the carts live in this process's memory, which is what the
/// local run and the in-process edge do on purpose.
pub fn cart(pool: Option<commerce_core::db::Pool>) -> Router {
    let repo: Arc<dyn commerce_cart::CartRepository> = match pool {
        Some(pool) => Arc::new(commerce_cart::DbCartRepository::new(pool)),
        None => Arc::new(commerce_cart::MemoryCartRepository::new(stores::cart())),
    };
    mount(ServiceId::Cart, commerce_cart::router(repo, prices()))
}

/// The order Function. It serves `/v1/checkouts` and `/v1/orders`.
///
/// The pool is the database the orders live in, when the deployment has one.
/// Without one the orders live in this process's memory, which is what the
/// local run and the in-process edge do on purpose.
pub fn order(pool: Option<commerce_core::db::Pool>) -> Router {
    let repo: Arc<dyn commerce_order::OrderRepository> = match pool {
        Some(pool) => Arc::new(commerce_order::DbOrderRepository::new(pool)),
        None => Arc::new(commerce_order::MemoryOrderRepository::new(stores::order())),
    };
    mount(ServiceId::Order, commerce_order::router(repo, prices()))
}

/// The content Function. It serves `/v1/content`.
pub fn content() -> Router {
    mount(
        ServiceId::Content,
        commerce_content::router(stores::content()),
    )
}

/// The identity Function. It serves `/v1/sessions`.
///
/// The pool is the database the sessions live in, when the deployment has
/// one. `None` means the sessions live in this process's memory, which is
/// what the local run and the in-process edge do on purpose.
pub fn identity(pool: Option<commerce_core::db::Pool>) -> Router {
    let repo = pool.map(|pool| Arc::new(commerce_identity::SessionRepo::new(pool)));
    mount(
        ServiceId::Identity,
        commerce_identity::router(stores::identity(), stores::identity_verifier(), repo),
    )
}

/// The inventory Function. It serves `/v1/inventory`.
pub fn inventory() -> Router {
    mount(
        ServiceId::Inventory,
        commerce_inventory::router(stores::inventory()),
    )
}

/// The payment adapter Function. It serves `/v1/payments`.
pub fn payment() -> Router {
    mount(
        ServiceId::Payment,
        commerce_payment_adapter::router(stores::payment()),
    )
}

/// The admin facade Function. It serves `/admin/v1`.
///
/// The facade reads its credentials and its upstream URLs from the
/// environment, so building it can fail; the error is the message the
/// deployment shows.
pub fn admin() -> Result<Router, String> {
    let state = commerce_admin::AdminState::from_env()?;
    Ok(mount(ServiceId::Admin, commerce_admin::router(state)))
}

/// The price authority, asking the catalogue Function over HTTP.
fn prices() -> Arc<dyn commerce_core::PriceSource> {
    let catalog = Upstreams::from_env()
        .base_url_for(ServiceId::Catalog)
        .unwrap_or_default()
        .to_string();
    Arc::new(commerce_core::price_authority::PriceAuthority::new(catalog))
}

/// Mount a service's router under the Function's own path.
///
/// The Function is reachable at `/api/svc/<service>`; the service's routes
/// are rooted at `/`, so the prefix is stripped before the request reaches
/// the router.
fn mount(id: ServiceId, inner: Router) -> Router {
    let prefix = service_prefix(id).to_string();
    Router::new().fallback(move |request: Request| {
        let prefix = prefix.clone();
        let inner = inner.clone();
        async move { dispatch(&prefix, inner, request).await }
    })
}

/// Hand a request to the service's own router.
///
/// The prefix is stripped from the *path*, not from the routing: the service
/// sees the path it registered, with the client's query string.
async fn dispatch(prefix: &str, inner: Router, mut request: Request) -> Response {
    let path = request.uri().path().to_string();
    let rest = match path.strip_prefix(prefix) {
        Some("") => "/".to_string(),
        Some(rest) => rest.to_string(),
        // A request that did not come in under the prefix is handed over
        // unchanged: the service decides what to do with it.
        None => path.clone(),
    };
    let query = request.uri().query().map(|query| query.to_string());
    let target = match query {
        Some(query) => format!("{rest}?{query}"),
        None => rest.clone(),
    };
    if let Ok(uri) = Uri::builder().path_and_query(target).build() {
        *request.uri_mut() = uri;
    }

    match inner.oneshot(request).await {
        Ok(response) => response,
        Err(error) => {
            // The service's router failed to produce a response. This is a
            // bug in the service, not a client error, and it must not be
            // reported as a 404 — that would send the client looking for a
            // route that exists.
            tracing::error!(%error, "the service router failed");
            super::problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "service.dispatch_failed",
                "the service handling this path could not answer",
            )
        }
    }
}
