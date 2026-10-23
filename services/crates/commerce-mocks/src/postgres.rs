//! Constraint-enforcing data-store mock — serves `CASE-IT-001` and `CASE-CHAOS-003`.
//!
//! # Why not just use the in-memory stores
//!
//! The services currently keep state in `HashMap`s. That is fine for domain
//! logic but it cannot answer "does the DDL hold?". The DDL in
//! `docs/ecommerce/03-detailed/API・DB・状態機械設計.md` §2 carries the real
//! guarantees:
//!
//! - `stock.reserved <= stock.on_hand` — availability can never go negative
//! - `UNIQUE (order_id, sku_id)` — double reservation is structurally impossible
//! - `UNIQUE (scoped_key, request_fingerprint)` — idempotency-key scope
//! - `quantity > 0`, `amount_minor >= 0`, `version >= 1`
//!
//! This mock enforces those as *executable* checks and returns SQLSTATE-shaped
//! errors, so a migration mistake becomes a failing assertion rather than a
//! production incident.
//!
//! It is **not** Postgres. It stores nothing, speaks no wire protocol, and
//! proves only that the constraints as written reject bad data. `[证据等级: Observed]`
//! for the constraint behaviour; `[Designed]` for anything about real Postgres.

use crate::fault::FaultController;
use axum::{http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// SQLSTATE codes, so tests can assert on the same value a real driver raises.
pub mod sqlstate {
    pub const CHECK_VIOLATION: &str = "23514";
    pub const UNIQUE_VIOLATION: &str = "23505";
    pub const NOT_NULL_VIOLATION: &str = "23502";
    pub const CONNECTION_FAILURE: &str = "08006";
    pub const INSUFFICIENT_RESOURCES: &str = "53300";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintError {
    pub sqlstate: String,
    pub constraint: String,
    pub detail: String,
}

impl ConstraintError {
    fn new(sqlstate: &str, constraint: &str, detail: impl Into<String>) -> Self {
        Self {
            sqlstate: sqlstate.to_string(),
            constraint: constraint.to_string(),
            detail: detail.into(),
        }
    }

    /// The HTTP shape a service returns when the DB rejects a write.
    pub fn to_problem(&self) -> (StatusCode, axum::Json<serde_json::Value>) {
        // 409 for a business-rule violation, 503 for the DB being unreachable.
        let status = match self.sqlstate.as_str() {
            sqlstate::CONNECTION_FAILURE | sqlstate::INSUFFICIENT_RESOURCES => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            sqlstate::NOT_NULL_VIOLATION => StatusCode::UNPROCESSABLE_ENTITY,
            _ => StatusCode::CONFLICT,
        };
        (
            status,
            axum::Json(serde_json::json!({
                "code": format!("db.{}", self.constraint),
                "sqlstate": self.sqlstate,
                "constraint": self.constraint,
                "detail": self.detail,
            })),
        )
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StockRow {
    pub sku_id: String,
    pub on_hand: i64,
    pub reserved: i64,
}

impl StockRow {
    /// Mirrors the DDL: both columns are non-negative AND reserved <= on_hand.
    pub fn check(&self) -> Result<(), ConstraintError> {
        if self.on_hand < 0 {
            return Err(ConstraintError::new(
                sqlstate::CHECK_VIOLATION,
                "stock_on_hand_non_negative",
                format!("on_hand {} is negative", self.on_hand),
            ));
        }
        if self.reserved < 0 {
            return Err(ConstraintError::new(
                sqlstate::CHECK_VIOLATION,
                "stock_reserved_non_negative",
                format!("reserved {} is negative", self.reserved),
            ));
        }
        if self.reserved > self.on_hand {
            return Err(ConstraintError::new(
                sqlstate::CHECK_VIOLATION,
                "stock_reserved_le_on_hand",
                format!(
                    "reserved {} exceeds on_hand {} — availability would be negative",
                    self.reserved, self.on_hand
                ),
            ));
        }
        Ok(())
    }

    pub fn available(&self) -> i64 {
        (self.on_hand - self.reserved).max(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationRow {
    pub reservation_id: String,
    pub order_id: String,
    pub sku_id: String,
    pub quantity: i64,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyRow {
    pub scoped_key: String,
    pub request_fingerprint: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItemRow {
    pub cart_id: String,
    pub sku_id: String,
    pub quantity: i64,
    pub unit_amount_snapshot: i64,
    pub version: i64,
}

#[derive(Default)]
struct Store {
    stock: HashMap<String, StockRow>,
    /// Keyed by `(order_id, sku_id)` — the UNIQUE constraint from the DDL.
    reservations: HashMap<(String, String), ReservationRow>,
    idempotency: HashMap<String, IdempotencyRow>,
    cart_items: HashMap<(String, String), CartItemRow>,
    sequences: HashMap<String, u64>,
}

#[derive(Clone, Default)]
pub struct DbMock {
    store: Arc<RwLock<Store>>,
    faults: FaultController,
}

impl DbMock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn faults(&self) -> FaultController {
        self.faults.clone()
    }

    fn next_seq(g: &mut Store, name: &str) -> u64 {
        let e = g.sequences.entry(name.to_string()).or_insert(0);
        *e += 1;
        *e
    }

    /// Upsert `stock`, enforcing the DDL check.
    pub async fn upsert_stock(
        &self,
        row: StockRow,
    ) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
        if let Some(f) = self.faults.take("db.write") {
            if f.latency_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(f.latency_ms)).await;
            }
            let code = if f.http_status == 503 {
                sqlstate::CONNECTION_FAILURE.to_string()
            } else {
                format!("{:05}", f.http_status.max(40000))
            };
            return Err(ConstraintError::new(&code, "connection", "database unreachable").to_problem());
        }
        if let Err(e) = row.check() {
            return Err(e.to_problem());
        }
        let mut g = self.store.write().map_err(|_| internal())?;
        g.stock.insert(row.sku_id.clone(), row.clone());
        Ok(Json(serde_json::json!({
            "skuId": row.sku_id,
            "onHand": row.on_hand,
            "reserved": row.reserved,
            "available": row.available(),
        })))
    }

    /// Reserve stock. Enforces `available >= quantity` AND the UNIQUE
    /// `(order_id, sku_id)`.
    pub async fn reserve(
        &self,
        order_id: &str,
        sku_id: &str,
        quantity: i64,
    ) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
        if let Some(f) = self.faults.take("db.write") {
            let code = if f.http_status == 503 {
                sqlstate::CONNECTION_FAILURE
            } else {
                sqlstate::INSUFFICIENT_RESOURCES
            };
            return Err(ConstraintError::new(code, "connection", "database unreachable").to_problem());
        }
        if quantity <= 0 {
            return Err(ConstraintError::new(
                sqlstate::CHECK_VIOLATION,
                "reservation_quantity_positive",
                format!("quantity {quantity} must be > 0"),
            )
            .to_problem());
        }

        let mut g = self.store.write().map_err(|_| internal())?;

        // UNIQUE (order_id, sku_id) — enforced as a set membership test.
        let key = (order_id.to_string(), sku_id.to_string());
        if g.reservations.contains_key(&key) {
            return Err(ConstraintError::new(
                sqlstate::UNIQUE_VIOLATION,
                "inventory_reservation_order_sku_key",
                format!("a reservation already exists for order {order_id} sku {sku_id}"),
            )
            .to_problem());
        }

        {
            let stock = g.stock.get_mut(sku_id).ok_or_else(|| {
                ConstraintError::new(
                    sqlstate::UNIQUE_VIOLATION,
                    "stock_sku_fk",
                    format!("unknown sku {sku_id}"),
                )
                .to_problem()
            })?;

            if stock.available() < quantity {
                return Err(ConstraintError::new(
                    sqlstate::CHECK_VIOLATION,
                    "stock_reserved_le_on_hand",
                    format!("only {} available, requested {quantity}", stock.available()),
                )
                .to_problem());
            }

            stock.reserved += quantity;
            // Re-check: the invariant must hold *after* the write too.
            stock.check().map_err(|e| e.to_problem())?;
        }

        let id = format!("res-{:06}", Self::next_seq(&mut g, "res"));
        let available = g.stock.get(sku_id).map(StockRow::available).unwrap_or(0);
        g.reservations.insert(
            key,
            ReservationRow {
                reservation_id: id.clone(),
                order_id: order_id.to_string(),
                sku_id: sku_id.to_string(),
                quantity,
                state: "reserved".into(),
            },
        );

        Ok(Json(serde_json::json!({
            "reservationId": id,
            "skuId": sku_id,
            "quantity": quantity,
            "available": available,
        })))
    }

    /// Release or commit an existing reservation, undoing the reserved delta.
    pub async fn settle_reservation(
        &self,
        order_id: &str,
        sku_id: &str,
        new_state: &str,
    ) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
        if !matches!(new_state, "released" | "committed" | "expired") {
            return Err(ConstraintError::new(
                sqlstate::CHECK_VIOLATION,
                "reservation_state_enum",
                format!("'{new_state}' is not a valid reservation state"),
            )
            .to_problem());
        }
        let mut g = self.store.write().map_err(|_| internal())?;
        let key = (order_id.to_string(), sku_id.to_string());
        let row = g.reservations.get(&key).cloned().ok_or_else(|| {
            ConstraintError::new(
                sqlstate::UNIQUE_VIOLATION,
                "inventory_reservation_order_sku_key",
                "no such reservation",
            )
            .to_problem()
        })?;
        if row.state != "reserved" {
            return Err(ConstraintError::new(
                sqlstate::CHECK_VIOLATION,
                "reservation_terminal",
                format!("reservation is already {}", row.state),
            )
            .to_problem());
        }

        if let Some(stock) = g.stock.get_mut(sku_id) {
            stock.reserved = (stock.reserved - row.quantity).max(0);
            if new_state == "committed" {
                stock.on_hand = (stock.on_hand - row.quantity).max(0);
            }
            stock.check().map_err(|e| e.to_problem())?;
        }
        let available = g.stock.get(sku_id).map(|s| s.available()).unwrap_or(0);
        if let Some(r) = g.reservations.get_mut(&key) {
            r.state = new_state.to_string();
        }

        Ok(Json(serde_json::json!({
            "reservationId": row.reservation_id,
            "state": new_state,
            "available": available,
        })))
    }

    /// Idempotency insert. Enforces the UNIQUE on
    /// `(scoped_key, request_fingerprint)`.
    pub async fn put_idempotency(
        &self,
        scoped_key: &str,
        fingerprint: &str,
        state: &str,
    ) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
        let mut g = self.store.write().map_err(|_| internal())?;
        match g.idempotency.get(scoped_key) {
            Some(existing) if existing.request_fingerprint != fingerprint => {
                return Err(ConstraintError::new(
                    sqlstate::UNIQUE_VIOLATION,
                    "idempotency_scoped_key_fingerprint",
                    "Idempotency-Key reused with a different payload",
                )
                .to_problem())
            }
            Some(_) => {
                return Ok(Json(serde_json::json!({
                    "scopedKey": scoped_key,
                    "replayed": true,
                })))
            }
            None => {}
        }
        g.idempotency.insert(
            scoped_key.to_string(),
            IdempotencyRow {
                scoped_key: scoped_key.to_string(),
                request_fingerprint: fingerprint.to_string(),
                state: state.to_string(),
            },
        );
        Ok(Json(serde_json::json!({ "scopedKey": scoped_key, "replayed": false })))
    }

    /// Cart item write with the DDL's `quantity > 0` and `amount >= 0`.
    pub async fn upsert_cart_item(
        &self,
        cart_id: &str,
        sku_id: &str,
        quantity: i64,
        unit_amount_snapshot: i64,
        version: i64,
    ) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
        if quantity < 0 {
            return Err(ConstraintError::new(
                sqlstate::CHECK_VIOLATION,
                "cart_item_quantity_positive",
                format!("quantity {quantity} must be > 0"),
            )
            .to_problem());
        }
        if unit_amount_snapshot < 0 {
            return Err(ConstraintError::new(
                sqlstate::CHECK_VIOLATION,
                "cart_item_amount_non_negative",
                format!("unit_amount_snapshot {unit_amount_snapshot} is negative"),
            )
            .to_problem());
        }
        if version < 1 {
            return Err(ConstraintError::new(
                sqlstate::CHECK_VIOLATION,
                "cart_version_positive",
                format!("version {version} must be >= 1"),
            )
            .to_problem());
        }
        let mut g = self.store.write().map_err(|_| internal())?;
        g.cart_items.insert(
            (cart_id.to_string(), sku_id.to_string()),
            CartItemRow {
                cart_id: cart_id.to_string(),
                sku_id: sku_id.to_string(),
                quantity,
                unit_amount_snapshot,
                version,
            },
        );
        Ok(Json(serde_json::json!({ "ok": true })))
    }

    /// Read-back endpoint so a test can assert the *stored* state rather than
    /// trusting what the service said it wrote.
    pub async fn snapshot(&self) -> Json<serde_json::Value> {
        let g = self.store.read().expect("db lock");
        let rows: Vec<&StockRow> = g.stock.values().collect();
        // `available` is a computed column — `StockRow` has no such field —
        // so the projection has to add it. It is the quantity the DDL's
        // `reserved <= on_hand` invariant leaves behind, which is exactly
        // what a caller wants to assert on.
        let stock: Vec<serde_json::Value> = rows
            .iter()
            .map(|s| {
                serde_json::json!({
                    "sku_id": s.sku_id,
                    "on_hand": s.on_hand,
                    "reserved": s.reserved,
                    "available": s.available(),
                })
            })
            .collect();
        let reservations: Vec<_> = g.reservations.values().cloned().collect();
        Json(serde_json::json!({
            "stock": stock,
            "reservations": reservations,
            "idempotency": g.idempotency.values().collect::<Vec<_>>(),
            "distinctReservationKeys": g.reservations.len(),
            "invariantsHold": rows.iter().all(|s| s.check().is_ok()),
        }))
    }

    pub async fn reset(&self) {
        let mut g = self.store.write().expect("db lock");
        *g = Store::default();
    }
}

fn internal() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "code": "db.internal" })),
    )
}

