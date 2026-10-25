//! The orders, in a store every instance of the service shares.
//!
//! ## Why the orders, and not only the sessions
//!
//! A Vercel Function may have several instances, and a store that lives in
//! one process's memory is therefore per-instance. An order created on one
//! instance is invisible to the next: the shopper's own order list is empty
//! on the instance that answers, and the operator's list is missing an
//! order that was really placed.
//!
//! ## What is the truth
//!
//! When a database is configured, **the database is the truth** and the
//! in-memory map is not consulted for an order this process did not serve
//! itself. A store that answered from its own memory while the database said
//! otherwise is the failure this whole design exists to prevent.
//!
//! ## The state machine is not touched
//!
//! The rules about transitions, the step budget, the idempotency gate and
//! the trail live in [`OrderStore`], and they are the same rules whichever
//! store is underneath. The database-backed store therefore does not
//! re-implement them: it reads what the request needs, lets the in-memory
//! state machine do its work, and writes the result back — in one
//! transaction, so an order is never half written.
//!
//! ## Two things are decided by the database, not by the process
//!
//! - **The ids.** The order, the checkout, the reservation and the provider
//!   reference come from a sequence, so two instances cannot mint the same
//!   one. The ids are reserved as a block, because the state machine that
//!   allocates them is synchronous and cannot await.
//! - **The idempotency key.** Two requests with the same key must not both
//!   create an order. The key takes an advisory lock, so the second request
//!   waits for the first one to finish and then sees its answer.

use async_trait::async_trait;
use commerce_core::error::{status, AppError};
use commerce_core::idempotency::{IdempotencyRecord, IdempotencyState};
use commerce_core::{CheckoutState, DomainEvent};
use serde_json::Value;
use sqlx::postgres::PgPool;
use sqlx::Row;
use std::collections::HashSet;

use crate::{Order, OrderLine, OrderStore, PaymentOutcome, SharedOrder, TrailEntry};

/// The order service's store.
///
/// The service does not care whether the orders live in a process's memory
/// or in a database: it asks this, and it gets the same answers either way.
#[async_trait]
pub trait OrderRepository: Send + Sync {
    /// `checkout.submit` — the whole Saga, driven by the payment outcome.
    async fn submit(
        &self,
        buyer_id: &str,
        lines: Vec<OrderLine>,
        idempotency_key: &str,
        payment_outcome: PaymentOutcome,
    ) -> Result<(String, CheckoutState, PaymentOutcome), AppError>;

    /// `checkout.reconcile` — the only way out of `PaymentUnknown`.
    async fn reconcile(&self, order_id: &str, authorized: bool) -> Result<CheckoutState, AppError>;

    /// One order, by its id.
    async fn get(&self, order_id: &str) -> Result<Order, AppError>;

    /// One order, by its checkout's id.
    async fn by_checkout(&self, checkout_id: &str) -> Result<Order, AppError>;

    /// The orders of one buyer.
    async fn list_for(&self, buyer_id: &str) -> Result<Vec<Order>, AppError>;

    /// Every order, across the buyers. The operator's view.
    async fn list_all(&self) -> Result<Vec<Order>, AppError>;

    /// The orders whose payment outcome is not yet known.
    async fn awaiting_reconciliation(&self) -> Result<Vec<Order>, AppError>;
}

/// The orders in this process's memory.
///
/// The local run and the in-process edge use it on purpose: they are one
/// process, so there is nobody to share the orders with.
pub struct MemoryOrderRepository {
    store: SharedOrder,
}

impl MemoryOrderRepository {
    pub fn new(store: SharedOrder) -> Self {
        Self { store }
    }
}

#[async_trait]
impl OrderRepository for MemoryOrderRepository {
    async fn submit(
        &self,
        buyer_id: &str,
        lines: Vec<OrderLine>,
        idempotency_key: &str,
        payment_outcome: PaymentOutcome,
    ) -> Result<(String, CheckoutState, PaymentOutcome), AppError> {
        let mut guard = self
            .store
            .write()
            .map_err(|_| AppError::Internal("order lock poisoned".into()))?;
        guard.submit(buyer_id, lines, idempotency_key, payment_outcome)
    }

    async fn reconcile(&self, order_id: &str, authorized: bool) -> Result<CheckoutState, AppError> {
        let mut guard = self
            .store
            .write()
            .map_err(|_| AppError::Internal("order lock poisoned".into()))?;
        guard.reconcile(order_id, authorized)
    }

