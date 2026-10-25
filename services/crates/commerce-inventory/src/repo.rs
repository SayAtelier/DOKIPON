//! The stock ledger, in a store every instance of the service shares.
//!
//! ## Why the ledger, and not only the orders
//!
//! A Vercel Function may have several instances, and a store that lives in
//! one process's memory is therefore per-instance. The stock levels the
//! operator console shows are the levels of the instance that answered, and
//! a reservation taken on one instance is invisible to the next — so the
//! units can be promised twice.
//!
//! ## What is the truth
//!
//! When a database is configured, **the database is the truth** and the
//! in-memory map is not consulted for a level this process did not serve
//! itself. A store that answered from its own memory while the database said
//! otherwise is the failure this whole design exists to prevent.
//!
//! ## The state machine is not touched
//!
//! The rules — one active reservation per order and SKU, a reservation
//! cannot exceed the stock on hand, a commit takes the units out of the
//! warehouse and a release gives them back — live in [`InventoryStore`], and
//! they are the same rules whichever store is underneath. The
//! database-backed store reads what the request needs, lets the in-memory
//! state machine do its work, and writes the result back — in one
//! transaction, so the stock and the reservation cannot disagree.
//!
//! ## Two things are decided by the database
//!
//! - **The id.** The reservations come from a sequence, so two instances
//!   cannot mint the same one. The ids are reserved as a block, because the
//!   state machine that allocates them is synchronous and cannot await.
//! - **The stock.** The row is locked while the reservation is made
//!   (`FOR UPDATE`), so two instances reserving the last unit cannot both
//!   succeed.

use async_trait::async_trait;
use commerce_core::error::{status, AppError};
use commerce_core::ReservationState;
use sqlx::postgres::PgPool;
use sqlx::Row;

use crate::{InventoryStore, Reservation, SharedInventory, StockLevel, StockLevelRow, StockPage};

/// The inventory service's store.
///
/// The service does not care whether the ledger lives in a process's memory
/// or in a database: it asks this, and it gets the same answers either way.
#[async_trait]
pub trait InventoryRepository: Send + Sync {
    /// `inventory.list` — a page of the stock levels.
    async fn list(&self, offset: usize, limit: usize) -> Result<StockPage, AppError>;

    /// `inventory.reserve` — idempotent per order and SKU.
    async fn reserve(
        &self,
        sku: &str,
        order_id: &str,
        quantity: i64,
    ) -> Result<Reservation, AppError>;

    /// Called when the payment succeeds and the order is confirmed.
    async fn commit(&self, reservation_id: &str) -> Result<Reservation, AppError>;

    /// Compensation: payment declined / order rejected / cancelled.
    async fn release(&self, reservation_id: &str) -> Result<Reservation, AppError>;
}

/// The ledger in this process's memory.
///
/// The local run and the in-process edge use it on purpose: they are one
/// process, so there is nobody to share the ledger with.
pub struct MemoryInventoryRepository {
    store: SharedInventory,
}

impl MemoryInventoryRepository {
    pub fn new(store: SharedInventory) -> Self {
        Self { store }
    }
}

