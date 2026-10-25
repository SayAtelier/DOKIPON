//! Inventory service — available / reserved / committed with reservation TTL.
//!
//! Invariants (atlas §B5, §E):
//! - `available = on_hand − reserved` and **availability must never go negative**.
//! - A reservation is uniquely keyed so a double-reserve is impossible.
//! - Reservations expire on TTL; expiry is a release, not a silent commit.
//! - Compensating a confirmed order releases the reservation; a rejected or
//!   cancelled order releases too. Only a confirmed order commits.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use commerce_core::JsonBody;
use commerce_core::{
    error::{status, AppError},
    ReservationState,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub type SharedInventory = Arc<RwLock<InventoryStore>>;

pub mod repo;

pub use repo::{DbInventoryRepository, InventoryRepository, MemoryInventoryRepository};

pub const RESERVATION_TTL_MS: i64 = 15 * 60 * 1000; // 15 minutes

/// The demo stock levels the service starts with.
///
/// These SKUs must match the catalogue's seed: the checkout path looks stock
/// up by sku, and a sku that exists in one service but not the other makes a
/// purchasable product fail at the last step of the checkout. A made-to-order
/// SKU is absent here for the same reason it is absent from the catalogue's
/// stock map — it has no stock concept. The same holds for an `Unbounded`
/// SKU: only a `Reserved` SKU takes part in the inventory step
/// (`StockPolicy::needs_inventory`), so only those have a level to report.
///
/// The list route publishes these SKUs, so a seed that drifts from the
/// catalogue's shows up in the admin console as a SKU with no product.
pub const DEMO_STOCK_SEED: &[(&str, i64)] = &[
    ("sku-bra-01", 42),
    ("sku-brief-01", 7),
    ("sku-set-01", 0),
    ("sku-night-01", 28),
    ("sku-adult-01", 3),
    ("sku-swim-01", 12),
    ("sku-sale-01", 5),
];

#[derive(Debug, Clone, Default)]
pub struct StockLevel {
    pub on_hand: i64,
    pub reserved: i64,
}

impl StockLevel {
    pub fn available(&self) -> i64 {
        (self.on_hand - self.reserved).max(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reservation {
    pub reservation_id: String,
    pub sku: String,
    pub order_id: String,
    pub quantity: i64,
    pub state: ReservationState,
    pub expires_at_ms: i64,
}

#[derive(Debug, Default)]
pub struct InventoryStore {
    pub stock: HashMap<String, StockLevel>,
    /// `reservation_id` → reservation. The unique key is what makes a
    /// double-reserve structurally impossible rather than merely unlikely.
    pub reservations: HashMap<String, Reservation>,
    pub sequence: u64,
}

impl InventoryStore {
    /// A store seeded with the given stock levels.
    ///
    /// The name the rest of the codebase uses; `seed` is the underlying
    /// constructor. Both exist because the catalogue and the inventory
    /// service must agree on the SKU list, and a mismatch there makes a
    /// purchasable product fail at the last step of the checkout.
    pub fn new(levels: &[(&str, i64)]) -> Self {
        Self::seed(levels)
    }

    pub fn seed(levels: &[(&str, i64)]) -> Self {
        Self {
            stock: levels
                .iter()
                .map(|(sku, n)| {
                    (
                        sku.to_string(),
                        StockLevel {
                            on_hand: *n,
                            reserved: 0,
                        },
                    )
                })
                .collect(),
            reservations: HashMap::new(),
            sequence: 0,
        }
    }

    fn next_reservation_id(&mut self) -> String {
        self.sequence += 1;
        format!("res-{:06}", self.sequence)
    }

    fn level_mut(&mut self, sku: &str) -> Result<&mut StockLevel, AppError> {
        self.stock.get_mut(sku).ok_or_else(|| {
            status::not_found(
                "inventory.sku_not_found",
                format!("unknown sku {sku}"),
                "inventory.reserve",
            )
        })
    }

    /// `inventory.reserve` — idempotent per `order_id`: reserving twice for the
    /// same order returns the SAME reservation instead of double-decrementing.
    pub fn reserve(
        &mut self,
        sku: &str,
        order_id: &str,
        quantity: i64,
    ) -> Result<Reservation, AppError> {
        const TRACE: &str = "inventory.reserve";
        if quantity <= 0 {
            return Err(status::unprocessable(
                "inventory.bad_quantity",
                "quantity must be >= 1",
                TRACE,
            ));
        }

        // Idempotency: one active reservation per (order_id, sku).
        if let Some(existing) = self
            .reservations
            .values()
            .find(|r| r.order_id == order_id && r.sku == sku && !r.state.is_terminal())
        {
            return Ok(existing.clone());
        }

        let level = self.level_mut(sku)?;
        if level.available() < quantity {
            return Err(status::conflict(
                "inventory.insufficient",
                format!("only {} available for {sku}", level.available()),
                TRACE,
            ));
        }
        level.reserved += quantity;

        let reservation = Reservation {
            reservation_id: self.next_reservation_id(),
            sku: sku.to_string(),
            order_id: order_id.to_string(),
            quantity,
            state: ReservationState::Reserved,
            expires_at_ms: commerce_core::idempotency::now_ms() + RESERVATION_TTL_MS,
        };
        self.reservations
            .insert(reservation.reservation_id.clone(), reservation.clone());
        Ok(reservation)
    }

    /// Called when the payment succeeds and the order is confirmed.
    pub fn commit(&mut self, reservation_id: &str) -> Result<Reservation, AppError> {
        const TRACE: &str = "inventory.commit";
        let res = self.reservations.get_mut(reservation_id).ok_or_else(|| {
            status::not_found(
                "inventory.reservation_not_found",
                "reservation not found",
                TRACE,
            )
        })?;
        if res.state.is_terminal() {
            return Err(status::conflict(
                "inventory.reservation_terminal",
                format!("reservation already {:?}", res.state),
                TRACE,
            ));
        }
        let sku = res.sku.clone();
        let qty = res.quantity;
        res.state = ReservationState::Committed;
        // reserved decreases, on_hand decreases — the units actually left the warehouse.
        if let Some(level) = self.stock.get_mut(&sku) {
            level.reserved = (level.reserved - qty).max(0);
            level.on_hand = (level.on_hand - qty).max(0);
        }
        Ok(res.clone())
    }

    /// Compensation: payment declined / order rejected / cancelled / confirmation failed.
    pub fn release(&mut self, reservation_id: &str) -> Result<Reservation, AppError> {
        const TRACE: &str = "inventory.release";
        let res = self.reservations.get_mut(reservation_id).ok_or_else(|| {
            status::not_found(
                "inventory.reservation_not_found",
                "reservation not found",
                TRACE,
            )
        })?;
        if res.state.is_terminal() {
            return Err(status::conflict(
                "inventory.reservation_terminal",
                format!("reservation already {:?}", res.state),
                TRACE,
            ));
        }
        let sku = res.sku.clone();
        let qty = res.quantity;
        res.state = ReservationState::Released;
        if let Some(level) = self.stock.get_mut(&sku) {
            level.reserved = (level.reserved - qty).max(0);
        }
        Ok(res.clone())
    }

    /// TTL sweep. Returns the reservations that expired, for the Order service's
    /// compensation queue. Deterministic given an injected `now`.
    pub fn expire_stale(&mut self, now: i64) -> Vec<Reservation> {
        let stale: Vec<String> = self
            .reservations
            .values()
            .filter(|r| !r.state.is_terminal() && r.expires_at_ms <= now)
            .map(|r| r.reservation_id.clone())
            .collect();
        let mut expired = Vec::new();
        for id in stale {
            if let Ok(r) = self.release(&id) {
                expired.push(r);
            }
        }
        expired
    }

    pub fn available(&self, sku: &str) -> Option<i64> {
        self.stock.get(sku).map(StockLevel::available)
    }

    /// Global invariant checker used by tests and by a startup self-check.
    pub fn invariants_hold(&self) -> bool {
        self.stock
            .values()
            .all(|l| l.on_hand >= 0 && l.reserved >= 0 && l.reserved <= l.on_hand)
    }
}

#[derive(Debug, Deserialize)]
pub struct ReserveBody {
    pub sku: String,
    pub order_id: String,
    pub quantity: i64,
}

#[derive(Debug, Deserialize)]
pub struct ReservationPath {
    pub reservation_id: String,
}

/// The query of `GET /v1/inventory`.
///
/// `cursor` is the offset of the next page, as the catalogue's list route
/// defines it. `limit` is clamped to `1..=100` so one caller cannot ask for
/// the whole store in a single response.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

/// One row of `GET /v1/inventory`.
///
/// The field names are camelCase because that is what the rest of the
/// commerce API emits: `onHand` / `reserved` / `available` are the three
/// numbers the stock row carries everywhere else in the codebase.
///
/// The row is keyed by SKU, and only by SKU. SKU is the inventory service's
/// key — it is what the reservation ledger stores and what the checkout path
/// looks up — while product identity belongs to the catalogue. There is no
/// product id here, because the inventory service does not know one.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StockLevelRow {
    pub sku: String,
    pub on_hand: i64,
    pub reserved: i64,
    pub available: i64,
}

/// A page of stock levels: `items` plus `next_cursor`.
///
/// The envelope is the one the catalogue's list route uses, so a client that
/// already pages the catalogue pages this the same way. `next_cursor` keeps
/// its snake_case spelling because that is the spelling the contract uses for
/// every list route.
#[derive(Debug, Clone, Serialize)]
pub struct StockPage {
    pub items: Vec<StockLevelRow>,
    pub next_cursor: Option<String>,
}

pub fn router(state: Arc<dyn InventoryRepository>) -> Router {
    Router::new()
        .route("/v1/inventory", get(list_inventory))
        .route("/v1/inventory/reservations", post(reserve))
        .route(
            "/v1/inventory/reservations/{reservation_id}/commit",
            post(commit),
        )
        .route(
            "/v1/inventory/reservations/{reservation_id}/release",
            post(release),
        )
        .route("/healthz", get(health))
        .with_state(state)
}

async fn health() -> StatusCode {
    StatusCode::OK
}

/// `inventory.list` — the stock levels this service actually holds.
///
/// The admin facade proxies `GET /admin/v1/inventory` here, so the rows are
/// the live state of the ledger rather than a copy of it.
///
/// The read is pure: it does not sweep expired reservations. A reservation
/// past its TTL stays reserved in the ledger until it is committed or
/// released — `expire_stale` exists for the Order service's compensation
/// queue, and nothing schedules it here — so a read must not change the
/// ledger it reports on.
async fn list_inventory(
    State(store): State<Arc<dyn InventoryRepository>>,
    Query(q): Query<ListQuery>,
) -> Result<Json<StockPage>, AppError> {
    let limit = q.limit.unwrap_or(20).clamp(1, 100) as usize;
    let offset = q
        .cursor
        .as_deref()
        .and_then(|c| c.parse::<usize>().ok())
        .unwrap_or(0);

    Ok(Json(store.list(offset, limit).await?))
}

async fn reserve(
    State(store): State<Arc<dyn InventoryRepository>>,
    JsonBody(body): JsonBody<ReserveBody>,
) -> Result<Json<Reservation>, AppError> {
    Ok(Json(
        store
            .reserve(&body.sku, &body.order_id, body.quantity)
            .await?,
    ))
}

async fn commit(
    State(store): State<Arc<dyn InventoryRepository>>,
    axum::extract::Path(p): axum::extract::Path<ReservationPath>,
) -> Result<Json<Reservation>, AppError> {
    Ok(Json(store.commit(&p.reservation_id).await?))
}

async fn release(
    State(store): State<Arc<dyn InventoryRepository>>,
    axum::extract::Path(p): axum::extract::Path<ReservationPath>,
) -> Result<Json<Reservation>, AppError> {
    Ok(Json(store.release(&p.reservation_id).await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    /// `GET /v1/inventory` and parse the answer.
    async fn get_json(app: &Router, uri: &str) -> serde_json::Value {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .expect("the route must answer");
        assert_eq!(response.status(), StatusCode::OK, "{uri} must answer 200");
        let body = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&body).expect("json body")
    }

    /// The list route must serve the levels the store really holds, in the
    /// wire shape the contract names.
    ///
    /// The admin facade proxies `GET /admin/v1/inventory` to this route, so
    /// while the route did not exist the admin route answered 404 and the
    /// console had no stock view at all.
    #[tokio::test]
    async fn the_list_route_serves_the_stock_levels() {
        let store: SharedInventory = Arc::new(RwLock::new(InventoryStore::new(DEMO_STOCK_SEED)));
        let app = router(Arc::new(MemoryInventoryRepository::new(store)));

        let json = get_json(&app, "/v1/inventory").await;

        let items = json["items"].as_array().expect("items array");
        assert_eq!(items.len(), DEMO_STOCK_SEED.len());
        // Sorted by SKU, so the page is stable.
        assert_eq!(items[0]["sku"], "sku-adult-01");
        assert_eq!(items[0]["onHand"], 3);
        assert_eq!(items[0]["reserved"], 0);
        assert_eq!(items[0]["available"], 3);
        assert!(
            json["next_cursor"].is_null(),
            "one page holds the whole store"
        );
    }

    /// An open reservation must show up in the listing, and the availability
    /// must drop by exactly the reserved amount.
    #[tokio::test]
    async fn the_list_route_reports_the_open_reservations() {
        let store: SharedInventory = Arc::new(RwLock::new(InventoryStore::new(&[("sku-a", 10)])));
        {
            let mut guard = store.write().unwrap();
            guard.reserve("sku-a", "order-1", 3).unwrap();
        }
        let app = router(Arc::new(MemoryInventoryRepository::new(store)));

        let json = get_json(&app, "/v1/inventory").await;

        assert_eq!(json["items"][0]["onHand"], 10);
        assert_eq!(json["items"][0]["reserved"], 3);
        assert_eq!(json["items"][0]["available"], 7);
    }

    /// The listing pages by cursor, so a caller can walk a store that is
    /// larger than one page.
    #[tokio::test]
    async fn the_list_route_pages_by_cursor() {
        let store: SharedInventory = Arc::new(RwLock::new(InventoryStore::new(&[
            ("sku-a", 1),
            ("sku-b", 2),
            ("sku-c", 3),
        ])));
        let app = router(Arc::new(MemoryInventoryRepository::new(store)));

        let first = get_json(&app, "/v1/inventory?limit=1").await;
        assert_eq!(first["items"].as_array().unwrap().len(), 1);
        assert_eq!(first["items"][0]["sku"], "sku-a");
        assert_eq!(first["next_cursor"], "1");

        let second = get_json(&app, "/v1/inventory?limit=1&cursor=1").await;
        assert_eq!(second["items"][0]["sku"], "sku-b");
        assert_eq!(second["next_cursor"], "2");

        let last = get_json(&app, "/v1/inventory?limit=1&cursor=2").await;
        assert_eq!(last["items"][0]["sku"], "sku-c");
        assert!(last["next_cursor"].is_null(), "the last page ends the walk");
    }

    /// The published SKUs must be the catalogue's SKUs.
    ///
    /// The list route publishes the store's SKUs, so a seed that had drifted
    /// from the catalogue's would show an operator a SKU that no product has
    /// — and a product that the catalogue sells but the inventory does not
    /// know would fail at the last step of the checkout.
    #[test]
    fn the_demo_stock_seed_matches_the_catalogue_seed() {
        let catalogue = commerce_catalog::CatalogStore::seed_demo();
        let inventory = InventoryStore::new(DEMO_STOCK_SEED);

        let catalogue_levels: Vec<(&str, i64)> = catalogue
            .stock
            .iter()
            .map(|(sku, quantity)| (sku.as_str(), *quantity))
            .collect();
        let inventory_levels: Vec<(&str, i64)> = inventory
            .stock
            .iter()
            .map(|(sku, level)| (sku.as_str(), level.on_hand))
            .collect();

        let mut catalogue_levels = catalogue_levels;
        catalogue_levels.sort_by(|a, b| a.0.cmp(b.0));
        let mut inventory_levels = inventory_levels;
        inventory_levels.sort_by(|a, b| a.0.cmp(b.0));

        assert_eq!(
            inventory_levels
                .iter()
                .map(|(sku, _)| *sku)
                .collect::<Vec<_>>(),
            catalogue_levels
                .iter()
                .map(|(sku, _)| *sku)
                .collect::<Vec<_>>(),
            "the inventory and the catalogue must know the same SKUs"
        );
        assert_eq!(
            inventory_levels, catalogue_levels,
            "the inventory and the catalogue must agree on the quantities"
        );
    }

    #[test]
    fn reserve_then_commit_reduces_both_on_hand_and_reserved() {
        let mut s = InventoryStore::new(&[("sku-a", 10)]);
        let r = s.reserve("sku-a", "order-1", 3).unwrap();
        assert_eq!(s.available("sku-a"), Some(7));
        s.commit(&r.reservation_id).unwrap();
        assert_eq!(
            s.available("sku-a"),
            Some(7),
            "available unchanged after commit"
        );
        assert_eq!(s.stock["sku-a"].on_hand, 7);
        assert_eq!(s.stock["sku-a"].reserved, 0);
        assert!(s.invariants_hold());
    }

    #[test]
    fn reserve_then_release_restores_availability() {
        let mut s = InventoryStore::new(&[("sku-a", 10)]);
        let r = s.reserve("sku-a", "order-1", 4).unwrap();
        assert_eq!(s.available("sku-a"), Some(6));
        s.release(&r.reservation_id).unwrap();
        assert_eq!(s.available("sku-a"), Some(10));
        assert_eq!(s.stock["sku-a"].on_hand, 10);
        assert!(s.invariants_hold());
    }

    #[test]
    fn availability_never_goes_negative() {
        let mut s = InventoryStore::new(&[("sku-a", 1)]);
        let err = s.reserve("sku-a", "order-1", 2).unwrap_err();
        assert!(matches!(err, AppError::Problem(p) if p.code == "inventory.insufficient"));
        assert_eq!(s.available("sku-a"), Some(1));
        assert!(s.invariants_hold());
    }

    #[test]
    fn concurrent_last_item_only_one_winner() {
        let mut s = InventoryStore::new(&[("sku-last", 1)]);
        let first = s.reserve("sku-last", "order-A", 1);
        let second = s.reserve("sku-last", "order-B", 1);
        assert!(first.is_ok());
        assert!(second.is_err(), "second buyer must lose");
        assert!(s.invariants_hold());
    }

    #[test]
    fn double_reserve_for_same_order_is_idempotent() {
        let mut s = InventoryStore::new(&[("sku-a", 10)]);
        let a = s.reserve("sku-a", "order-1", 2).unwrap();
        let b = s.reserve("sku-a", "order-1", 2).unwrap();
        assert_eq!(a.reservation_id, b.reservation_id);
        assert_eq!(s.stock["sku-a"].reserved, 2, "must not double-decrement");
    }

    #[test]
    fn terminal_reservation_cannot_be_released_or_committed_twice() {
        let mut s = InventoryStore::new(&[("sku-a", 10)]);
        let r = s.reserve("sku-a", "order-1", 2).unwrap();
        s.commit(&r.reservation_id).unwrap();
        assert!(s.commit(&r.reservation_id).is_err());
        assert!(s.release(&r.reservation_id).is_err());
        assert!(s.invariants_hold());
    }

    #[test]
    fn ttl_expiry_releases_instead_of_committing() {
        let mut s = InventoryStore::new(&[("sku-a", 10)]);
        let r = s.reserve("sku-a", "order-1", 5).unwrap();
        assert_eq!(s.available("sku-a"), Some(5));
        let now = r.expires_at_ms + 1;
        let expired = s.expire_stale(now);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].state, ReservationState::Released);
        assert_eq!(s.available("sku-a"), Some(10));
        assert!(s.invariants_hold());
    }

    #[test]
    fn ttl_sweep_is_idempotent() {
        let mut s = InventoryStore::new(&[("sku-a", 10)]);
        let r = s.reserve("sku-a", "order-1", 5).unwrap();
        let now = r.expires_at_ms + 1;
        assert_eq!(s.expire_stale(now).len(), 1);
        assert_eq!(
            s.expire_stale(now).len(),
            0,
            "second sweep must find nothing"
        );
        assert_eq!(s.available("sku-a"), Some(10));
    }

    #[test]
    fn unknown_sku_and_bad_quantity_are_rejected() {
        let mut s = InventoryStore::new(&[("sku-a", 10)]);
        assert!(s.reserve("sku-ghost", "o", 1).is_err());
        assert!(s.reserve("sku-a", "o", 0).is_err());
        assert!(s.reserve("sku-a", "o", -3).is_err());
    }
}