    async fn get(&self, order_id: &str) -> Result<Order, AppError> {
        let guard = self
            .store
            .read()
            .map_err(|_| AppError::Internal("order lock poisoned".into()))?;
        guard.get(order_id).cloned()
    }

    async fn by_checkout(&self, checkout_id: &str) -> Result<Order, AppError> {
        let guard = self
            .store
            .read()
            .map_err(|_| AppError::Internal("order lock poisoned".into()))?;
        guard.by_checkout(checkout_id).cloned()
    }

    async fn list_for(&self, buyer_id: &str) -> Result<Vec<Order>, AppError> {
        let guard = self
            .store
            .read()
            .map_err(|_| AppError::Internal("order lock poisoned".into()))?;
        Ok(guard.list_for(buyer_id).into_iter().cloned().collect())
    }

    async fn list_all(&self) -> Result<Vec<Order>, AppError> {
        let guard = self
            .store
            .read()
            .map_err(|_| AppError::Internal("order lock poisoned".into()))?;
        Ok(guard.orders.values().cloned().collect())
    }

    async fn awaiting_reconciliation(&self) -> Result<Vec<Order>, AppError> {
        let guard = self
            .store
            .read()
            .map_err(|_| AppError::Internal("order lock poisoned".into()))?;
        Ok(guard
            .awaiting_reconciliation()
            .into_iter()
            .cloned()
            .collect())
    }
}

/// The orders in the database.
pub struct DbOrderRepository {
    pool: PgPool,
}

impl DbOrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// How many ids one checkout may need: the order, the checkout, the
/// reservation and the provider reference.
///
/// The block is reserved from the sequence before the saga runs, because the
/// saga that allocates the ids is synchronous. A saga that needed more than
/// this would be refused rather than allowed to mint an id another instance
/// may also mint.
const ID_BLOCK: u64 = 8;

#[async_trait]
impl OrderRepository for DbOrderRepository {
    async fn submit(
        &self,
        buyer_id: &str,
        lines: Vec<OrderLine>,
        idempotency_key: &str,
        payment_outcome: PaymentOutcome,
    ) -> Result<(String, CheckoutState, PaymentOutcome), AppError> {
        const TRACE: &str = "checkout.submit";

        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let now_ms = commerce_core::idempotency::now_ms();

        // --- The idempotency key ---------------------------------------------
        //
        // The key takes an advisory lock, so a second request with the same
        // key waits for the first one to finish and then finds its answer
        // instead of creating a second order.
        let scoped =
            commerce_core::idempotency::hash_key(buyer_id, "checkout.submit", idempotency_key);
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
            .bind(&scoped)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;

        // The store the saga runs in. It is filled with exactly what this
        // request needs to see, and the saga's own rules decide what happens.
        let mut store = OrderStore::new(now_ms);
        if let Some(record) = load_idempotency(&mut tx, buyer_id, &scoped).await? {
            // A replayed answer names its checkout, and the checkout names
            // its order. Both are loaded, so the saga can answer from them.
            let checkout_id = match &record.state {
                IdempotencyState::Completed { response_body, .. } => response_body
                    .get("checkoutId")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                IdempotencyState::InProgress => None,
            };
            store.idempotency.insert(scoped.clone(), record);
            if let Some(checkout_id) = checkout_id {
                if let Some(order) = load_by_checkout(&mut tx, &checkout_id).await? {
                    store
                        .checkouts
                        .insert(checkout_id.clone(), order.order_id.clone());
                    store.orders.insert(order.order_id.clone(), order);
                }
            }
        }

        // --- The ids ----------------------------------------------------------
        let first_id = reserve_ids(&mut tx, ID_BLOCK).await?;
        store.sequence = first_id - 1;

        let before: HashSet<String> = store.orders.keys().cloned().collect();
        let (checkout_id, state, outcome) =
            store.submit(buyer_id, lines, idempotency_key, payment_outcome)?;
        if store.sequence > first_id + ID_BLOCK - 1 {
            // The saga wanted more ids than the block had. The transaction is
            // rolled back by the caller, and the ids it did use are not
            // written — so nothing collides with another instance.
            return Err(AppError::Internal(
                "the checkout needed more ids than the store reserved".into(),
            ));
        }
        let _ = TRACE;

        // --- Write it back ----------------------------------------------------
        for order in store.orders.values() {
            if before.contains(&order.order_id) {
                continue;
            }
            save_order(&mut tx, order, now_ms).await?;
        }
        for (checkout, order_id) in store.checkouts.iter() {
            if before.contains(order_id) {
                continue;
            }
            let order_state = store
                .orders
                .get(order_id)
                .map(|order| order.state)
                .unwrap_or(state);
            save_checkout(&mut tx, checkout, order_id, order_state, now_ms).await?;
        }
        if let Some(record) = store.idempotency.get(&scoped) {
            save_idempotency(&mut tx, buyer_id, &scoped, record).await?;
        }

        tx.commit().await.map_err(db_error)?;
        Ok((checkout_id, state, outcome))
    }