#[async_trait]
impl InventoryRepository for MemoryInventoryRepository {
    async fn list(&self, offset: usize, limit: usize) -> Result<StockPage, AppError> {
        let guard = self
            .store
            .read()
            .map_err(|_| AppError::Internal("inventory lock poisoned".into()))?;

        // Sorted by SKU so a page is stable: the same cursor returns the same
        // rows as long as the store does not change underneath it.
        let mut sorted: Vec<(&str, &StockLevel)> = guard
            .stock
            .iter()
            .map(|(sku, level)| (sku.as_str(), level))
            .collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));

        let total = sorted.len();
        let items: Vec<StockLevelRow> = sorted
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|(sku, level)| StockLevelRow {
                sku: sku.to_string(),
                on_hand: level.on_hand,
                reserved: level.reserved,
                available: level.available(),
            })
            .collect();

        let next_cursor =
            (offset + items.len() < total).then(|| (offset + items.len()).to_string());
        Ok(StockPage { items, next_cursor })
    }

    async fn reserve(
        &self,
        sku: &str,
        order_id: &str,
        quantity: i64,
    ) -> Result<Reservation, AppError> {
        let mut guard = self
            .store
            .write()
            .map_err(|_| AppError::Internal("inventory lock poisoned".into()))?;
        guard.reserve(sku, order_id, quantity)
    }

    async fn commit(&self, reservation_id: &str) -> Result<Reservation, AppError> {
        let mut guard = self
            .store
            .write()
            .map_err(|_| AppError::Internal("inventory lock poisoned".into()))?;
        guard.commit(reservation_id)
    }

    async fn release(&self, reservation_id: &str) -> Result<Reservation, AppError> {
        let mut guard = self
            .store
            .write()
            .map_err(|_| AppError::Internal("inventory lock poisoned".into()))?;
        guard.release(reservation_id)
    }
}

/// The ledger in the database.
pub struct DbInventoryRepository {
    pool: PgPool,
}

impl DbInventoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Put the demo stock levels into the ledger.
    ///
    /// A SKU that is already there keeps its level: the reserved units of a
    /// live reservation must not be overwritten by a boot. A SKU that is
    /// missing is added, so a catalogue that grows a SKU shows up in the
    /// ledger the next time the service starts.
    pub async fn seed(&self, levels: &[(&str, i64)]) -> Result<(), AppError> {
        for (sku, on_hand) in levels {
            sqlx::query(
                "INSERT INTO stock (sku_id, on_hand, reserved) VALUES ($1, $2, 0) \
                 ON CONFLICT (sku_id) DO NOTHING",
            )
            .bind(sku)
            .bind(on_hand)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        }
        Ok(())
    }
}

/// How many ids one reservation may need. A reservation needs one.
const ID_BLOCK: u64 = 4;

#[async_trait]
impl InventoryRepository for DbInventoryRepository {
    async fn list(&self, offset: usize, limit: usize) -> Result<StockPage, AppError> {
        let rows = sqlx::query(
            "SELECT sku_id, on_hand, reserved FROM stock ORDER BY sku_id \
             LIMIT $1 OFFSET $2",
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;

        let total: i64 = sqlx::query_scalar("SELECT count(*) FROM stock")
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let sku: String = row.try_get("sku_id").map_err(read_error)?;
            let on_hand: i64 = row.try_get("on_hand").map_err(read_error)?;
            let reserved: i64 = row.try_get("reserved").map_err(read_error)?;
            items.push(StockLevelRow {
                sku,
                on_hand,
                reserved,
                available: (on_hand - reserved).max(0),
            });
        }

        let next_cursor =
            (offset + items.len() < total as usize).then(|| (offset + items.len()).to_string());
        Ok(StockPage { items, next_cursor })
    }

    async fn reserve(
        &self,
        sku: &str,
        order_id: &str,
        quantity: i64,
    ) -> Result<Reservation, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;

        // The stock row is locked for the rest of the transaction: two
        // instances reserving the last unit cannot both see it as available.
        let level = sqlx::query("SELECT on_hand, reserved FROM stock WHERE sku_id = $1 FOR UPDATE")
            .bind(sku)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?;
        let Some(level) = level else {
            return Err(status::not_found(
                "inventory.sku_not_found",
                format!("unknown sku {sku}"),
                "inventory.reserve",
            ));
        };
        let on_hand: i64 = level.try_get("on_hand").map_err(read_error)?;
        let reserved: i64 = level.try_get("reserved").map_err(read_error)?;

        // The store the rules live in. It is filled with exactly what this
        // request needs to see.
        let mut store = InventoryStore::seed(&[]);
        store
            .stock
            .insert(sku.to_string(), StockLevel { on_hand, reserved });
        if let Some(active) = load_active_reservation(&mut tx, order_id, sku).await? {
            store
                .reservations
                .insert(active.reservation_id.clone(), active);
        }

