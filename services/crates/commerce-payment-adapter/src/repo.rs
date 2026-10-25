//! The payment attempts, in a store every instance of the service shares.
//!
//! ## Why the attempts, and not only the orders
//!
//! A Vercel Function may have several instances, and a store that lives in
//! one process's memory is therefore per-instance. A provider event that
//! arrives at one instance is invisible to the next, so a webhook can be
//! accepted twice — and the operator's payments list shows one attempt on
//! one instance and a different one on the next.
//!
//! ## What is the truth
//!
//! When a database is configured, **the database is the truth** and the
//! in-memory map is not consulted for an attempt this process did not serve
//! itself. A store that answered from its own memory while the database said
//! otherwise is the failure this whole design exists to prevent.
//!
//! ## The state machine is not touched
//!
//! The rules about the signature, the staleness window, the replay and the
//! outcome live in [`PaymentAdapter`], and they are the same rules whichever
//! store is underneath. The database-backed store reads what the request
//! needs, lets the in-memory state machine do its work, and writes the
//! result back.
//!
//! One rule is the database's, because it *has* to be: **the replay check**.
//! Whether a provider event was already taken is shared state, so the
//! database decides it — the primary key on the seen table makes two
//! instances accepting one event impossible.

use async_trait::async_trait;
use commerce_core::error::{status, AppError};
use commerce_core::{Currency, Money};
use sqlx::postgres::PgPool;
use sqlx::Row;

use crate::{
    AuthorizationOutcome, PaymentAdapter, PaymentAttempt, PaymentAttemptSummary, PaymentPage,
    SharedPayment, WebhookEvent, MAX_PAGE_LIMIT,
};

/// The payment adapter's store.
///
/// The service does not care whether the attempts live in a process's memory
/// or in a database: it asks this, and it gets the same answers either way.
#[async_trait]
pub trait PaymentRepository: Send + Sync {
    /// `payment.authorize` — never performs a real charge in tests/dev.
    ///
    /// The `payment_method_ref` is an opaque token; this service does not and
    /// cannot see card data.
    async fn authorize(
        &self,
        order_id: &str,
        payment_method_ref: &str,
        amount: Money,
    ) -> Result<PaymentAttempt, AppError>;

    /// `payment.reconcile` — the only exit from `Unknown`.
    async fn reconcile(
        &self,
        provider_ref: &str,
        provider_answer: AuthorizationOutcome,
    ) -> Result<PaymentAttempt, AppError>;

    /// The refund of an attempt.
    async fn refund(&self, provider_ref: &str) -> Result<PaymentAttempt, AppError>;

    /// `payment.webhook` — the provider's own answer.
    async fn verify_and_accept(
        &self,
        event: WebhookEvent,
        now_ms: i64,
    ) -> Result<PaymentAttempt, AppError>;

    /// One page of payment attempts, oldest first.
    async fn list(
        &self,
        order_id: Option<&str>,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<PaymentPage, AppError>;

    /// How many attempts are waiting for a verdict.
    async fn stuck_unknown_count(&self) -> Result<usize, AppError>;
}

/// The attempts in this process's memory.
///
/// The local run and the in-process edge use it on purpose: they are one
/// process, so there is nobody to share the attempts with.
pub struct MemoryPaymentRepository {
    store: SharedPayment,
}

impl MemoryPaymentRepository {
    pub fn new(store: SharedPayment) -> Self {
        Self { store }
    }
}

#[async_trait]
impl PaymentRepository for MemoryPaymentRepository {
    async fn authorize(
        &self,
        order_id: &str,
        payment_method_ref: &str,
        amount: Money,
    ) -> Result<PaymentAttempt, AppError> {
        let mut guard = self
            .store
            .write()
            .map_err(|_| AppError::Internal("payment lock poisoned".into()))?;
        guard.tick();
        guard.authorize(order_id, payment_method_ref, amount)
    }

    async fn reconcile(
        &self,
        provider_ref: &str,
        provider_answer: AuthorizationOutcome,
    ) -> Result<PaymentAttempt, AppError> {
        let mut guard = self
            .store
            .write()
            .map_err(|_| AppError::Internal("payment lock poisoned".into()))?;
        guard.tick();
        guard.reconcile(provider_ref, provider_answer)
    }