    async fn reconcile(&self, order_id: &str, authorized: bool) -> Result<CheckoutState, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let now_ms = commerce_core::idempotency::now_ms();

        let mut store = OrderStore::new(now_ms);
        if let Some(order) = load_order(&mut tx, order_id).await? {
            store.orders.insert(order.order_id.clone(), order);
        }
        let state = store.reconcile(order_id, authorized)?;
        if let Some(order) = store.orders.get(order_id) {
            save_order(&mut tx, order, now_ms).await?;
        }

        tx.commit().await.map_err(db_error)?;
        Ok(state)
    }

    async fn get(&self, order_id: &str) -> Result<Order, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let order = load_order(&mut tx, order_id).await?.ok_or_else(|| {
            status::not_found("order.not_found", "order not found", "orders.detail")
        })?;
        tx.commit().await.map_err(db_error)?;
        Ok(order)
    }

    async fn by_checkout(&self, checkout_id: &str) -> Result<Order, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let order = load_by_checkout(&mut tx, checkout_id)
            .await?
            .ok_or_else(|| {
                status::not_found(
                    "checkout.not_found",
                    "checkout not found",
                    "checkout.status",
                )
            })?;
        tx.commit().await.map_err(db_error)?;
        Ok(order)
    }

    async fn list_for(&self, buyer_id: &str) -> Result<Vec<Order>, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let orders = load_orders(&mut tx, Some(buyer_id), None).await?;
        tx.commit().await.map_err(db_error)?;
        Ok(orders)
    }

    async fn list_all(&self) -> Result<Vec<Order>, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let orders = load_orders(&mut tx, None, None).await?;
        tx.commit().await.map_err(db_error)?;
        Ok(orders)
    }

    async fn awaiting_reconciliation(&self) -> Result<Vec<Order>, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let orders = load_orders(&mut tx, None, Some(true)).await?;
        tx.commit().await.map_err(db_error)?;
        Ok(orders)
    }
}

/// The ids, reserved from the sequence.
///
/// The block is consumed from the sequence, so the ids inside it belong to
/// this request alone: another instance cannot mint them.
async fn reserve_ids(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    count: u64,
) -> Result<u64, AppError> {
    let first: i64 = sqlx::query_scalar("SELECT nextval('order_seq')")
        .fetch_one(&mut **tx)
        .await
        .map_err(db_error)?;
    for _ in 1..count {
        sqlx::query("SELECT nextval('order_seq')")
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;
    }
    u64::try_from(first).map_err(|_| AppError::Internal("the order sequence went backwards".into()))
}

