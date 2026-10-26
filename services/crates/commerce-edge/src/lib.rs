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

pub mod msa;
pub mod stores;

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
///
/// The table itself lives in `commerce-core::routes`, because the BFF reads
/// the same table when the services run as separate containers: two tables
/// would drift, and a drifted table is a 404 that only the deployed topology
/// shows.
pub use commerce_core::routes::{owner_of as route_for, ServiceId};

/// The credentials a deployment has to set before it starts a service.
///
/// Re-exported here so that the deployment's entry points — which depend on
/// this crate, not on `commerce-core` directly — can check the credentials
/// without naming another dependency.
pub use commerce_core::secrets;

/// The database the services persist to, when the deployment has one.
///
/// Re-exported here for the same reason as [`secrets`]: the deployment's
/// entry points depend on this crate, not on `commerce-core` directly.
pub use commerce_core::db;

/// The stock levels, served in-process.
///
/// The catalogue asks the inventory service how much of each SKU is left, so
/// that the storefront is told what the ledger says rather than what the
/// catalogue copied of it a while ago.
///
/// In this deployment the ledger is **in this process**, so the answer is
/// read from the store the inventory service already holds — no HTTP round
/// trip, and no chance of the two disagreeing.
#[derive(Clone)]
pub struct InProcessStocks {
    inventory: Arc<std::sync::RwLock<commerce_inventory::InventoryStore>>,
}

impl InProcessStocks {
    pub fn new(inventory: Arc<std::sync::RwLock<commerce_inventory::InventoryStore>>) -> Self {
        Self { inventory }
    }
}

#[async_trait::async_trait]
impl commerce_core::StockSource for InProcessStocks {
    async fn fetch_levels(
        &self,
        skus: &[String],
    ) -> Result<Vec<(String, commerce_core::StockLevel)>, String> {
        let guard = self
            .inventory
            .read()
            .map_err(|_| "the inventory ledger's lock is poisoned".to_string())?;
        Ok(guard
            .stock
            .iter()
            .filter(|(sku, _)| skus.iter().any(|wanted| wanted == *sku))
            .map(|(sku, level)| {
                (
                    sku.clone(),
                    commerce_core::StockLevel {
                        on_hand: level.on_hand,
                        reserved: level.reserved,
                    },
                )
            })
            .collect())
    }
}

/// The stock reservations, served in-process.
///
/// The order service asks the inventory service to hold the units before it
/// asks for the payment. In this deployment the ledger is **in this
/// process**, so the reservation is made against the store the inventory
/// service already holds — the same rules, the same ledger.
#[derive(Clone)]
pub struct InProcessReservations {
    inventory: Arc<std::sync::RwLock<commerce_inventory::InventoryStore>>,
}

impl InProcessReservations {
    pub fn new(inventory: Arc<std::sync::RwLock<commerce_inventory::InventoryStore>>) -> Self {
        Self { inventory }
    }
}

#[async_trait::async_trait]
impl commerce_core::ReservationSource for InProcessReservations {
    async fn reserve(
        &self,
        sku: &str,
        order_id: &str,
        quantity: i64,
    ) -> Result<commerce_core::Reservation, String> {
        let mut guard = self
            .inventory
            .write()
            .map_err(|_| "the inventory ledger's lock is poisoned".to_string())?;
        guard
            .reserve(sku, order_id, quantity)
            .map_err(|error| error.to_string())
    }

    async fn commit(&self, reservation_id: &str) -> Result<commerce_core::Reservation, String> {
        let mut guard = self
            .inventory
            .write()
            .map_err(|_| "the inventory ledger's lock is poisoned".to_string())?;
        guard
            .commit(reservation_id)
            .map_err(|error| error.to_string())
    }

    async fn release(&self, reservation_id: &str) -> Result<commerce_core::Reservation, String> {
        let mut guard = self
            .inventory
            .write()
            .map_err(|_| "the inventory ledger's lock is poisoned".to_string())?;
        guard
            .release(reservation_id)
            .map_err(|error| error.to_string())
    }

    async fn list_for_order(
        &self,
        order_id: &str,
    ) -> Result<Vec<commerce_core::Reservation>, String> {
        let guard = self
            .inventory
            .read()
            .map_err(|_| "the inventory ledger's lock is poisoned".to_string())?;
        let mut held: Vec<commerce_core::Reservation> = guard
            .reservations
            .values()
            .filter(|reservation| reservation.order_id == order_id)
            .map(|reservation| commerce_core::Reservation {
                reservation_id: reservation.reservation_id.clone(),
                sku: reservation.sku.clone(),
                order_id: reservation.order_id.clone(),
                quantity: reservation.quantity,
                state: reservation.state,
                expires_at_ms: reservation.expires_at_ms,
            })
            .collect();
        held.sort_by(|a, b| a.reservation_id.cmp(&b.reservation_id));
        Ok(held)
    }
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
    // The stores are built by `stores`, so the in-process edge and the
    // function topology (`msa`) seed them the same way. The inventory
    // ledger's seed in particular has to match the catalogue's, or a
    // purchasable product fails at the last step of the checkout.
    let catalog = stores::catalog();
    let cart = stores::cart();
    let order = stores::order();
    let inventory = stores::inventory();
    let identity = stores::identity();
    let payment = stores::payment();
    let content = stores::content();

    let prices: Arc<dyn PriceSource> = Arc::new(InProcessPrices {
        catalog: catalog.clone(),
    });
    // The catalogue asks the ledger how much of each SKU is left, so the
    // storefront is told what the ledger says. Here the ledger is in this
    // process, so the answer comes from the store.
    let stocks: Arc<dyn commerce_core::StockSource> =
        Arc::new(InProcessStocks::new(inventory.clone()));
    // The order service asks the ledger to hold the units before it asks for
    // the payment. Here the ledger is in this process.
    let reservations: Arc<dyn commerce_core::ReservationSource> =
        Arc::new(InProcessReservations::new(inventory.clone()));

    let services = EdgeServices {
        bff: commerce_bff::router(Arc::new(std::sync::RwLock::new(
            commerce_bff::RateLimiter::default(),
        ))),
        catalog: commerce_catalog::router(catalog, stocks),
        // The cart and the order both need a price authority. The edge serves
        // prices in-process, so the two services get the in-process variant
        // rather than an HTTP client that would call back into itself.
        cart: commerce_cart::router(
            Arc::new(commerce_cart::MemoryCartRepository::new(cart)),
            prices.clone(),
        ),
        order: commerce_order::router(
            Arc::new(commerce_order::MemoryOrderRepository::new(order)),
            prices,
            reservations,
        ),
        content: commerce_content::router(content),
        identity: commerce_identity::router(identity, stores::identity_verifier(), None),
        inventory: commerce_inventory::router(Arc::new(
            commerce_inventory::MemoryInventoryRepository::new(inventory),
        )),
        payment: commerce_payment_adapter::router(Arc::new(
            commerce_payment_adapter::MemoryPaymentRepository::new(payment),
        )),
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