    async fn refund(&self, provider_ref: &str) -> Result<PaymentAttempt, AppError> {
        let mut guard = self
            .store
            .write()
            .map_err(|_| AppError::Internal("payment lock poisoned".into()))?;
        guard.tick();
        guard.refund(provider_ref)
    }

    async fn verify_and_accept(
        &self,
        event: WebhookEvent,
        now_ms: i64,
    ) -> Result<PaymentAttempt, AppError> {
        let mut guard = self
            .store
            .write()
            .map_err(|_| AppError::Internal("payment lock poisoned".into()))?;
        guard.verify_and_accept(event, now_ms)
    }

    async fn list(
        &self,
        order_id: Option<&str>,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<PaymentPage, AppError> {
        let guard = self
            .store
            .read()
            .map_err(|_| AppError::Internal("payment lock poisoned".into()))?;
        Ok(guard.list(order_id, cursor, limit))
    }

    async fn stuck_unknown_count(&self) -> Result<usize, AppError> {
        let guard = self
            .store
            .read()
            .map_err(|_| AppError::Internal("payment lock poisoned".into()))?;
        Ok(guard.stuck_unknown_count())
    }
}

/// The attempts in the database.
pub struct DbPaymentRepository {
    pool: PgPool,
    /// The provider's webhook secret. It signs and it verifies, and it never
    /// leaves this process.
    secret: String,
}

impl DbPaymentRepository {
    pub fn new(pool: PgPool, secret: impl Into<String>) -> Self {
        Self {
            pool,
            secret: secret.into(),
        }
    }
}

/// How many ids one authorization may need: the attempt and the provider
/// reference.
const ID_BLOCK: u64 = 8;

#[async_trait]
impl PaymentRepository for DbPaymentRepository {
    async fn authorize(
        &self,
        order_id: &str,
        payment_method_ref: &str,
        amount: Money,
    ) -> Result<PaymentAttempt, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let now_ms = commerce_core::idempotency::now_ms();

        // The store the rules live in. The provider script is the production
        // one: the scripted behaviours are for the tests and the dev
        // fixtures, and a real deployment always authorizes.
        let mut adapter = PaymentAdapter::new(self.secret.clone());
        adapter.now_ms = now_ms;

        let first_id = reserve_ids(&mut tx, ID_BLOCK).await?;
        adapter.sequence = first_id - 1;

        let attempt = adapter.authorize(order_id, payment_method_ref, amount)?;
        if adapter.sequence > first_id + ID_BLOCK - 1 {
            return Err(AppError::Internal(
                "the authorization needed more ids than the store reserved".into(),
            ));
        }

        sqlx::query(
            "INSERT INTO payment_attempt \
             (attempt_id, order_id, provider_ref, amount_minor, currency, state, \
              created_at_ms, authorized_at_ms, reconciliation_required) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&attempt.attempt_id)
        .bind(&attempt.order_id)
        .bind(&attempt.provider_ref)
        .bind(attempt.amount_minor)
        .bind(attempt.currency.as_str())
        .bind(state_text(attempt.status))
        .bind(now_ms)
        .bind(attempt.authorized_at_ms)
        .bind(attempt.reconciliation_required)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;

        tx.commit().await.map_err(db_error)?;
        Ok(attempt)
    }

    async fn reconcile(
        &self,
        provider_ref: &str,
        provider_answer: AuthorizationOutcome,
    ) -> Result<PaymentAttempt, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let now_ms = commerce_core::idempotency::now_ms();

        let attempt = load_attempt(&mut tx, provider_ref).await?.ok_or_else(|| {
            status::not_found(
                "payment.attempt_not_found",
                "provider_ref not found",
                "payment.reconcile",
            )
        })?;

        let mut adapter = PaymentAdapter::new(self.secret.clone());
        adapter.now_ms = now_ms;
        adapter.attempts.insert(provider_ref.to_string(), attempt);
        let reconciled = adapter.reconcile(provider_ref, provider_answer)?;