        let first_id = reserve_ids(&mut tx, ID_BLOCK).await?;
        store.sequence = first_id - 1;
        let reservation = store.reserve(sku, order_id, quantity)?;
        if store.sequence > first_id + ID_BLOCK - 1 {
            return Err(AppError::Internal(
                "the reservation needed more ids than the store reserved".into(),
            ));
        }

        // The reservation is new (an existing active one was returned by the
        // state machine without minting an id), so it is written.
        if store.sequence == first_id {
            sqlx::query(
                "INSERT INTO inventory_reservation \
                 (reservation_id, sku_id, order_id, quantity, state, expires_at_ms) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&reservation.reservation_id)
            .bind(&reservation.sku)
            .bind(&reservation.order_id)
            .bind(reservation.quantity)
            .bind(state_text(reservation.state))
            .bind(reservation.expires_at_ms)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;

            sqlx::query("UPDATE stock SET reserved = reserved + $2 WHERE sku_id = $1")
                .bind(sku)
                .bind(reservation.quantity)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
        }

        tx.commit().await.map_err(db_error)?;
        Ok(reservation)
    }

    async fn commit(&self, reservation_id: &str) -> Result<Reservation, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let reservation = load_reservation(&mut tx, reservation_id)
            .await?
            .ok_or_else(|| {
                status::not_found(
                    "inventory.reservation_not_found",
                    "reservation not found",
                    "inventory.commit",
                )
            })?;

        let level = sqlx::query("SELECT on_hand, reserved FROM stock WHERE sku_id = $1 FOR UPDATE")
            .bind(&reservation.sku)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?;
        let Some(level) = level else {
            return Err(status::not_found(
                "inventory.sku_not_found",
                format!("unknown sku {}", reservation.sku),
                "inventory.commit",
            ));
        };
        let on_hand: i64 = level.try_get("on_hand").map_err(read_error)?;
        let reserved: i64 = level.try_get("reserved").map_err(read_error)?;

        let mut store = InventoryStore::seed(&[]);
        store
            .stock
            .insert(reservation.sku.clone(), StockLevel { on_hand, reserved });
        store
            .reservations
            .insert(reservation.reservation_id.clone(), reservation.clone());
        let committed = store.commit(reservation_id)?;

        save_reservation(&mut tx, &committed).await?;
        sqlx::query(
            "UPDATE stock SET reserved = GREATEST(reserved - $2, 0), \
             on_hand = GREATEST(on_hand - $2, 0) WHERE sku_id = $1",
        )
        .bind(&committed.sku)
        .bind(committed.quantity)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;

        tx.commit().await.map_err(db_error)?;
        Ok(committed)
    }

    async fn release(&self, reservation_id: &str) -> Result<Reservation, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let reservation = load_reservation(&mut tx, reservation_id)
            .await?
            .ok_or_else(|| {
                status::not_found(
                    "inventory.reservation_not_found",
                    "reservation not found",
                    "inventory.release",
                )
            })?;

        let level = sqlx::query("SELECT on_hand, reserved FROM stock WHERE sku_id = $1 FOR UPDATE")
            .bind(&reservation.sku)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?;
        let Some(level) = level else {
            return Err(status::not_found(
                "inventory.sku_not_found",
                format!("unknown sku {}", reservation.sku),
                "inventory.release",
            ));
        };
        let on_hand: i64 = level.try_get("on_hand").map_err(read_error)?;
        let reserved: i64 = level.try_get("reserved").map_err(read_error)?;

        let mut store = InventoryStore::seed(&[]);
        store
            .stock
            .insert(reservation.sku.clone(), StockLevel { on_hand, reserved });
        store
            .reservations
            .insert(reservation.reservation_id.clone(), reservation.clone());
        let released = store.release(reservation_id)?;

        save_reservation(&mut tx, &released).await?;
        sqlx::query("UPDATE stock SET reserved = GREATEST(reserved - $2, 0) WHERE sku_id = $1")
            .bind(&released.sku)
            .bind(released.quantity)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;

        tx.commit().await.map_err(db_error)?;
        Ok(released)
    }
}

