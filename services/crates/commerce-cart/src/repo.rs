//! The carts, in a store every instance of the service shares.
//!
//! ## Why the carts, and not only the sessions
//!
//! A Vercel Function may have several instances, and a store that lives in
//! one process's memory is therefore per-instance. A cart created on one
//! instance is invisible to the next — so a shopper who adds a line and then
//! reloads the page can find the cart empty, or find somebody else's cart,
//! because two instances both mint `cart-000001`.
//!
//! ## What is the truth
//!
//! When a database is configured, **the database is the truth** and the
//! in-memory map is not consulted for a cart this process did not serve
//! itself. A store that answered from its own memory while the database said
//! otherwise is the failure this whole design exists to prevent.
//!
//! ## The state machine is not touched
//!
//! The rules about versions, conflicts and prices live in
//! [`CartStore::apply_mutation`], and they are the same rules whichever store
//! is underneath. The database-backed store therefore does not re-implement
//! them: it reads the cart, puts it into a store of its own, lets the
//! in-memory state machine do its work, and writes the result back — with a
//! version check in the same statement, so two instances changing one cart at
//! the same time cannot both win.

use async_trait::async_trait;
use commerce_core::error::{status, AppError};
use commerce_core::{Currency, Money};
use serde_json::Value;
use sqlx::postgres::PgPool;
use sqlx::Row;

use crate::{Cart, CartItem, CartStore, SharedCart};

/// The cart service's store.
///
/// The service does not care whether the carts live in a process's memory or
/// in a database: it asks this, and it gets the same answers either way.
#[async_trait]
pub trait CartRepository: Send + Sync {
    /// The cart, if there is one.
    async fn load(&self, cart_id: &str) -> Result<Option<Cart>, AppError>;

    /// Apply a mutation to a cart and return the cart that came out of it.
    ///
    /// `expected_version` is the caller's optimistic-lock version. A cart
    /// that is not at that version is a conflict, not a fresh cart.
    async fn apply_mutation(
        &self,
        cart_id: &str,
        owner_id: Option<&str>,
        sku: &str,
        quantity: u32,
        expected_version: i64,
        price: Money,
    ) -> Result<Cart, AppError>;

    /// The stored answer for an idempotency key, if there is one.
    ///
    /// The key is scoped to the cart: the contract says an Idempotency-Key
    /// is scoped to (actor, operation), and the cart is the actor here.
    async fn replay(&self, cart_id: &str, key: &str) -> Result<Option<Value>, AppError>;

    /// Remember the answer for an idempotency key.
    async fn remember(&self, cart_id: &str, key: &str, response: &Value) -> Result<(), AppError>;
}

/// The carts in this process's memory.
///
/// The local run and the in-process edge use it on purpose: they are one
/// process, so there is nobody to share the carts with.
pub struct MemoryCartRepository {
    store: SharedCart,
}

impl MemoryCartRepository {
    pub fn new(store: SharedCart) -> Self {
        Self { store }
    }
}

/// The idempotency key, scoped to the cart it belongs to.
///
/// The same function the order service uses, so both stores agree on what
/// "the same key" means.
fn scoped_key(cart_id: &str, key: &str) -> String {
    commerce_core::idempotency::hash_key(cart_id, "cart.update", key)
}

#[async_trait]
impl CartRepository for MemoryCartRepository {
    async fn load(&self, cart_id: &str) -> Result<Option<Cart>, AppError> {
        let guard = self
            .store
            .read()
            .map_err(|_| AppError::Internal("cart store lock poisoned".into()))?;
        Ok(guard.carts.get(cart_id).cloned())
    }

    async fn apply_mutation(
        &self,
        cart_id: &str,
        owner_id: Option<&str>,
        sku: &str,
        quantity: u32,
        expected_version: i64,
        price: Money,
    ) -> Result<Cart, AppError> {
        // The whole mutation happens under the write lock, on the shared
        // store: reading the cart, checking the version and writing the
        // result back are one indivisible step here.
        let mut guard = self
            .store
            .write()
            .map_err(|_| AppError::Internal("cart store lock poisoned".into()))?;
        guard.apply_mutation(cart_id, owner_id, sku, quantity, expected_version, price)
    }