/// The order, read out of the database.
async fn load_order(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    order_id: &str,
) -> Result<Option<Order>, AppError> {
    let row = sqlx::query(
        "SELECT order_id, buyer_id, state, total_amount_minor, total_currency, \
         reservation_id, provider_ref, transitions_used, awaiting_reconciliation \
         FROM commerce_order WHERE order_id = $1",
    )
    .bind(order_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)?;
    let Some(row) = row else {
        return Ok(None);
    };

    let order_id: String = row.try_get("order_id").map_err(read_error)?;
    let buyer_id: String = row.try_get("buyer_id").map_err(read_error)?;
    let state: String = row.try_get("state").map_err(read_error)?;
    let total_amount_minor: i64 = row.try_get("total_amount_minor").map_err(read_error)?;
    let total_currency: String = row.try_get("total_currency").map_err(read_error)?;
    let reservation_id: Option<String> = row.try_get("reservation_id").map_err(read_error)?;
    let provider_ref: Option<String> = row.try_get("provider_ref").map_err(read_error)?;
    let transitions_used: i32 = row.try_get("transitions_used").map_err(read_error)?;
    let awaiting_reconciliation: bool =
        row.try_get("awaiting_reconciliation").map_err(read_error)?;

    let line_rows = sqlx::query(
        "SELECT sku_id, quantity, unit_amount_minor, currency \
         FROM order_line WHERE order_id = $1 ORDER BY sku_id",
    )
    .bind(&order_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_error)?;
    let mut lines = Vec::with_capacity(line_rows.len());
    for line_row in line_rows {
        let sku: String = line_row.try_get("sku_id").map_err(read_error)?;
        let quantity: i32 = line_row.try_get("quantity").map_err(read_error)?;
        let unit_amount_minor: i64 = line_row.try_get("unit_amount_minor").map_err(read_error)?;
        let currency: String = line_row.try_get("currency").map_err(read_error)?;
        lines.push(OrderLine {
            sku,
            quantity: u32::try_from(quantity)
                .map_err(|_| AppError::Internal("a stored order line is negative".into()))?,
            unit_amount_minor,
            currency: currency_of(&currency)?,
        });
    }

    let event_rows = sqlx::query(
        "SELECT sequence, event, from_state, to_state \
         FROM order_event WHERE order_id = $1 ORDER BY sequence",
    )
    .bind(&order_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_error)?;
    let mut trail = Vec::with_capacity(event_rows.len());
    for event_row in event_rows {
        let sequence: i32 = event_row.try_get("sequence").map_err(read_error)?;
        let event: String = event_row.try_get("event").map_err(read_error)?;
        let from_state: String = event_row.try_get("from_state").map_err(read_error)?;
        let to_state: String = event_row.try_get("to_state").map_err(read_error)?;
        trail.push(TrailEntry {
            sequence: u32::try_from(sequence)
                .map_err(|_| AppError::Internal("a stored trail entry is negative".into()))?,
            event,
            from_state: state_of(&from_state)?,
            to_state: state_of(&to_state)?,
        });
    }

    let outbox_rows =
        sqlx::query("SELECT payload FROM outbox_event WHERE order_id = $1 ORDER BY id")
            .bind(&order_id)
            .fetch_all(&mut **tx)
            .await
            .map_err(db_error)?;
    let mut outbox = Vec::with_capacity(outbox_rows.len());
    for outbox_row in outbox_rows {
        let payload: String = outbox_row.try_get("payload").map_err(read_error)?;
        let event: DomainEvent = serde_json::from_str(&payload).map_err(|error| {
            AppError::Internal(format!("a stored domain event does not read: {error}"))
        })?;
        outbox.push(event);
    }

    Ok(Some(Order {
        order_id,
        buyer_id,
        state: state_of(&state)?,
        lines,
        total: crate::OrderTotal {
            amount_minor: total_amount_minor,
            currency: currency_of(&total_currency)?,
        },
        reservation_id,
        provider_ref,
        transitions_used: u32::try_from(transitions_used)
            .map_err(|_| AppError::Internal("a stored order's step count is negative".into()))?,
        trail,
        outbox,
        awaiting_reconciliation,
    }))
}

/// The order of a checkout, read out of the database.
async fn load_by_checkout(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    checkout_id: &str,
) -> Result<Option<Order>, AppError> {
    let row = sqlx::query("SELECT order_id FROM checkout WHERE checkout_id = $1")
        .bind(checkout_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let order_id: String = row.try_get("order_id").map_err(read_error)?;
    load_order(tx, &order_id).await
}

/// The orders, read out of the database.
async fn load_orders(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    buyer_id: Option<&str>,
    awaiting_reconciliation: Option<bool>,
) -> Result<Vec<Order>, AppError> {
    let mut ids = Vec::new();
    match (buyer_id, awaiting_reconciliation) {
        (Some(buyer), None) => {
            let rows = sqlx::query(
                "SELECT order_id FROM commerce_order WHERE buyer_id = $1 ORDER BY order_id",
            )
            .bind(buyer)
            .fetch_all(&mut **tx)
            .await
            .map_err(db_error)?;
            for row in rows {
                ids.push(row.try_get::<String, _>("order_id").map_err(read_error)?);
            }
        }
        (None, Some(flag)) => {
            let rows = sqlx::query(
                "SELECT order_id FROM commerce_order \
                 WHERE awaiting_reconciliation = $1 ORDER BY order_id",
            )
            .bind(flag)
            .fetch_all(&mut **tx)
            .await
            .map_err(db_error)?;
            for row in rows {
                ids.push(row.try_get::<String, _>("order_id").map_err(read_error)?);
            }
        }
        _ => {
            let rows = sqlx::query("SELECT order_id FROM commerce_order ORDER BY order_id")
                .fetch_all(&mut **tx)
                .await
                .map_err(db_error)?;
            for row in rows {
                ids.push(row.try_get::<String, _>("order_id").map_err(read_error)?);
            }
        }
    }

    let mut orders = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(order) = load_order(tx, &id).await? {
            orders.push(order);
        }
    }
    Ok(orders)
}

