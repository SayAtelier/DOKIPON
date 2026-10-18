//! Cart service — user or anonymous carts with optimistic concurrency.
//!
//! Atlas invariants implemented here:
//! - `cart.update` carries `expectedVersion`; a stale version is 409, never a silent overwrite.
//! - Mutations are idempotent (same key + same payload → replay).
//! - `unit_amount_snapshot` is recorded at add time for display, but **checkout
//!   recomputes all totals from the authoritative catalog price** (price tampering defence).

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, patch},
    Json, Router,
};
use commerce_core::{
    error::{status, AppError},
    Currency, Money,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

pub type SharedCart = Arc<CartStore>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CartItem {
    pub sku: String,
    pub quantity: u32,
    /// Display-only snapshot captured when the item was added.
    pub unit_amount_snapshot: i64,
    pub currency: Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cart {
    pub cart_id: String,
    /// `None` for anonymous carts. Authorization decisions are server-side only.
    pub owner_id: Option<String>,
    /// Monotonic optimistic-lock version. Every accepted mutation bumps it by 1.
    pub version: i64,
    pub items: Vec<CartItem>,
    pub expires_at_ms: i64,
}

impl Cart {
    /// Authoritative subtotal computed from **server-side unit prices** passed in.
    /// The client's own subtotal is never used.
    pub fn authoritative_subtotal(
        &self,
        price_of: &dyn Fn(&str) -> Option<Money>,
    ) -> Result<Money, AppError> {
        let mut total = None::<Money>;
        for item in &self.items {
            let unit = price_of(&item.sku).ok_or_else(|| {
                status::unprocessable(
                    "cart.price_unavailable",
                    format!("sku {} has no authoritative price", item.sku),
                    "cart.subtotal",
                )
            })?;
            let line = unit.checked_mul_rounded(item.quantity).map_err(|e| {
                AppError::Internal(format!("subtotal overflow: {e}"))
            })?;
            total = Some(match total {
                None => line,
                Some(acc) => acc.checked_add(line).map_err(|e| {
                    AppError::Internal(format!("subtotal overflow: {e}"))
                })?,
            });
        }
        Ok(total.unwrap_or_else(|| Money::zero(Currency::Jpy)))
    }

    fn touch(&mut self) {
        self.version += 1;
    }
}

#[derive(Debug, Clone, Default)]
pub struct CartStore {
    pub carts: HashMap<String, Cart>,
    /// Idempotency-Key → stored response for cart mutations.
    pub replays: HashMap<String, serde_json::Value>,
    seq: u64,
}

impl CartStore {
    pub fn new() -> Self {
        Self { carts: HashMap::new(), replays: HashMap::new(), seq: 0 }
    }

    pub fn next_id(&mut self) -> String {
        self.seq += 1;
        format!("cart-{:06}", self.seq)
    }

    /// Merge a new `expectedVersion` check. Returns the cart on success.
    pub fn apply_mutation(
        &mut self,
        cart_id: &str,
        sku: &str,
        new_quantity: u32,
        expected_version: i64,
        unit_price: Money,
    ) -> Result<Cart, AppError> {
        const TRACE: &str = "cart.update";
        let cart = self
            .carts
            .get_mut(cart_id)
            .ok_or_else(|| status::not_found("cart.not_found", "cart not found", TRACE))?;

        if cart.version != expected_version {
            return Err(status::conflict(
                "cart.version_conflict",
                format!(
                    "expected version {} but cart is at {}",
                    expected_version, cart.version
                ),
                TRACE,
            ));
        }

        let unit_minor = unit_price.amount_minor;
        let currency = unit_price.currency;

        if let Some(item) = cart.items.iter_mut().find(|i| &i.sku == sku) {
            if new_quantity == 0 {
                cart.items.retain(|i| &i.sku != sku);
            } else {
                item.quantity = new_quantity;
                item.unit_amount_snapshot = unit_minor;
                item.currency = currency;
            }
        } else if new_quantity > 0 {
            cart.items.push(CartItem {
                sku: sku.to_string(),
                quantity: new_quantity,
                unit_amount_snapshot: unit_minor,
                currency,
            });
        }

        if cart.items.is_empty() {
            cart.expires_at_ms = commerce_core::idempotency::now_ms() + 30 * 24 * 3600 * 1000;
        }
        cart.touch();
        Ok(cart.clone())
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateItemBody {
    pub quantity: u32,
    pub expected_version: i64,
    /// Optional display-only hint from the client. **Ignored for money math.**
    pub client_unit_amount_minor: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CartPath {
    pub cart_id: String,
    pub sku: String,
}

pub fn router(state: SharedCart) -> Router {
    Router::new()
        .route("/v1/carts/{cart_id}", get(get_cart))
        .route("/v1/carts/{cart_id}/items/{sku}", patch(update_item))
        .route("/healthz", get(health))
        .with_state(state)
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn get_cart(
    State(store): State<SharedCart>,
    Path(p): Path<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Json<Cart>, AppError> {
    const TRACE: &str = "cart.get";
    let cart_id = p.get("cart_id").ok_or_else(|| status::bad_request("cart.missing_id", "cartId required", TRACE))?;
    let cart = store
        .carts
        .get(cart_id)
        .ok_or_else(|| status::not_found("cart.not_found", "cart not found", TRACE))?;

    // Server-side ownership check. A client-side session flag is never trusted.
    if let Some(owner) = &cart.owner_id {
        let presented = session_of(&headers);
        if presented.as_deref() != Some(owner.as_str()) {
            return Err(status::forbidden("cart.forbidden", "cart belongs to another session", TRACE));
        }
    }
    Ok(Json(cart.clone()))
}

fn session_of(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

async fn update_item(
    State(mut store): State<SharedCart>,
    Path(p): Path<CartPath>,
    headers: HeaderMap,
    Json(body): Json<UpdateItemBody>,
) -> Result<Json<Cart>, AppError> {
    const TRACE: &str = "cart.update";

    if body.expected_version < 1 {
        return Err(status::unprocessable(
            "cart.bad_version",
            "expectedVersion must be >= 1",
            TRACE,
        ));
    }
    if body.quantity > 99 {
        return Err(status::unprocessable(
            "cart.quantity_limit",
            "quantity per line must be <= 99",
            TRACE,
        ));
    }

    let key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let fingerprint = commerce_core::idempotency::fingerprint(&[
        &p.cart_id,
        &p.sku,
        &body.quantity.to_string(),
        &body.expected_version.to_string(),
    ]);

    if let Some(k) = &key {
        if let Some(cached) = store.replays.get(k) {
            if cached.get("_fp").and_then(|v| v.as_str()) == Some(fingerprint.as_str()) {
                return Ok(Json(serde_json::from_value(cached.clone()).map_err(|e| {
                    AppError::Internal(format!("replay decode: {e}"))
                })?));
            }
            return Err(status::conflict(
                "cart.idempotency_key_reuse",
                "Idempotency-Key reused with a different payload",
                TRACE,
            ));
        }
    }

    // Unit price is supplied by the caller (BFF reads it from the catalog).
    // If the caller omitted it, fall back to the snapshot — checkout still
    // recomputes authoritatively.
    let unit_price = Money::new(
        body.client_unit_amount_minor.unwrap_or_else(|| {
            store
                .carts
                .get(&p.cart_id)
                .and_then(|c| c.items.iter().find(|i| i.sku == p.sku))
                .map(|i| i.unit_amount_snapshot)
                .unwrap_or(0)
        }),
        Currency::Jpy,
    )
    .map_err(|_| status::unprocessable("cart.bad_price", "unit price must be >= 0", TRACE))?;

    let updated = {
        let store = Arc::make_mut(&mut store);
        store.apply_mutation(
            &p.cart_id,
            &p.sku,
            body.quantity,
            body.expected_version,
            unit_price,
        )?
    };

    if let Some(k) = key {
        let mut cached = serde_json::to_value(&updated).unwrap_or(serde_json::Value::Null);
        if let Some(obj) = cached.as_object_mut() {
            obj.insert("_fp".into(), serde_json::Value::String(fingerprint));
        }
        Arc::make_mut(&mut store).replays.insert(k, cached);
    }
    Ok(Json(updated))
}

/// Shared main so `cargo run -p commerce-cart` works standalone.
#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let store: SharedCart = Arc::new(CartStore::new());
    let app = router(store);
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind cart");
    tracing::info!("cart service listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.expect("serve cart");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with_cart() -> CartStore {
        let mut s = CartStore::new();
        let id = s.next_id();
        s.carts.insert(
            id.clone(),
            Cart { cart_id: id, owner_id: None, version: 1, items: vec![], expires_at_ms: 0 },
        );
        s
    }

    #[test]
    fn mutation_bumps_version_and_sets_snapshot() {
        let mut s = store_with_cart();
        let id = s.carts.keys().next().unwrap().clone();
        let price = Money::new(4800, Currency::Jpy).unwrap();
        let c = s
            .apply_mutation(&id, "sku-tee-01", 2, 1, price)
            .unwrap();
        assert_eq!(c.version, 2);
        assert_eq!(c.items[0].quantity, 2);
        assert_eq!(c.items[0].unit_amount_snapshot, 4800);
    }

    #[test]
    fn cart_subtotal_uses_authoritative_prices_for_two_skus() {
        let mut s = store_with_cart();
        let id = s.carts.keys().next().unwrap().clone();
        // Both snapshots are deliberately wrong (simulating a tampered client).
        s.apply_mutation(&id, "sku-tee-01", 2, 1, Money::new(1, Currency::Jpy).unwrap()).unwrap();
        s.apply_mutation(&id, "sku-hoodie-01", 1, 2, Money::new(1, Currency::Jpy).unwrap()).unwrap();
        let cart = s.carts[&id].clone();
        let tee = Money::new(4800, Currency::Jpy).unwrap();
        let hoodie = Money::new(12800, Currency::Jpy).unwrap();
        let total = cart
            .authoritative_subtotal(&|sku| match sku {
                "sku-tee-01" => Some(tee),
                "sku-hoodie-01" => Some(hoodie),
                _ => None,
            })
            .unwrap();
        assert_eq!(total.amount_minor, 2 * 4800 + 12800);
        assert!(total.amount_minor > cart.items.iter().map(|i| i.unit_amount_snapshot).sum::<i64>());
    }

    #[test]
    fn stale_expected_version_is_conflict_not_overwrite() {
        let mut s = store_with_cart();
        let id = s.carts.keys().next().unwrap().clone();
        let price = Money::new(4800, Currency::Jpy).unwrap();
        s.apply_mutation(&id, "sku-tee-01", 1, 1, price).unwrap();
        let err = s.apply_mutation(&id, "sku-tee-01", 5, 1, price).unwrap_err();
        assert!(matches!(err, AppError::Problem(p) if p.code == "cart.version_conflict"));
        // The cart must be unchanged after a rejected mutation.
        assert_eq!(s.carts[&id].items[0].quantity, 1);
    }

    #[test]
    fn quantity_zero_removes_the_line() {
        let mut s = store_with_cart();
        let id = s.carts.keys().next().unwrap().clone();
        let price = Money::new(4800, Currency::Jpy).unwrap();
        s.apply_mutation(&id, "sku-tee-01", 3, 1, price).unwrap();
        let c = s.apply_mutation(&id, "sku-tee-01", 0, 2, price).unwrap();
        assert!(c.items.is_empty());
    }

    #[test]
    fn subtotal_is_computed_from_authoritative_prices() {
        let mut s = store_with_cart();
        let id = s.carts.keys().next().unwrap().clone();
        // Snapshot says 1 (deliberately wrong, simulating a tampered client).
        let c = s.apply_mutation(&id, "sku-tee-01", 2, 1, Money::new(1, Currency::Jpy).unwrap()).unwrap();
        assert_eq!(c.items[0].unit_amount_snapshot, 1);
        let authoritative = Money::new(4800, Currency::Jpy).unwrap();
        let total = c
            .authoritative_subtotal(&|sku| (sku == "sku-tee-01").then_some(authoritative))
            .unwrap();
        assert_eq!(total.amount_minor, 9600, "must use server price, not snapshot");
    }

    #[test]
    fn unknown_sku_subtotal_is_an_error_not_a_zero_price() {
        let mut s = store_with_cart();
        let id = s.carts.keys().next().unwrap().clone();
        s.apply_mutation(&id, "sku-ghost", 1, 1, Money::new(100, Currency::Jpy).unwrap()).unwrap();
        let cart = s.carts[&id].clone();
        let err = cart.authoritative_subtotal(&|_| None).unwrap_err();
        assert!(matches!(err, AppError::Problem(p) if p.code == "cart.price_unavailable"));
    }

    #[test]
    fn missing_cart_is_404() {
        let mut s = store_with_cart();
        let err = s
            .apply_mutation("cart-does-not-exist", "sku", 1, 1, Money::new(1, Currency::Jpy).unwrap())
            .unwrap_err();
        assert!(matches!(err, AppError::Problem(p) if p.code == "cart.not_found"));
    }
}