    async fn replay(&self, cart_id: &str, key: &str) -> Result<Option<Value>, AppError> {
        let guard = self
            .store
            .read()
            .map_err(|_| AppError::Internal("cart store lock poisoned".into()))?;
        Ok(guard.replays.get(&scoped_key(cart_id, key)).cloned())
    }

    async fn remember(&self, cart_id: &str, key: &str, response: &Value) -> Result<(), AppError> {
        let mut guard = self
            .store
            .write()
            .map_err(|_| AppError::Internal("cart store lock poisoned".into()))?;
        guard
            .replays
            .insert(scoped_key(cart_id, key), response.clone());
        Ok(())
    }
}

/// The carts in the database.
pub struct DbCartRepository {
    pool: PgPool,
}

impl DbCartRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CartRepository for DbCartRepository {
    async fn load(&self, cart_id: &str) -> Result<Option<Cart>, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let cart = load_in(&mut tx, cart_id).await?;
        tx.commit().await.map_err(db_error)?;
        Ok(cart)
    }

    async fn apply_mutation(
        &self,
        cart_id: &str,
        owner_id: Option<&str>,
        sku: &str,
        quantity: u32,
        expected_version: i64,
        price: Money,
    ) -> Result<Cart, AppError> {
        const TRACE: &str = "cart.update";

        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let existing = load_in(&mut tx, cart_id).await?;

        // The state machine is the in-memory one, unchanged. The cart is put
        // into a store of its own, the mutation is applied, and the result is
        // written back.
        let mut store = CartStore::new();
        if let Some(cart) = existing.clone() {
            store.carts.insert(cart_id.to_string(), cart);
        }
        let updated =
            store.apply_mutation(cart_id, owner_id, sku, quantity, expected_version, price)?;

        // The cart that was there when this request read it has to still be
        // there, at the same version, when the result is written back —
        // otherwise another instance changed it in between, and this write
        // would silently drop that change.
        if existing.is_some() {
            let changed = sqlx::query(
                "UPDATE cart SET owner_id = $2, version = $3, expires_at_ms = $4 \
                 WHERE cart_id = $1 AND version = $5",
            )
            .bind(&updated.cart_id)
            .bind(updated.owner_id.as_deref())
            .bind(version_of(updated.version)?)
            .bind(updated.expires_at_ms)
            .bind(version_of(expected_version)?)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
            if changed.rows_affected() == 0 {
                return Err(status::conflict(
                    "cart.version_conflict",
                    format!("cart {cart_id} changed while this mutation was being applied"),
                    TRACE,
                ));
            }
        } else {
            // A cart that did not exist is created by this mutation, and only
            // by it: if another instance created the same cart in the
            // meantime, the insert collides and the mutation is a conflict.
            let inserted = sqlx::query(
                "INSERT INTO cart (cart_id, owner_id, version, expires_at_ms) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(&updated.cart_id)
            .bind(updated.owner_id.as_deref())
            .bind(version_of(updated.version)?)
            .bind(updated.expires_at_ms)
            .execute(&mut *tx)
            .await;
            if let Err(error) = inserted {
                if is_unique_violation(&error) {
                    return Err(status::conflict(
                        "cart.version_conflict",
                        format!("cart {cart_id} was created by another request"),
                        TRACE,
                    ));
                }
                return Err(db_error(error));
            }
        }

        // The lines are replaced, not merged: the cart is the truth, and a
        // line the shopper removed must not survive in the table.
        sqlx::query("DELETE FROM cart_item WHERE cart_id = $1")
            .bind(&updated.cart_id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        for item in &updated.items {
            sqlx::query(
                "INSERT INTO cart_item \
                 (cart_id, sku_id, quantity, unit_amount_snapshot, currency) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(&updated.cart_id)
            .bind(&item.sku)
            .bind(i32::try_from(item.quantity).map_err(|_| {
                AppError::Internal("a cart line's quantity does not fit the database".into())
            })?)
            .bind(item.unit_amount_snapshot)
            .bind(item.currency.as_str())
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        }

        tx.commit().await.map_err(db_error)?;
        Ok(updated)
    }

    async fn replay(&self, cart_id: &str, key: &str) -> Result<Option<Value>, AppError> {
        let row = sqlx::query(
            "SELECT response FROM idempotency_record \
             WHERE actor = $1 AND operation = $2 AND key = $3",
        )
        .bind(cart_id)
        .bind("cart.update")
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let response: String = row.try_get("response").map_err(read_error)?;
        serde_json::from_str(&response)
            .map(Some)
            .map_err(|error| AppError::Internal(format!("replay decode: {error}")))
    }

    async fn remember(&self, cart_id: &str, key: &str, response: &Value) -> Result<(), AppError> {
        let body = serde_json::to_string(response)
            .map_err(|error| AppError::Internal(format!("replay encode: {error}")))?;
        let fingerprint = response
            .get("_fp")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        sqlx::query(
            "INSERT INTO idempotency_record \
             (actor, operation, key, fingerprint, response, status, at_ms) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (actor, operation, key) DO UPDATE SET \
               fingerprint = EXCLUDED.fingerprint, response = EXCLUDED.response, \
               status = EXCLUDED.status, at_ms = EXCLUDED.at_ms",
        )
        .bind(cart_id)
        .bind("cart.update")
        .bind(key)
        .bind(fingerprint)
        .bind(body)
        .bind(200_i32)
        .bind(commerce_core::idempotency::now_ms())
        .execute(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(())
    }
}

/// The cart, and its lines, read out of the database.
async fn load_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cart_id: &str,
) -> Result<Option<Cart>, AppError> {
    let row = sqlx::query(
        "SELECT cart_id, owner_id, version, expires_at_ms FROM cart WHERE cart_id = $1",
    )
    .bind(cart_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)?;
    let Some(row) = row else {
        return Ok(None);
    };

    let owner_id: Option<String> = row.try_get("owner_id").map_err(read_error)?;
    let version: i32 = row.try_get("version").map_err(read_error)?;
    let expires_at_ms: i64 = row.try_get("expires_at_ms").map_err(read_error)?;

    let item_rows = sqlx::query(
        "SELECT sku_id, quantity, unit_amount_snapshot, currency \
         FROM cart_item WHERE cart_id = $1 ORDER BY sku_id",
    )
    .bind(cart_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_error)?;

    let mut items = Vec::with_capacity(item_rows.len());
    for item_row in item_rows {
        let sku: String = item_row.try_get("sku_id").map_err(read_error)?;
        let quantity: i32 = item_row.try_get("quantity").map_err(read_error)?;
        let unit_amount_snapshot: i64 = item_row
            .try_get("unit_amount_snapshot")
            .map_err(read_error)?;
        let currency: String = item_row.try_get("currency").map_err(read_error)?;
        items.push(CartItem {
            sku,
            quantity: u32::try_from(quantity).map_err(|_| {
                AppError::Internal("a stored cart line has a negative quantity".into())
            })?,
            unit_amount_snapshot,
            currency: currency_of(&currency)?,
        });
    }

    Ok(Some(Cart {
        cart_id: row.try_get("cart_id").map_err(read_error)?,
        owner_id,
        version: i64::from(version),
        items,
        expires_at_ms,
    }))
}

/// A currency, as it is stored.
///
/// The column is `char(3)`, so a stored value may carry padding; it is
/// trimmed before it is read.
fn currency_of(stored: &str) -> Result<Currency, AppError> {
    serde_json::from_value(Value::String(stored.trim().to_string()))
        .map_err(|error| AppError::Internal(format!("a stored currency is not one: {error}")))
}

/// A version, as the database stores it.
fn version_of(version: i64) -> Result<i32, AppError> {
    i32::try_from(version)
        .map_err(|_| AppError::Internal("a cart's version does not fit the database".into()))
}

/// A database failure, as a service error.
fn db_error(error: sqlx::Error) -> AppError {
    AppError::Internal(format!("the cart store is unavailable: {error}"))
}

/// A field that would not read.
fn read_error(error: sqlx::Error) -> AppError {
    AppError::Internal(format!("could not read the cart: {error}"))
}

/// Whether the error is a unique-constraint violation.
fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(db) if db.code().as_deref() == Some("23505"))
}