/// The idempotency record, read out of the database.
async fn load_idempotency(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &str,
    key: &str,
) -> Result<Option<IdempotencyRecord>, AppError> {
    let row = sqlx::query(
        "SELECT fingerprint, response, status, at_ms, expires_at_ms \
         FROM idempotency_record WHERE actor = $1 AND operation = $2 AND key = $3",
    )
    .bind(actor)
    .bind("checkout.submit")
    .bind(key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)?;
    let Some(row) = row else {
        return Ok(None);
    };

    let fingerprint: String = row.try_get("fingerprint").map_err(read_error)?;
    let response: String = row.try_get("response").map_err(read_error)?;
    let status_code: i32 = row.try_get("status").map_err(read_error)?;
    let at_ms: i64 = row.try_get("at_ms").map_err(read_error)?;
    let expires_at_ms: i64 = row.try_get("expires_at_ms").map_err(read_error)?;

    let body: Value = serde_json::from_str(&response)
        .map_err(|error| AppError::Internal(format!("a stored answer does not read: {error}")))?;
    Ok(Some(IdempotencyRecord {
        scoped_key: key.to_string(),
        request_fingerprint: fingerprint,
        state: IdempotencyState::Completed {
            status_code: u16::try_from(status_code).unwrap_or(0),
            response_body: body,
        },
        created_at_ms: at_ms,
        expires_at_ms,
    }))
}

/// The order, written back.
async fn save_order(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    order: &Order,
    now_ms: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO commerce_order \
         (order_id, buyer_id, state, total_amount_minor, total_currency, \
          created_at_ms, updated_at_ms, reservation_id, provider_ref, \
          transitions_used, awaiting_reconciliation) \
         VALUES ($1, $2, $3, $4, $5, $6, $6, $7, $8, $9, $10) \
         ON CONFLICT (order_id) DO UPDATE SET \
           buyer_id = EXCLUDED.buyer_id, state = EXCLUDED.state, \
           total_amount_minor = EXCLUDED.total_amount_minor, \
           total_currency = EXCLUDED.total_currency, \
           updated_at_ms = EXCLUDED.updated_at_ms, \
           reservation_id = EXCLUDED.reservation_id, \
           provider_ref = EXCLUDED.provider_ref, \
           transitions_used = EXCLUDED.transitions_used, \
           awaiting_reconciliation = EXCLUDED.awaiting_reconciliation",
    )
    .bind(&order.order_id)
    .bind(&order.buyer_id)
    .bind(order.state.as_str())
    .bind(order.total.amount_minor)
    .bind(order.total.currency.as_str())
    .bind(now_ms)
    .bind(order.reservation_id.as_deref())
    .bind(order.provider_ref.as_deref())
    .bind(i32::try_from(order.transitions_used).map_err(|_| {
        AppError::Internal("an order's step count does not fit the database".into())
    })?)
    .bind(order.awaiting_reconciliation)
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;

    // The lines are replaced, not merged: the order is the truth, and a line
    // that is not on it any more must not survive in the table.
    sqlx::query("DELETE FROM order_line WHERE order_id = $1")
        .bind(&order.order_id)
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
    for line in &order.lines {
        sqlx::query(
            "INSERT INTO order_line \
             (order_id, sku_id, quantity, unit_amount_minor, currency) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&order.order_id)
        .bind(&line.sku)
        .bind(i32::try_from(line.quantity).map_err(|_| {
            AppError::Internal("an order line's quantity does not fit the database".into())
        })?)
        .bind(line.unit_amount_minor)
        .bind(line.currency.as_str())
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
    }

    // The trail only grows: the entries that are already there stay, and the
    // new ones are added.
    for entry in &order.trail {
        sqlx::query(
            "INSERT INTO order_event (order_id, sequence, event, from_state, to_state, at_ms) \
             VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (order_id, sequence) DO NOTHING",
        )
        .bind(&order.order_id)
        .bind(i32::try_from(entry.sequence).map_err(|_| {
            AppError::Internal("a trail entry's sequence does not fit the database".into())
        })?)
        .bind(&entry.event)
        .bind(entry.from_state.as_str())
        .bind(entry.to_state.as_str())
        .bind(now_ms)
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
    }

    // The events that were not published yet are this order's, and they are
    // written again: the store is the truth about what the order produced.
    // The published ones are left alone — they have been sent.
    sqlx::query("DELETE FROM outbox_event WHERE order_id = $1 AND published_at_ms IS NULL")
        .bind(&order.order_id)
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
    for event in &order.outbox {
        let payload = serde_json::to_value(event).map_err(|error| {
            AppError::Internal(format!("a domain event does not encode: {error}"))
        })?;
        let topic = payload
            .get("type")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string();
        sqlx::query(
            "INSERT INTO outbox_event (topic, payload, published_at_ms, created_at_ms, order_id) \
             VALUES ($1, $2, NULL, $3, $4)",
        )
        .bind(&topic)
        .bind(payload.to_string())
        .bind(now_ms)
        .bind(&order.order_id)
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
    }

    Ok(())
}

