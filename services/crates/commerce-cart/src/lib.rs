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
use commerce_core::JsonBody;
use commerce_core::{
    error::{status, AppError},
    Currency, Money, PriceSource,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub mod repo;

pub use repo::{CartRepository, DbCartRepository, MemoryCartRepository};

/// The cart store, shared across requests.
///
/// It is an `Arc<RwLock<…>>`, not a bare `Arc<CartStore>`. With a bare Arc,
/// Axum's `State` extractor clones the Arc on every request, so
/// `Arc::make_mut` sees a reference count above one and **clones the store**:
/// the mutation lands on a request-local copy and is thrown away when the
/// request ends. The cart service answered "200 OK, version 2" and then
/// returned "cart not found" on the next read. The write lock makes the
/// mutation land on the shared store.
pub type SharedCart = Arc<RwLock<CartStore>>;

/// A cart line, as the client sees it.
///
/// The contract (`contracts/openapi.yaml` -> `CartItem`) names the amount
/// `unit_amount_snapshot`, and the Dart client reads that name, so it stays
/// snake_case here. `currency` and `sku` need no rename.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CartItem {
    pub sku: String,
    pub quantity: u32,
    /// Display-only snapshot captured when the item was added.
    pub unit_amount_snapshot: i64,
    pub currency: Currency,
}

