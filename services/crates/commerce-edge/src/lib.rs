//! The DOKIPON edge: every commerce service behind one router.
//!
//! ## Why this exists
//!
//! The stack is eight Axum services. On a cluster they are eight containers
//! behind an Envoy gateway, and that is how they run in production. But a
//! serverless platform gives you **one** function per deployment, and the
//! eight services cannot be eight functions without either eight deployments
//! or a rewrite into something stateless — and they are not stateless: the
//! cart, the order store and the inventory ledger all live in memory.
//!
//! So the edge mounts all eight routers into one process and dispatches by
//! path. The services keep their own routers, their own state and their own
//! types; nothing about their internal design changes. What changes is only
//! *where* they live.
//!
//! ## How the dispatch works
//!
//! Mounting the eight routers at the root does not work, because the services
//! disagree about two paths:
//!
//! | Path | Conflict |
//! |---|---|
//! | `/healthz` | five services define it |
//! | `/v1/sessions/guest` | the BFF and the identity service both define it |
//!
//! Axum panics at startup on a duplicate route, so the root mount is not an
//! option without renaming paths inside the services — which would change
//! the contract's public paths, and the contract is the SSOT.
//!
//! Instead the edge keeps the eight routers whole and **dispatches** to one of
//! them per request. A request arrives at the edge's fallback; the fallback
//! picks the service that owns the path and forwards the request to that
//! service's router with `tower::Service::oneshot`, then returns the
//! service's response unchanged.
//!
//! The advantages over path rewriting:
//!
//! - the service routers are used **exactly** as they were written — the
//!   path they see is the path they registered, so no prefix can leak;
//! - the edge's own routing table is the only place that knows the public
//!   path list, so the public surface is readable in one place;
//! - a service that adds a route needs no change to the edge's dispatch, only
//!   a line in the path table.
//!
//! The cost is one `oneshot` per request, which is a tower call, not a
//! network round trip: the services are all in this process.
//!
//! ## The paths the client sees
//!
//! These are the contract's paths, unchanged:
//!
//! | Prefix | Service |
//! |---|---|
//! | `/v1/storefront` | BFF |
//! | `/v1/products` | catalogue |
//! | `/v1/carts` | cart |
//! | `/v1/checkouts`, `/v1/orders` | order |
//! | `/v1/content` | content |
//! | `/v1/sessions` | identity |
//! | `/v1/inventory` | inventory |
//! | `/v1/payments` | payment adapter |
//! | `/admin/v1` | admin facade |
//!
//! ## What this deployment does not solve
//!
//! The services hold their state in memory, and a serverless platform may run
//! several instances of the same function. A cart created on one instance is
//! invisible to another. This is a **demonstration and preview** deployment:
//! it is correct for the catalogue, the content document and the admin
//! console, and it is *eventually* correct for the cart and the order store
//! (a shopper's next request may land on a different instance). A production
//! deployment needs the state moved into a shared store — Postgres or Redis —
//! which is a change to the stores, not to this edge.
//!
//! The price authority is the exception to "in-memory": it is a pure function
//! of the catalogue, so the edge can serve it in-process without an HTTP
//! round trip and without the multi-instance hazard.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use commerce_core::price_authority::PriceSource;
use tower::ServiceExt;

/// The catalogue prices, served in-process.
///
/// Every other service needs the catalogue's price for a SKU before it can do
/// its job: the cart prices a line, the order re-prices a checkout. On the
/// cluster those calls are HTTP; here they read the same `CatalogStore` the
/// catalogue handler reads, so the price cannot disagree with the price the
/// catalogue shows, and a request does not pay for an extra round trip.
///
/// The catalogue store is immutable after seeding in this build, so no lock
/// is needed to read it.
#[derive(Clone)]
pub struct InProcessPrices {
    catalog: Arc<commerce_catalog::CatalogStore>,
}

impl InProcessPrices {
    /// Look up the authoritative price for a SKU.
    pub fn price_of(&self, sku: &str) -> Option<commerce_core::Money> {
        let found = self.catalog.skus.get(sku)?;
        let currency = found.currency;
        commerce_core::Money::new(found.amount_minor, currency).ok()
    }
}