/// The checkout, written back.
async fn save_checkout(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    checkout_id: &str,
    order_id: &str,
    state: CheckoutState,
    now_ms: i64,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO checkout (checkout_id, order_id, state, created_at_ms, updated_at_ms) \
         VALUES ($1, $2, $3, $4, $4) \
         ON CONFLICT (checkout_id) DO UPDATE SET \
           state = EXCLUDED.state, updated_at_ms = EXCLUDED.updated_at_ms",
    )
    .bind(checkout_id)
    .bind(order_id)
    .bind(state.as_str())
    .bind(now_ms)
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;
    Ok(())
}

/// The idempotency record, written back.
async fn save_idempotency(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &str,
    key: &str,
    record: &IdempotencyRecord,
) -> Result<(), AppError> {
    let (status_code, response) = match &record.state {
        IdempotencyState::InProgress => (0_i32, "{}".to_string()),
        IdempotencyState::Completed {
            status_code,
            response_body,
        } => (
            i32::from(*status_code),
            serde_json::to_string(response_body).map_err(|error| {
                AppError::Internal(format!("a stored answer does not encode: {error}"))
            })?,
        ),
    };
    sqlx::query(
        "INSERT INTO idempotency_record \
         (actor, operation, key, fingerprint, response, status, at_ms, expires_at_ms) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         ON CONFLICT (actor, operation, key) DO UPDATE SET \
           fingerprint = EXCLUDED.fingerprint, response = EXCLUDED.response, \
           status = EXCLUDED.status, at_ms = EXCLUDED.at_ms, \
           expires_at_ms = EXCLUDED.expires_at_ms",
    )
    .bind(actor)
    .bind("checkout.submit")
    .bind(key)
    .bind(&record.request_fingerprint)
    .bind(response)
    .bind(status_code)
    .bind(record.created_at_ms)
    .bind(record.expires_at_ms)
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;
    Ok(())
}

/// A currency, as it is stored.
///
/// The column is `char(3)`, so a stored value may carry padding; it is
/// trimmed before it is read.
fn currency_of(stored: &str) -> Result<commerce_core::Currency, AppError> {
    serde_json::from_value(Value::String(stored.trim().to_string()))
        .map_err(|error| AppError::Internal(format!("a stored currency is not one: {error}")))
}

/// A checkout state, as it is stored.
fn state_of(stored: &str) -> Result<CheckoutState, AppError> {
    serde_json::from_value(Value::String(stored.trim().to_string()))
        .map_err(|error| AppError::Internal(format!("a stored state is not one: {error}")))
}

/// A database failure, as a service error.
fn db_error(error: sqlx::Error) -> AppError {
    AppError::Internal(format!("the order store is unavailable: {error}"))
}

/// A field that would not read.
fn read_error(error: sqlx::Error) -> AppError {
    AppError::Internal(format!("could not read the order: {error}"))
}