        save_attempt(&mut tx, &reconciled).await?;
        tx.commit().await.map_err(db_error)?;
        Ok(reconciled)
    }

    async fn refund(&self, provider_ref: &str) -> Result<PaymentAttempt, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let now_ms = commerce_core::idempotency::now_ms();

        let attempt = load_attempt(&mut tx, provider_ref).await?.ok_or_else(|| {
            status::not_found(
                "payment.attempt_not_found",
                "provider_ref not found",
                "payment.refund",
            )
        })?;

        let mut adapter = PaymentAdapter::new(self.secret.clone());
        adapter.now_ms = now_ms;
        adapter.attempts.insert(provider_ref.to_string(), attempt);
        let refunded = adapter.refund(provider_ref)?;

        save_attempt(&mut tx, &refunded).await?;
        tx.commit().await.map_err(db_error)?;
        Ok(refunded)
    }

    async fn verify_and_accept(
        &self,
        event: WebhookEvent,
        now_ms: i64,
    ) -> Result<PaymentAttempt, AppError> {
        const TRACE: &str = "payment.webhook";

        let mut tx = self.pool.begin().await.map_err(db_error)?;

        // The replay check is the database's, because it has to be: whether
        // the provider event was already taken is shared state, and the
        // primary key on the seen table makes two instances accepting one
        // event impossible.
        //
        // The insert and the attempt's change are one transaction, so an
        // event that cannot be applied — an unknown attempt, an unsupported
        // event type — leaves the event untaken. The in-memory store burned
        // it; burning it would mean the provider can never retry an event
        // that arrived before its attempt was written.
        let taken = sqlx::query(
            "INSERT INTO payment_webhook_seen (provider_event_id, seen_at_ms) \
             VALUES ($1, $2) ON CONFLICT (provider_event_id) DO NOTHING",
        )
        .bind(&event.provider_event_id)
        .bind(now_ms)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
        if taken.rows_affected() == 0 {
            return Err(status::conflict(
                "payment.webhook_replay",
                "provider event already processed",
                TRACE,
            ));
        }

        let attempt = load_attempt(&mut tx, &event.provider_ref)
            .await?
            .ok_or_else(|| {
                status::not_found("payment.attempt_not_found", "unknown provider_ref", TRACE)
            })?;

        let mut adapter = PaymentAdapter::new(self.secret.clone());
        adapter.now_ms = now_ms;
        adapter.attempts.insert(event.provider_ref.clone(), attempt);
        let accepted = adapter.verify_and_accept(event, now_ms)?;

        save_attempt(&mut tx, &accepted).await?;
        tx.commit().await.map_err(db_error)?;
        Ok(accepted)
    }