/// The ids, reserved from the sequence.
async fn reserve_ids(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    count: u64,
) -> Result<u64, AppError> {
    let first: i64 = sqlx::query_scalar("SELECT nextval('inventory_reservation_seq')")
        .fetch_one(&mut **tx)
        .await
        .map_err(db_error)?;
    for _ in 1..count {
        sqlx::query("SELECT nextval('inventory_reservation_seq')")
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;
    }
    u64::try_from(first)
        .map_err(|_| AppError::Internal("the reservation sequence went backwards".into()))
}

/// The reservation, read out of the database.
async fn load_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    reservation_id: &str,
) -> Result<Option<Reservation>, AppError> {
    let row = sqlx::query(
        "SELECT reservation_id, sku_id, order_id, quantity, state, expires_at_ms \
         FROM inventory_reservation WHERE reservation_id = $1",
    )
    .bind(reservation_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let state: String = row.try_get("state").map_err(read_error)?;
    Ok(Some(Reservation {
        reservation_id: row.try_get("reservation_id").map_err(read_error)?,
        sku: row.try_get("sku_id").map_err(read_error)?,
        order_id: row.try_get("order_id").map_err(read_error)?,
        quantity: row.try_get("quantity").map_err(read_error)?,
        state: state_of(&state)?,
        expires_at_ms: row.try_get("expires_at_ms").map_err(read_error)?,
    }))
}

/// The active reservation of an order and a SKU, if there is one.
async fn load_active_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    order_id: &str,
    sku: &str,
) -> Result<Option<Reservation>, AppError> {
    let row = sqlx::query(
        "SELECT reservation_id, sku_id, order_id, quantity, state, expires_at_ms \
         FROM inventory_reservation \
         WHERE order_id = $1 AND sku_id = $2 AND state IN ('pending', 'reserved')",
    )
    .bind(order_id)
    .bind(sku)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let state: String = row.try_get("state").map_err(read_error)?;
    Ok(Some(Reservation {
        reservation_id: row.try_get("reservation_id").map_err(read_error)?,
        sku: row.try_get("sku_id").map_err(read_error)?,
        order_id: row.try_get("order_id").map_err(read_error)?,
        quantity: row.try_get("quantity").map_err(read_error)?,
        state: state_of(&state)?,
        expires_at_ms: row.try_get("expires_at_ms").map_err(read_error)?,
    }))
}

/// The reservation, written back.
async fn save_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    reservation: &Reservation,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE inventory_reservation SET state = $2, expires_at_ms = $3 \
         WHERE reservation_id = $1",
    )
    .bind(&reservation.reservation_id)
    .bind(state_text(reservation.state))
    .bind(reservation.expires_at_ms)
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;
    Ok(())
}

/// A reservation state, as it is stored.
fn state_text(state: ReservationState) -> &'static str {
    match state {
        ReservationState::Pending => "pending",
        ReservationState::Reserved => "reserved",
        ReservationState::Committed => "committed",
        ReservationState::Released => "released",
        ReservationState::Expired => "expired",
    }
}

/// A reservation state, as it is read back.
fn state_of(stored: &str) -> Result<ReservationState, AppError> {
    serde_json::from_value(serde_json::Value::String(stored.trim().to_string())).map_err(|error| {
        AppError::Internal(format!("a stored reservation state is not one: {error}"))
    })
}

/// A database failure, as a service error.
fn db_error(error: sqlx::Error) -> AppError {
    AppError::Internal(format!("the inventory store is unavailable: {error}"))
}

/// A field that would not read.
fn read_error(error: sqlx::Error) -> AppError {
    AppError::Internal(format!("could not read the ledger: {error}"))
}