/// A cart, as the client sees it.
///
/// The contract names the id `cartId` (camelCase) but keeps `owner_id` and
/// `expires_at_ms` in snake_case — the contract is a mix, so the renames are
/// per-field rather than blanket. The Dart client reads `cartId` with a
/// `cart_id` fallback, so it works either way, but the contract's spelling
/// is the one that should be produced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cart {
    #[serde(rename = "cartId")]
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
            let line = unit
                .checked_mul_rounded(item.quantity)
                .map_err(|e| AppError::Internal(format!("subtotal overflow: {e}")))?;
            total = Some(match total {
                None => line,
                Some(acc) => acc
                    .checked_add(line)
                    .map_err(|e| AppError::Internal(format!("subtotal overflow: {e}")))?,
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
        Self {
            carts: HashMap::new(),
            replays: HashMap::new(),
            seq: 0,
        }
    }

    pub fn next_id(&mut self) -> String {
        self.seq += 1;
        format!("cart-{:06}", self.seq)
    }

    /// Merge a new `expectedVersion` check. Returns the cart on success.
    ///
    /// A cart that does not exist yet is **created** by the first mutation.
    /// This service has no "create cart" endpoint, and the contract's
    /// `cartId` is a client-supplied path parameter — so the first
    /// `PATCH /v1/carts/{cartId}/items/{sku}` is what brings the cart into
    /// being. Returning 404 there made the shop unbuyable: a shopper could
    /// never get past "add to cart".
    ///
    /// The created cart starts at version 1, and the caller's
    /// `expectedVersion` of 1 matches it.
    pub fn apply_mutation(
        &mut self,
        cart_id: &str,
        sku: &str,
        new_quantity: u32,
        expected_version: i64,
        unit_price: Money,
    ) -> Result<Cart, AppError> {
        const TRACE: &str = "cart.update";

        if !self.carts.contains_key(cart_id) {
            if expected_version != 1 {
                // The caller believes the cart is at some other version, so
                // creating it here would silently discard their view. That is
                // a conflict, not a fresh cart.
                return Err(status::conflict(
                    "cart.version_conflict",
                    format!("cart {cart_id} does not exist at version {expected_version}"),
                    TRACE,
                ));
            }
            self.carts.insert(
                cart_id.to_string(),
                Cart {
                    cart_id: cart_id.to_string(),
                    owner_id: None,
                    version: 1,
                    items: vec![],
                    expires_at_ms: commerce_core::idempotency::now_ms() + 30 * 24 * 3600 * 1000,
                },
            );
        }

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

        if let Some(item) = cart.items.iter_mut().find(|i| i.sku == sku) {
            if new_quantity == 0 {
                cart.items.retain(|i| i.sku != sku);
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

/// The cart mutation body.
///
/// The contract (`contracts/openapi.yaml` -> the cart update request) says
/// `expectedVersion` and `clientUnitAmountMinor`; this struct required
/// `expected_version` and `client_unit_amount_minor`, so a contract-
/// conforming client was rejected with 422 before it could change anything.
/// `rename` makes the contract's spelling the accepted one; `alias` keeps the
/// old snake_case working.
#[derive(Debug, Deserialize)]
pub struct UpdateItemBody {
    pub quantity: u32,
    #[serde(rename = "expectedVersion", alias = "expected_version")]
    pub expected_version: i64,
    /// Optional display-only hint from the client. **Ignored for money math.**
    #[serde(rename = "clientUnitAmountMinor", alias = "client_unit_amount_minor")]
    pub client_unit_amount_minor: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CartPath {
    pub cart_id: String,
    pub sku: String,
}

pub fn router(state: Arc<dyn CartRepository>, prices: Arc<dyn PriceSource>) -> Router {
    Router::new()
        .route("/v1/carts/{cart_id}", get(get_cart))
        .route("/v1/carts/{cart_id}/items/{sku}", patch(update_item))
        .route("/healthz", get(health))
        .with_state((state, prices))
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn get_cart(
    State((repo, _prices)): State<(Arc<dyn CartRepository>, Arc<dyn PriceSource>)>,
    Path(p): Path<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Json<Cart>, AppError> {
    const TRACE: &str = "cart.get";
    let cart_id = p
        .get("cart_id")
        .ok_or_else(|| status::bad_request("cart.missing_id", "cartId required", TRACE))?;
    let cart = repo
        .load(cart_id)
        .await?
        .ok_or_else(|| status::not_found("cart.not_found", "cart not found", TRACE))?;

    // Server-side ownership check. A client-side session flag is never trusted.
    if let Some(owner) = &cart.owner_id {
        let presented = session_of(&headers);
        if presented.as_deref() != Some(owner.as_str()) {
            return Err(status::forbidden(
                "cart.forbidden",
                "cart belongs to another session",
                TRACE,
            ));
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
    State((repo, prices)): State<(Arc<dyn CartRepository>, Arc<dyn PriceSource>)>,
    Path(p): Path<CartPath>,
    headers: HeaderMap,
    JsonBody(body): JsonBody<UpdateItemBody>,
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
        if let Some(cached) = repo.replay(&p.cart_id, k).await? {
            if cached.get("_fp").and_then(|v| v.as_str()) == Some(fingerprint.as_str()) {
                return Ok(Json(
                    serde_json::from_value(cached.clone())
                        .map_err(|e| AppError::Internal(format!("replay decode: {e}")))?,
                ));
            }
            return Err(status::conflict(
                "cart.idempotency_key_reuse",
                "Idempotency-Key reused with a different payload",
                TRACE,
            ));
        }
    }

    // The unit price comes from the catalogue, never from the client.
    //
    // The client may send `clientUnitAmountMinor`; it is a display hint and
    // is discarded here. A cart whose displayed price the client can choose
    // shows the shopper a different number from the one they are charged at
    // checkout, which is both a trust problem and a support problem.
    //
    // A SKU the catalogue does not know cannot be priced, so the mutation is
    // rejected rather than stored at some default.
    let catalogue = prices
        .fetch_prices(std::slice::from_ref(&p.sku))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "cart could not be priced from the catalogue");
            status::service_unavailable(
                "cart.pricing_unavailable",
                "pricing is temporarily unavailable",
                TRACE,
            )
        })?;
    let unit_price = catalogue
        .iter()
        .find(|(sku, _)| sku == &p.sku)
        .map(|(_, money)| *money)
        .ok_or_else(|| {
            tracing::warn!(sku = %p.sku, "cart line has no catalogue price");
            status::unprocessable(
                "cart.price_unavailable",
                format!("sku {} has no authoritative price", p.sku),
                TRACE,
            )
        })?;

    // The mutation happens in the store, which is either this process's map
    // or the database every instance shares.
    let updated = repo
        .apply_mutation(
            &p.cart_id,
            &p.sku,
            body.quantity,
            body.expected_version,
            unit_price,
        )
        .await?;

    if let Some(k) = key {
        let mut cached = serde_json::to_value(&updated).unwrap_or(serde_json::Value::Null);
        if let Some(obj) = cached.as_object_mut() {
            obj.insert("_fp".into(), serde_json::Value::String(fingerprint));
        }
        repo.remember(&p.cart_id, &k, &cached).await?;
    }
    Ok(Json(updated))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with_cart() -> CartStore {
        let mut s = CartStore::new();
        let id = s.next_id();
        s.carts.insert(
            id.clone(),
            Cart {
                cart_id: id,
                owner_id: None,
                version: 1,
                items: vec![],
                expires_at_ms: 0,
            },
        );
        s
    }

    #[test]
    fn mutation_bumps_version_and_sets_snapshot() {
        let mut s = store_with_cart();
        let id = s.carts.keys().next().unwrap().clone();
        let price = Money::new(4800, Currency::Jpy).unwrap();
        let c = s.apply_mutation(&id, "sku-tee-01", 2, 1, price).unwrap();
        assert_eq!(c.version, 2);
        assert_eq!(c.items[0].quantity, 2);
        assert_eq!(c.items[0].unit_amount_snapshot, 4800);
    }

    #[test]
    fn cart_subtotal_uses_authoritative_prices_for_two_skus() {
        let mut s = store_with_cart();
        let id = s.carts.keys().next().unwrap().clone();
        // Both snapshots are deliberately wrong (simulating a tampered client).
        s.apply_mutation(
            &id,
            "sku-tee-01",
            2,
            1,
            Money::new(1, Currency::Jpy).unwrap(),
        )
        .unwrap();
        s.apply_mutation(
            &id,
            "sku-hoodie-01",
            1,
            2,
            Money::new(1, Currency::Jpy).unwrap(),
        )
        .unwrap();
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
        assert!(
            total.amount_minor
                > cart
                    .items
                    .iter()
                    .map(|i| i.unit_amount_snapshot)
                    .sum::<i64>()
        );
    }

    #[test]
    fn stale_expected_version_is_conflict_not_overwrite() {
        let mut s = store_with_cart();
        let id = s.carts.keys().next().unwrap().clone();
        let price = Money::new(4800, Currency::Jpy).unwrap();
        s.apply_mutation(&id, "sku-tee-01", 1, 1, price).unwrap();
        let err = s
            .apply_mutation(&id, "sku-tee-01", 5, 1, price)
            .unwrap_err();
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
        let c = s
            .apply_mutation(
                &id,
                "sku-tee-01",
                2,
                1,
                Money::new(1, Currency::Jpy).unwrap(),
            )
            .unwrap();
        assert_eq!(c.items[0].unit_amount_snapshot, 1);
        let authoritative = Money::new(4800, Currency::Jpy).unwrap();
        let total = c
            .authoritative_subtotal(&|sku| (sku == "sku-tee-01").then_some(authoritative))
            .unwrap();
        assert_eq!(
            total.amount_minor, 9600,
            "must use server price, not snapshot"
        );
    }

    #[test]
    fn unknown_sku_subtotal_is_an_error_not_a_zero_price() {
        let mut s = store_with_cart();
        let id = s.carts.keys().next().unwrap().clone();
        s.apply_mutation(
            &id,
            "sku-ghost",
            1,
            1,
            Money::new(100, Currency::Jpy).unwrap(),
        )
        .unwrap();
        let cart = s.carts[&id].clone();
        let err = cart.authoritative_subtotal(&|_| None).unwrap_err();
        assert!(matches!(err, AppError::Problem(p) if p.code == "cart.price_unavailable"));
    }

    /// The first mutation on an unknown cart creates it.
    ///
    /// This used to be a 404. The change was necessary rather than
    /// cosmetic: the service exposes no "create cart" route, and the
    /// contract's `cartId` is a client-supplied path parameter. With a 404
    /// here, a shopper could never add anything — the very first
    /// `PATCH /v1/carts/{cartId}/items/{sku}` was always against a cart
    /// that did not exist yet, so the shop had no way to buy anything.
    ///
    /// The created cart starts at version 1, which is what the client's
    /// `expectedVersion: 1` claims.
    #[test]
    fn the_first_mutation_creates_the_cart() {
        let mut s = CartStore::new();
        let price = Money::new(4800, Currency::Jpy).unwrap();
        let cart = s
            .apply_mutation("cart-new", "sku-tee-01", 2, 1, price)
            .expect("the first mutation creates the cart");
        assert_eq!(cart.version, 2, "the mutation bumps the new cart to 2");
        assert_eq!(cart.items.len(), 1);
        assert!(s.carts.contains_key("cart-new"));
    }

    /// A mutation that claims a version other than 1 against an unknown
    /// cart is a conflict, not a fresh cart: the caller believes the cart
    /// exists, so creating it here would silently discard their view.
    #[test]
    fn a_stale_version_against_an_unknown_cart_is_a_conflict() {
        let mut s = CartStore::new();
        let err = s
            .apply_mutation(
                "cart-does-not-exist",
                "sku",
                1,
                7,
                Money::new(1, Currency::Jpy).unwrap(),
            )
            .unwrap_err();
        assert!(matches!(err, AppError::Problem(p) if p.code == "cart.version_conflict"));
    }
}
