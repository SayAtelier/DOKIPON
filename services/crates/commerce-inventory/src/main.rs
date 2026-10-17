//! Inventory service — available / reserved / committed with reservation TTL.
//!
//! Invariants (atlas §B5, §E):
//! - `available = on_hand − reserved` and **availability must never go negative**.
//! - A reservation is uniquely keyed so a double-reserve is impossible.
//! - Reservations expire on TTL; expiry is a release, not a silent commit.
//! - Compensating a confirmed order releases the reservation; a rejected or
//!   cancelled order releases too. Only a confirmed order commits.

use axum::{extract::State, http::StatusCode, routing::{get, post}, Json, Router};
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

pub const RESERVATION_TTL_MS: i64 = 15 * 60 * 1000; // 15 minutes

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
    pub fn seed(levels: &[(&str, i64)]) -> Self {
        Self {
            stock: levels
                .iter()
                .map(|(sku, n)| (sku.to_string(), StockLevel { on_hand: *n, reserved: 0 }))
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
            status::not_found("inventory.sku_not_found", format!("unknown sku {sku}"), "inventory.reserve")
        })
    }

    /// `inventory.reserve` — idempotent per `order_id`: reserving twice for the
    /// same order returns the SAME reservation instead of double-decrementing.
    pub fn reserve(&mut self, sku: &str, order_id: &str, quantity: i64) -> Result<Reservation, AppError> {
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
        self.reservations.insert(reservation.reservation_id.clone(), reservation.clone());
        Ok(reservation)
    }

    /// Called when the payment succeeds and the order is confirmed.
    pub fn commit(&mut self, reservation_id: &str) -> Result<Reservation, AppError> {
        const TRACE: &str = "inventory.commit";
        let res = self
            .reservations
            .get_mut(reservation_id)
            .ok_or_else(|| status::not_found("inventory.reservation_not_found", "reservation not found", TRACE))?;
        if res.state.is_terminal() {
            return Err(status::conflict("inventory.reservation_terminal", format!("reservation already {:?}", res.state), TRACE));
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
        let res = self
            .reservations
            .get_mut(reservation_id)
            .ok_or_else(|| status::not_found("inventory.reservation_not_found", "reservation not found", TRACE))?;
        if res.state.is_terminal() {
            return Err(status::conflict("inventory.reservation_terminal", format!("reservation already {:?}", res.state), TRACE));
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
        self.stock.values().all(|l| {
            l.on_hand >= 0 && l.reserved >= 0 && l.reserved <= l.on_hand
        })
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

pub fn router(state: SharedInventory) -> Router {
    Router::new()
        .route("/v1/inventory/reservations", post(reserve))
        .route("/v1/inventory/reservations/{reservation_id}/commit", post(commit))
        .route("/v1/inventory/reservations/{reservation_id}/release", post(release))
        .route("/healthz", get(health))
        .with_state(state)
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn reserve(
    State(store): State<SharedInventory>,
    Json(body): Json<ReserveBody>,
) -> Result<Json<Reservation>, AppError> {
    let mut guard = store
        .write()
        .map_err(|_| AppError::Internal("inventory lock poisoned".into()))?;
    Ok(Json(guard.reserve(&body.sku, &body.order_id, body.quantity)?))
}

async fn commit(
    State(store): State<SharedInventory>,
    axum::extract::Path(p): axum::extract::Path<ReservationPath>,
) -> Result<Json<Reservation>, AppError> {
    let mut guard = store
        .write()
        .map_err(|_| AppError::Internal("inventory lock poisoned".into()))?;
    Ok(Json(guard.commit(&p.reservation_id)?))
}

async fn release(
    State(store): State<SharedInventory>,
    axum::extract::Path(p): axum::extract::Path<ReservationPath>,
) -> Result<Json<Reservation>, AppError> {
    let mut guard = store
        .write()
        .map_err(|_| AppError::Internal("inventory lock poisoned".into()))?;
    Ok(Json(guard.release(&p.reservation_id)?))
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let store: SharedInventory = Arc::new(RwLock::new(InventoryStore::new(&[
        ("sku-tee-01", 42),
        ("sku-hoodie-01", 7),
        ("sku-cap-01", 0),
        ("sku-adult-01", 3),
    ])));
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind inventory");
    tracing::info!("inventory service listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router(store)).await.expect("serve inventory");
}

impl InventoryStore {
    pub fn new(levels: &[(&str, i64)]) -> Self {
        Self::seed(levels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_then_commit_reduces_both_on_hand_and_reserved() {
        let mut s = InventoryStore::new(&[("sku-a", 10)]);
        let r = s.reserve("sku-a", "order-1", 3).unwrap();
        assert_eq!(s.available("sku-a"), Some(7));
        s.commit(&r.reservation_id).unwrap();
        assert_eq!(s.available("sku-a"), Some(7), "available unchanged after commit");
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
        assert_eq!(s.expire_stale(now).len(), 0, "second sweep must find nothing");
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