#[async_trait::async_trait]
impl PriceSource for InProcessPrices {
    /// The edge's variant of the price lookup: one catalogue scan, not one
    /// HTTP call per line.
    async fn fetch_prices(
        &self,
        skus: &[String],
    ) -> Result<Vec<(String, commerce_core::Money)>, String> {
        Ok(skus
            .iter()
            .filter_map(|sku| self.price_of(sku).map(|money| (sku.clone(), money)))
            .collect())
    }
}

/// Which service owns a public path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceId {
    Bff,
    Catalog,
    Cart,
    Order,
    Content,
    Identity,
    Inventory,
    Payment,
    Admin,
}

/// Pick the service that owns a path.
///
/// A path that matches nothing is a 404 from the edge, not a pass-through to
/// the last service: a typo in a public path must not be answered by
/// whichever service happens to be last in the table.
pub fn route_for(path: &str) -> Option<ServiceId> {
    const TABLE: &[(&str, ServiceId)] = &[
        ("/v1/storefront", ServiceId::Bff),
        ("/v1/products", ServiceId::Catalog),
        ("/v1/carts", ServiceId::Cart),
        ("/v1/checkouts", ServiceId::Order),
        ("/v1/orders", ServiceId::Order),
        ("/v1/content", ServiceId::Content),
        ("/v1/sessions", ServiceId::Identity),
        ("/v1/inventory", ServiceId::Inventory),
        ("/v1/payments", ServiceId::Payment),
        ("/admin/v1", ServiceId::Admin),
    ];
    TABLE
        .iter()
        .find(|(prefix, _)| path == *prefix || path.starts_with(&format!("{prefix}/")))
        .map(|(_, id)| *id)
}

/// The eight service routers, ready to be dispatched to.
#[derive(Clone)]
pub struct EdgeServices {
    pub bff: Router,
    pub catalog: Router,
    pub cart: Router,
    pub order: Router,
    pub content: Router,
    pub identity: Router,
    pub inventory: Router,
    pub payment: Router,
    pub admin: Router,
}

impl EdgeServices {
    /// The router that owns a path.
    fn router_for(&self, id: ServiceId) -> &Router {
        match id {
            ServiceId::Bff => &self.bff,
            ServiceId::Catalog => &self.catalog,
            ServiceId::Cart => &self.cart,
            ServiceId::Order => &self.order,
            ServiceId::Content => &self.content,
            ServiceId::Identity => &self.identity,
            ServiceId::Inventory => &self.inventory,
            ServiceId::Payment => &self.payment,
            ServiceId::Admin => &self.admin,
        }
    }
}