    async fn list(
        &self,
        order_id: Option<&str>,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<PaymentPage, AppError> {
        let limit = limit.clamp(1, MAX_PAGE_LIMIT as usize);

        // The page is ordered by the attempt id: the ids are zero padded and
        // monotonic, so the order is the creation order, and the cursor stays
        // valid while the store grows.
        let rows = sqlx::query(
            "SELECT attempt_id, order_id, provider_ref, amount_minor, currency, state, \
             created_at_ms, authorized_at_ms, reconciliation_required \
             FROM payment_attempt \
             WHERE ($1::text IS NULL OR order_id = $1) \
               AND ($2::text IS NULL OR attempt_id > $2) \
             ORDER BY attempt_id LIMIT $3",
        )
        .bind(order_id)
        .bind(cursor)
        .bind((limit + 1) as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;

        let mut items = Vec::with_capacity(rows.len().min(limit));
        let mut next_cursor = None;
        for row in rows.iter().take(limit) {
            items.push(summary_of(row)?);
        }
        if rows.len() > limit {
            // One more matching attempt is left over, so the caller gets a
            // cursor that starts at the page it did not get.
            next_cursor = items.last().map(|item| item.payment_attempt_id.clone());
        }

        Ok(PaymentPage { items, next_cursor })
    }

    async fn stuck_unknown_count(&self) -> Result<usize, AppError> {
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM payment_attempt WHERE state = 'unknown'")
                .fetch_one(&self.pool)
                .await
                .map_err(db_error)?;
        Ok(count as usize)
    }
}

/// The ids, reserved from the sequence.
async fn reserve_ids(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    count: u64,
) -> Result<u64, AppError> {
    let first: i64 = sqlx::query_scalar("SELECT nextval('payment_attempt_seq')")
        .fetch_one(&mut **tx)
        .await
        .map_err(db_error)?;
    for _ in 1..count {
        sqlx::query("SELECT nextval('payment_attempt_seq')")
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;
    }
    u64::try_from(first)
        .map_err(|_| AppError::Internal("the attempt sequence went backwards".into()))
}

/// The attempt, read out of the database.
async fn load_attempt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    provider_ref: &str,
) -> Result<Option<PaymentAttempt>, AppError> {
    let row = sqlx::query(
        "SELECT attempt_id, order_id, provider_ref, amount_minor, currency, state, \
         created_at_ms, authorized_at_ms, reconciliation_required \
         FROM payment_attempt WHERE provider_ref = $1",
    )
    .bind(provider_ref)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let state: String = row.try_get("state").map_err(read_error)?;
    let currency: String = row.try_get("currency").map_err(read_error)?;
    Ok(Some(PaymentAttempt {
        attempt_id: row.try_get("attempt_id").map_err(read_error)?,
        order_id: row.try_get("order_id").map_err(read_error)?,
        provider_ref: row.try_get("provider_ref").map_err(read_error)?,
        amount_minor: row.try_get("amount_minor").map_err(read_error)?,
        currency: currency_of(&currency)?,
        status: state_of(&state)?,
        authorized_at_ms: row.try_get("authorized_at_ms").map_err(read_error)?,
        reconciliation_required: row.try_get("reconciliation_required").map_err(read_error)?,
    }))
}

/// The attempt, written back.
async fn save_attempt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    attempt: &PaymentAttempt,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE payment_attempt SET state = $2, authorized_at_ms = $3, \
         reconciliation_required = $4 WHERE attempt_id = $1",
    )
    .bind(&attempt.attempt_id)
    .bind(state_text(attempt.status))
    .bind(attempt.authorized_at_ms)
    .bind(attempt.reconciliation_required)
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;
    Ok(())
}

/// The listing's projection of one attempt.
fn summary_of(row: &sqlx::postgres::PgRow) -> Result<PaymentAttemptSummary, AppError> {
    let state: String = row.try_get("state").map_err(read_error)?;
    let currency: String = row.try_get("currency").map_err(read_error)?;
    Ok(PaymentAttemptSummary {
        payment_attempt_id: row.try_get("attempt_id").map_err(read_error)?,
        order_id: row.try_get("order_id").map_err(read_error)?,
        status: state_of(&state)?,
        amount: crate::PaymentAmount {
            amount_minor: row.try_get("amount_minor").map_err(read_error)?,
            currency: currency_of(&currency)?,
        },
    })
}

/// A currency, as it is stored.
///
/// The column is `char(3)`, so a stored value may carry padding; it is
/// trimmed before it is read.
fn currency_of(stored: &str) -> Result<Currency, AppError> {
    serde_json::from_value(serde_json::Value::String(stored.trim().to_string()))
        .map_err(|error| AppError::Internal(format!("a stored currency is not one: {error}")))
}

/// An outcome, as it is stored.
fn state_text(state: AuthorizationOutcome) -> &'static str {
    match state {
        AuthorizationOutcome::Authorized => "authorized",
        AuthorizationOutcome::Declined => "declined",
        AuthorizationOutcome::Unknown => "unknown",
    }
}

/// An outcome, as it is read back.
fn state_of(stored: &str) -> Result<AuthorizationOutcome, AppError> {
    serde_json::from_value(serde_json::Value::String(stored.trim().to_string()))
        .map_err(|error| AppError::Internal(format!("a stored outcome is not one: {error}")))
}

/// A database failure, as a service error.
fn db_error(error: sqlx::Error) -> AppError {
    AppError::Internal(format!("the payment store is unavailable: {error}"))
}

/// A field that would not read.
fn read_error(error: sqlx::Error) -> AppError {
    AppError::Internal(format!("could not read the attempt: {error}"))
}