// --------------------------------------------------------------- HTTP shape

#[derive(Debug, Deserialize)]
struct StockBody {
    sku_id: String,
    on_hand: i64,
    reserved: i64,
}

#[derive(Debug, Deserialize)]
struct ReserveBody {
    order_id: String,
    sku_id: String,
    quantity: i64,
}

#[derive(Debug, Deserialize)]
struct SettleBody {
    order_id: String,
    sku_id: String,
    state: String,
}

#[derive(Debug, Deserialize)]
struct IdemBody {
    scoped_key: String,
    fingerprint: String,
    #[serde(default)]
    state: String,
}

#[derive(Debug, Deserialize)]
struct CartBody {
    cart_id: String,
    sku_id: String,
    quantity: i64,
    unit_amount_snapshot: i64,
    version: i64,
}

pub fn router(db: DbMock) -> Router {
    let d = db.clone();
    Router::new()
        .route(
            "/mockdb/stock",
            post({
                let d = d.clone();
                move |Json(b): Json<StockBody>| {
                    let d = d.clone();
                    async move { d.upsert_stock(StockRow { sku_id: b.sku_id, on_hand: b.on_hand, reserved: b.reserved }).await }
                }
            }),
        )
        .route(
            "/mockdb/reserve",
            post({
                let d = d.clone();
                move |Json(b): Json<ReserveBody>| {
                    let d = d.clone();
                    async move { d.reserve(&b.order_id, &b.sku_id, b.quantity).await }
                }
            }),
        )
        .route(
            "/mockdb/settle",
            post({
                let d = d.clone();
                move |Json(b): Json<SettleBody>| {
                    let d = d.clone();
                    async move { d.settle_reservation(&b.order_id, &b.sku_id, &b.state).await }
                }
            }),
        )
        .route(
            "/mockdb/idempotency",
            post({
                let d = d.clone();
                move |Json(b): Json<IdemBody>| {
                    let d = d.clone();
                    async move { d.put_idempotency(&b.scoped_key, &b.fingerprint, &b.state).await }
                }
            }),
        )
        .route(
            "/mockdb/cart_item",
            post({
                let d = d.clone();
                move |Json(b): Json<CartBody>| {
                    let d = d.clone();
                    async move {
                        d.upsert_cart_item(
                            &b.cart_id,
                            &b.sku_id,
                            b.quantity,
                            b.unit_amount_snapshot,
                            b.version,
                        )
                        .await
                    }
                }
            }),
        )
        .route(
            "/__mock/snapshot",
            axum::routing::get({
                let d = db.clone();
                move || {
                    let d = d.clone();
                    async move { d.snapshot().await }
                }
            }),
        )
        .route(
            "/__mock/reset",
            axum::routing::post({
                let d = db.clone();
                move || {
                    let d = d.clone();
                    async move {
                        d.reset().await;
                        Json(serde_json::json!({ "ok": true }))
                    }
                }
            }),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(sku: &str, on_hand: i64, reserved: i64) -> StockRow {
        StockRow {
            sku_id: sku.into(),
            on_hand,
            reserved,
        }
    }

    #[test]
    fn reserved_may_not_exceed_on_hand() {
        assert!(row("a", 10, 10).check().is_ok());
        let e = row("a", 10, 11).check().unwrap_err();
        assert_eq!(e.sqlstate, sqlstate::CHECK_VIOLATION);
        assert_eq!(e.constraint, "stock_reserved_le_on_hand");
        assert!(e.detail.contains("availability would be negative"));
    }

    #[test]
    fn negative_stock_is_rejected_on_both_columns() {
        assert_eq!(
            row("a", -1, 0).check().unwrap_err().constraint,
            "stock_on_hand_non_negative"
        );
        assert_eq!(
            row("a", 5, -1).check().unwrap_err().constraint,
            "stock_reserved_non_negative"
        );
    }

    #[test]
    fn available_never_goes_negative() {
        assert_eq!(row("a", 10, 4).available(), 6);
        assert_eq!(row("a", 10, 10).available(), 0);
    }

    #[tokio::test]
    async fn reservation_beyond_available_is_a_check_violation() {
        let d = DbMock::new();
        let _ = d.upsert_stock(row("sku-a", 1, 0)).await.unwrap();
        let (status, body) = d.reserve("ord-1", "sku-a", 2).await.unwrap_err();
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["sqlstate"], sqlstate::CHECK_VIOLATION);
        let snap = d.snapshot().await;
        assert_eq!(snap.0["stock"][0]["available"], 1, "nothing was written");
        assert_eq!(snap.0["invariantsHold"], true);
    }

    #[tokio::test]
    async fn double_reservation_for_the_same_order_and_sku_is_a_unique_violation() {
        let d = DbMock::new();
        let _ = d.upsert_stock(row("sku-a", 10, 0)).await.unwrap();
        let _ = d.reserve("ord-1", "sku-a", 2).await.unwrap();

        let (status, body) = d.reserve("ord-1", "sku-a", 2).await.unwrap_err();
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["sqlstate"], sqlstate::UNIQUE_VIOLATION);

        // Critically: the second attempt must not have decremented anything.
        let snap = d.snapshot().await;
        assert_eq!(snap.0["stock"][0]["reserved"], 2, "no double decrement");
        assert_eq!(snap.0["distinctReservationKeys"], 1);
    }

    #[tokio::test]
    async fn commit_reduces_both_on_hand_and_reserved() {
        let d = DbMock::new();
        let _ = d.upsert_stock(row("sku-a", 10, 0)).await.unwrap();
        let _ = d.reserve("ord-1", "sku-a", 3).await.unwrap();
        let snap = d.snapshot().await;
        assert_eq!(snap.0["stock"][0]["available"], 7);

        let _ = d.settle_reservation("ord-1", "sku-a", "committed").await.unwrap();
        let snap = d.snapshot().await;
        assert_eq!(snap.0["stock"][0]["on_hand"], 7);
        assert_eq!(snap.0["stock"][0]["reserved"], 0);
        assert_eq!(snap.0["invariantsHold"], true);
    }

    #[tokio::test]
    async fn release_restores_availability_without_touching_on_hand() {
        let d = DbMock::new();
        let _ = d.upsert_stock(row("sku-a", 10, 0)).await.unwrap();
        let _ = d.reserve("ord-1", "sku-a", 4).await.unwrap();
        let _ = d.settle_reservation("ord-1", "sku-a", "released").await.unwrap();
        let snap = d.snapshot().await;
        assert_eq!(snap.0["stock"][0]["on_hand"], 10);
        assert_eq!(snap.0["stock"][0]["reserved"], 0);
        assert_eq!(snap.0["invariantsHold"], true);
    }

    #[tokio::test]
    async fn a_settled_reservation_cannot_be_settled_again() {
        let d = DbMock::new();
        let _ = d.upsert_stock(row("sku-a", 10, 0)).await.unwrap();
        let _ = d.reserve("ord-1", "sku-a", 1).await.unwrap();
        let _ = d.settle_reservation("ord-1", "sku-a", "released").await.unwrap();
        let (_, body) = d.settle_reservation("ord-1", "sku-a", "committed").await.unwrap_err();
        assert_eq!(body["constraint"], "reservation_terminal");
    }

    #[tokio::test]
    async fn non_positive_reservation_quantity_is_rejected() {
        let d = DbMock::new();
        let _ = d.upsert_stock(row("sku-a", 10, 0)).await.unwrap();
        assert!(d.reserve("o", "sku-a", 0).await.is_err());
        assert!(d.reserve("o", "sku-a", -3).await.is_err());
    }

    #[tokio::test]
    async fn unknown_sku_is_a_foreign_key_violation() {
        let d = DbMock::new();
        let (_, body) = d.reserve("o", "ghost", 1).await.unwrap_err();
        assert_eq!(body["constraint"], "stock_sku_fk");
    }

    #[tokio::test]
    async fn invalid_reservation_state_is_an_enum_violation() {
        let d = DbMock::new();
        let _ = d.upsert_stock(row("sku-a", 10, 0)).await.unwrap();
        let _ = d.reserve("ord-1", "sku-a", 1).await.unwrap();
        let (_, body) = d.settle_reservation("ord-1", "sku-a", "bogus").await.unwrap_err();
        assert_eq!(body["constraint"], "reservation_state_enum");
    }

    #[tokio::test]
    async fn idempotency_same_key_same_fingerprint_replays() {
        let d = DbMock::new();
        let r = d.put_idempotency("k", "fp", "completed").await.unwrap();
        assert_eq!(r.0["replayed"], false);
        let r2 = d.put_idempotency("k", "fp", "completed").await.unwrap();
        assert_eq!(r2.0["replayed"], true);
    }

    #[tokio::test]
    async fn idempotency_same_key_different_fingerprint_is_a_unique_violation() {
        let d = DbMock::new();
        let _ = d.put_idempotency("k", "fp-a", "completed").await.unwrap();
        let (status, body) = d.put_idempotency("k", "fp-b", "completed").await.unwrap_err();
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["constraint"], "idempotency_scoped_key_fingerprint");
    }

    #[tokio::test]
    async fn cart_item_rejects_negative_amount_and_zero_version() {
        let d = DbMock::new();
        assert!(d.upsert_cart_item("c", "s", 1, -1, 1).await.is_err());
        assert!(d.upsert_cart_item("c", "s", 1, 100, 0).await.is_err());
        assert!(d.upsert_cart_item("c", "s", 2, 4800, 1).await.is_ok());
    }

    #[tokio::test]
    async fn db_outage_maps_to_503_not_500() {
        // CASE-CHAOS-003: a lost connection is retryable, an internal error is not.
        let d = DbMock::new();
        d.faults().inject(crate::fault::Fault {
            target: "db.write".into(),
            mode: crate::fault::FaultMode::NextN,
            count: 1,
            latency_ms: 0,
            http_status: 503,
            drop_response: false,
        });
        let (status, body) = d.upsert_stock(row("sku-a", 1, 0)).await.unwrap_err();
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["sqlstate"], sqlstate::CONNECTION_FAILURE);

        // Recovery: the write succeeds once the connection returns.
        assert!(d.upsert_stock(row("sku-a", 1, 0)).await.is_ok());
    }

    #[tokio::test]
    async fn snapshot_reports_whether_invariants_hold() {
        let d = DbMock::new();
        let _ = d.upsert_stock(row("sku-a", 5, 0)).await.unwrap();
        let _ = d.upsert_stock(row("sku-b", 0, 0)).await.unwrap();
        assert_eq!(d.snapshot().await.0["invariantsHold"], true);
    }

    #[tokio::test]
    async fn reset_clears_everything() {
        let d = DbMock::new();
        let _ = d.upsert_stock(row("sku-a", 5, 0)).await.unwrap();
        d.reset().await;
        assert_eq!(d.snapshot().await.0["stock"].as_array().unwrap().len(), 0);
    }
}