/// Build the eight service routers and the edge that dispatches to them.
///
/// The `admin` state is passed in rather than read from the environment so a
/// test can mount the same edge with a mock upstream.
pub fn build(admin: commerce_admin::AdminState) -> (Router, EdgeServices) {
    let catalog = Arc::new(commerce_catalog::CatalogStore::seed_demo());
    let cart = Arc::new(std::sync::RwLock::new(commerce_cart::CartStore::new()));
    let order = Arc::new(std::sync::RwLock::new(commerce_order::OrderStore::new(
        commerce_core::idempotency::now_ms(),
    )));
    let inventory = Arc::new(std::sync::RwLock::new(
        commerce_inventory::InventoryStore::new(&[
            // These SKUs must match the catalogue's seed: the checkout path
            // looks stock up by sku, and a sku that exists in one service but
            // not the other makes a purchasable product fail at the last step.
            //
            // Only the `Reserved` skus are here. `Unbounded` and
            // `MadeToOrder` need no stock (`StockPolicy::needs_inventory`),
            // which is why `sku-cami-01` is absent.
            ("sku-bra-01", 42),
            ("sku-brief-01", 7),
            ("sku-set-01", 0),
            ("sku-night-01", 28),
            ("sku-adult-01", 3),
            ("sku-swim-01", 12),
            ("sku-sale-01", 5),
        ]),
    ));
    let identity = Arc::new(std::sync::RwLock::new(
        commerce_identity::IdentityStore::new(commerce_core::idempotency::now_ms()),
    ));
    let payment = Arc::new(std::sync::RwLock::new(
        commerce_payment_adapter::PaymentAdapter::new(
            std::env::var("PSP_WEBHOOK_SECRET").unwrap_or_else(|_| "dev-only-secret".into()),
        ),
    ));
    let content = Arc::new(std::sync::RwLock::new(commerce_content::ContentStore::new(
        std::env::var("CONTENT_PATH").ok().map(std::path::PathBuf::from),
    )));

    let prices: Arc<dyn PriceSource> = Arc::new(InProcessPrices {
        catalog: catalog.clone(),
    });

    let services = EdgeServices {
        bff: commerce_bff::router(Arc::new(std::sync::RwLock::new(
            commerce_bff::RateLimiter::default(),
        ))),
        catalog: commerce_catalog::router(catalog),
        // The cart and the order both need a price authority. The edge serves
        // prices in-process, so the two services get the in-process variant
        // rather than an HTTP client that would call back into itself.
        cart: commerce_cart::router(cart, prices.clone()),
        order: commerce_order::router(order, prices),
        content: commerce_content::router(content),
        identity: commerce_identity::router(
            identity,
            Arc::new(commerce_identity::TokenVerifier {
                expected_audience: std::env::var("LINE_CHANNEL_ID")
                    .unwrap_or_else(|_| "dev-channel".into()),
                expected_issuer: "https://access.line.me".to_string(),
                signing_key: std::env::var("LINE_ID_TOKEN_KEY")
                    .unwrap_or_else(|_| "dev-only-key".into()),
            }),
        ),
        inventory: commerce_inventory::router(inventory),
        payment: commerce_payment_adapter::router(payment),
        admin: commerce_admin::router(admin),
    };

    let edge = Router::new()
        // The health check answers for the whole edge.
        .route("/healthz", get(edge_health))
        // Everything else is dispatched to the service that owns the path.
        .fallback(dispatch)
        .with_state(services.clone());

    (edge, services)
}

/// The edge's own health check.
///
/// It answers 200 when the edge is up. It does **not** try to reach every
/// service: in this deployment they are all in this process, so "the edge is
/// up" is the only question there is.
async fn edge_health() -> StatusCode {
    StatusCode::OK
}

/// Forward a request to the service that owns its path.
///
/// The request is passed through unchanged: the service router sees exactly
/// the path, method, headers and body it would have seen if it were the only
/// process on the machine.
async fn dispatch(State(services): State<EdgeServices>, request: Request) -> Response {
    let path = request.uri().path().to_string();
    let Some(id) = route_for(&path) else {
        return not_found(&path);
    };

    let router = services.router_for(id).clone();
    match router.oneshot(request).await {
        Ok(response) => response,
        Err(error) => {
            // A service's router failed to produce a response. This is a bug
            // in the service, not a client error, and it must not be
            // reported as a 404 — that would send the client looking for a
            // route that exists.
            tracing::error!(%error, path = %path, "service router failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(axum::http::header::CONTENT_TYPE, "application/problem+json")],
                serde_json::json!({
                    "type": "https://api.gemarc.example/problems/internal",
                    "title": "Internal Server Error",
                    "status": 500,
                    "code": "edge.dispatch_failed",
                    "detail": "the service handling this path could not answer",
                    "traceId": "",
                    "retryable": true,
                })
                .to_string(),
            )
                .into_response()
        }
    }
}

/// A path that belongs to no service.
///
/// This is the same shape as every other failure in the codebase: an RFC 9457
/// ProblemDetails, so a client parses one error format regardless of which
/// layer failed.
fn not_found(path: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        [(axum::http::header::CONTENT_TYPE, "application/problem+json")],
        serde_json::json!({
            "type": "https://api.gemarc.example/problems/not-found",
            "title": "Not Found",
            "status": 404,
            "code": "edge.no_such_route",
            "detail": format!("no service handles {path}"),
            "traceId": "",
            "retryable": false,
        })
        .to_string(),
    )
        .into_response()
}
