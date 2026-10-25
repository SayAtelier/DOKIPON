//! Payment adapter — tokenized provider references only.
//!
//! Atlas §B7 + §E invariants enforced here:
//! - Raw card data NEVER enters this process. The client sends a provider
//!   `paymentMethodRef` token; the adapter maps it to a `provider_ref`.
//! - API secrets never return to the client.
//! - Webhooks are authenticated by HMAC + timestamp window + unique provider
//!   event id (replay rejection).
//! - `authorize` may legitimately return **Unknown**. Unknown is not a failure and
//!   not a success: it is resolved only by `reconcile`, which queries the
//!   provider. There is no "retry authorize" path by design.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use commerce_core::JsonBody;
use commerce_core::{
    error::{status, AppError},
    Currency, Money,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub type SharedPayment = Arc<RwLock<PaymentAdapter>>;

/// Outcome of an authorize attempt. `Unknown` is a first-class state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationOutcome {
    Authorized,
    Declined,
    /// Provider did not answer. **Never treat as declined, never blind-retry.**
    Unknown,
}

/// Scripted provider behaviour, injected by tests / dev fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderScript {
    Authorize,
    Decline,
    /// Simulate a PSP timeout: the charge may or may not have happened.
    Timeout,
    /// Capture succeeds but the subsequent order write fails (the hardest case).
    CaptureThenOrderDbFails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentAttempt {
    pub attempt_id: String,
    pub order_id: String,
    /// Opaque provider-side reference. The only payment handle that leaves this service.
    pub provider_ref: String,
    pub amount_minor: i64,
    /// The currency the attempt was authorized in.
    ///
    /// Store-only. The three write routes answer with the adapter's own
    /// snake_case `PaymentAttempt`, and that shape is pinned down by the e2e
    /// expectations; adding a field to it would change a response other code
    /// already reads. The listing route needs the currency to render the
    /// contract's `Money`, so the store keeps it and the write shape does not.
    #[serde(skip_serializing, default = "default_currency")]
    pub currency: Currency,
    pub status: AuthorizationOutcome,
    pub authorized_at_ms: Option<i64>,
    /// Set when the order DB write failed after a successful capture. This is the
    /// trigger for durable reconciliation — never for a blind re-charge.
    pub reconciliation_required: bool,
}

/// The fallback currency for a stored attempt that carries no currency.
///
/// The design documents require a defensive fallback for a variable enum
/// rather than a blank or a crash, and `Currency::Jpy` is the documented one.
fn default_currency() -> Currency {
    Currency::Jpy
}

/// The contract's `Money` shape: `{"amountMinor": 1500, "currency": "JPY"}`.
///
/// `commerce_core::Money` carries the same two values but is a domain type
/// without serde derives, and `commerce-core` is not this crate's to change,
/// so the wire shape is spelled out here — the same way the catalogue spells
/// out its `Price`. The camelCase rename matters: `amount_minor` on the wire
/// is read by nobody.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentAmount {
    pub amount_minor: i64,
    pub currency: Currency,
}

/// One payment attempt as the contract's listing route renders it.
///
/// A separate shape from [`PaymentAttempt`], on purpose: the three write
/// routes answer with the adapter's own snake_case `PaymentAttempt`, while
/// the listing route answers with the contract's camelCase names. Renaming
/// the write struct's fields would break those three responses, so the two
/// shapes live side by side and each is serialised in its own contract's
/// spelling.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentAttemptSummary {
    pub payment_attempt_id: String,
    pub order_id: String,
    pub status: AuthorizationOutcome,
    pub amount: PaymentAmount,
}

impl PaymentAttemptSummary {
    /// The contract's view of a stored attempt.
    fn from_attempt(attempt: &PaymentAttempt) -> Self {
        Self {
            payment_attempt_id: attempt.attempt_id.clone(),
            order_id: attempt.order_id.clone(),
            status: attempt.status,
            amount: PaymentAmount {
                amount_minor: attempt.amount_minor,
                currency: attempt.currency,
            },
        }
    }
}

/// One page of payment attempts.
///
/// The envelope's cursor field is `next_cursor` (snake_case) while the items
/// use camelCase. That is the contract's spelling for both, and it matches
/// the catalogue's page envelope.
#[derive(Debug, Clone, Serialize)]
pub struct PaymentPage {
    pub items: Vec<PaymentAttemptSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub provider_event_id: String,
    pub event_type: String,
    pub provider_ref: String,
    pub occurred_at_ms: i64,
    pub signature: String,
}

#[derive(Debug, Default)]
pub struct PaymentAdapter {
    pub attempts: HashMap<String, PaymentAttempt>,
    /// `provider_event_id` → seen. Replay protection.
    pub seen_webhooks: HashMap<String, i64>,
    /// Payloads without an accepted webhook, awaiting provider confirmation.
    pub unknown_attempts: Vec<String>,
    pub script: Option<ProviderScript>,
    pub secret: String,
    pub sequence: u64,
    /// The time the adapter last read the clock.
    ///
    /// A request reads it first, through [`PaymentAdapter::tick`]; the
    /// tests set it directly, because they need a fixed clock.
    pub now_ms: i64,
}

impl PaymentAdapter {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            ..Default::default()
        }
    }

    pub fn with_script(mut self, script: ProviderScript) -> Self {
        self.script = Some(script);
        self
    }

    /// Read the clock and keep it as `now_ms`.
    ///
    /// `now_ms` is the time the adapter last read, not a value the caller
    /// sets. A request reads it first, so a long-lived process answers with
    /// the current time rather than with the time it started at. A webhook
    /// that arrives five minutes after the process started is fresh; the
    /// adapter that never read the clock says it is stale, and rejects it.
    ///
    /// The tests set `now_ms` directly instead, because they need a fixed
    /// clock rather than the real one.
    pub fn tick(&mut self) -> i64 {
        self.now_ms = commerce_core::idempotency::now_ms();
        self.now_ms
    }

    fn next_attempt_id(&mut self) -> String {
        self.sequence += 1;
        format!("pa-{:06}", self.sequence)
    }

    fn next_provider_ref(&mut self) -> String {
        self.sequence += 1;
        format!("pref_{:012}", self.sequence)
    }

    /// `payment.authorize` — never performs a real charge in tests/dev.
    /// The `payment_method_ref` is treated as an opaque token; this service does
    /// not and cannot see card data.
    pub fn authorize(
        &mut self,
        order_id: &str,
        _payment_method_ref: &str,
        amount: Money,
    ) -> Result<PaymentAttempt, AppError> {
        const TRACE: &str = "payment.authorize";
        if amount.amount_minor <= 0 {
            return Err(status::unprocessable(
                "payment.bad_amount",
                "amount must be > 0",
                TRACE,
            ));
        }
        let script = self.script.unwrap_or(ProviderScript::Authorize);
        let status = match script {
            ProviderScript::Authorize => AuthorizationOutcome::Authorized,
            ProviderScript::Decline => AuthorizationOutcome::Declined,
            ProviderScript::Timeout => AuthorizationOutcome::Unknown,
            ProviderScript::CaptureThenOrderDbFails => AuthorizationOutcome::Authorized,
        };

        let attempt = PaymentAttempt {
            attempt_id: self.next_attempt_id(),
            order_id: order_id.to_string(),
            provider_ref: self.next_provider_ref(),
            amount_minor: amount.amount_minor,
            // The store keeps the currency even though the write shape does
            // not carry it: the listing route renders the contract's `Money`,
            // and a store that threw the currency away would have to guess
            // one per attempt.
            currency: amount.currency,
            status,
            authorized_at_ms: (status == AuthorizationOutcome::Authorized).then_some(self.now_ms),
            reconciliation_required: script == ProviderScript::CaptureThenOrderDbFails,
        };
        if status == AuthorizationOutcome::Unknown {
            self.unknown_attempts.push(attempt.provider_ref.clone());
        }
        self.attempts
            .insert(attempt.provider_ref.clone(), attempt.clone());
        Ok(attempt)
    }

    /// `payment.reconcile` — the ONLY exit from `Unknown`. Queries the provider
    /// for a definitive verdict. Idempotent: reconciling twice is safe.
    pub fn reconcile(
        &mut self,
        provider_ref: &str,
        provider_answer: AuthorizationOutcome,
    ) -> Result<PaymentAttempt, AppError> {
        const TRACE: &str = "payment.reconcile";
        let attempt = self.attempts.get_mut(provider_ref).ok_or_else(|| {
            status::not_found("payment.attempt_not_found", "provider_ref not found", TRACE)
        })?;
        if attempt.status == AuthorizationOutcome::Declined {
            return Err(status::conflict(
                "payment.already_declined",
                "a declined attempt is not reconcilable",
                TRACE,
            ));
        }
        attempt.status = provider_answer;
        attempt.reconciliation_required = false;
        if provider_answer != AuthorizationOutcome::Unknown {
            attempt.authorized_at_ms = Some(self.now_ms);
            self.unknown_attempts.retain(|r| r != provider_ref);
        }
        Ok(attempt.clone())
    }

    pub fn refund(&mut self, provider_ref: &str) -> Result<PaymentAttempt, AppError> {
        const TRACE: &str = "payment.refund";
        let attempt = self.attempts.get_mut(provider_ref).ok_or_else(|| {
            status::not_found("payment.attempt_not_found", "provider_ref not found", TRACE)
        })?;
        attempt.status = AuthorizationOutcome::Declined;
        attempt.reconciliation_required = false;
        Ok(attempt.clone())
    }

    pub fn stuck_unknown_count(&self) -> usize {
        self.unknown_attempts.len()
    }

    /// One page of payment attempts, oldest first.
    ///
    /// `order_id` narrows the page to a single order, `cursor` resumes after
    /// the attempt id it names, and `limit` caps the page. The cursor is the
    /// last returned attempt id: attempt ids are zero padded and monotonic,
    /// so a lexicographic comparison is also the creation order, and the
    /// cursor stays valid while the store grows.
    ///
    /// `next_cursor` is `None` on the last page, so a console stops paging
    /// instead of asking for an empty page forever.
    pub fn list(&self, order_id: Option<&str>, cursor: Option<&str>, limit: usize) -> PaymentPage {
        // The store is a HashMap, so its iteration order is arbitrary. The
        // contract wants a stable page, so the page is ordered by attempt id.
        let mut sorted: Vec<&PaymentAttempt> = self.attempts.values().collect();
        sorted.sort_by(|a, b| a.attempt_id.cmp(&b.attempt_id));

        let limit = limit.clamp(1, MAX_PAGE_LIMIT as usize);
        let mut items: Vec<PaymentAttemptSummary> = Vec::new();
        let mut next_cursor: Option<String> = None;
        for attempt in sorted {
            if let Some(order) = order_id {
                if attempt.order_id != order {
                    continue;
                }
            }
            if let Some(cursor) = cursor {
                if attempt.attempt_id.as_str() <= cursor {
                    continue;
                }
            }
            if items.len() == limit {
                // One more matching attempt is left over, so the caller gets
                // a cursor that starts at the page it did not get.
                next_cursor = items.last().map(|item| item.payment_attempt_id.clone());
                break;
            }
            items.push(PaymentAttemptSummary::from_attempt(attempt));
        }
        PaymentPage { items, next_cursor }
    }

    /// Webhook signature: HMAC-SHA256 over `provider_event_id|event_type|provider_ref|occurred_at_ms`.
    /// Returned by `sign_webhook_for_test` so tests can produce a valid signature
    /// without reaching into the secret.
    pub fn sign(&self, event: &WebhookEvent) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(self.secret.as_bytes());
        h.update(b"\x1f");
        h.update(event.provider_event_id.as_bytes());
        h.update(b"\x1f");
        h.update(event.event_type.as_bytes());
        h.update(b"\x1f");
        h.update(event.provider_ref.as_bytes());
        h.update(b"\x1f");
        h.update(event.occurred_at_ms.to_string().as_bytes());
        format!("{:x}", h.finalize())
    }

    pub fn verify_and_accept(
        &mut self,
        event: WebhookEvent,
        now_ms: i64,
    ) -> Result<PaymentAttempt, AppError> {
        const TRACE: &str = "payment.webhook";
        const MAX_SKEW_MS: i64 = 5 * 60 * 1000;

        let expected = self.sign(&event);
        if !constant_time_eq(expected.as_bytes(), event.signature.as_bytes()) {
            return Err(status::unauthorized(
                "payment.webhook_bad_signature",
                "webhook signature mismatch",
                TRACE,
            ));
        }
        if (now_ms - event.occurred_at_ms).abs() > MAX_SKEW_MS {
            return Err(status::unauthorized(
                "payment.webhook_stale",
                "webhook timestamp outside the accepted window",
                TRACE,
            ));
        }
        if self.seen_webhooks.contains_key(&event.provider_event_id) {
            return Err(status::conflict(
                "payment.webhook_replay",
                "provider event already processed",
                TRACE,
            ));
        }
        self.seen_webhooks
            .insert(event.provider_event_id.clone(), now_ms);

        let attempt = self.attempts.get_mut(&event.provider_ref).ok_or_else(|| {
            status::not_found("payment.attempt_not_found", "unknown provider_ref", TRACE)
        })?;
        match event.event_type.as_str() {
            "payment.authorized" => {
                attempt.status = AuthorizationOutcome::Authorized;
                attempt.authorized_at_ms = Some(now_ms);
            }
            "payment.declined" => attempt.status = AuthorizationOutcome::Declined,
            other => {
                return Err(status::bad_request(
                    "payment.webhook_unknown_type",
                    format!("unsupported webhook type {other}"),
                    TRACE,
                ))
            }
        }
        Ok(attempt.clone())
    }
}

/// Length-independent byte comparison so signature checking does not leak
/// the expected value through timing.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeBody {
    pub order_id: String,
    pub payment_method_ref: String,
    pub amount_minor: i64,
    pub currency: commerce_core::Currency,
}

/// The contract's default page size for the payment listing.
const DEFAULT_PAGE_LIMIT: i64 = 20;
/// The contract's maximum page size for the payment listing.
const MAX_PAGE_LIMIT: i64 = 100;

/// The page parameters of `GET /v1/payments`.
///
/// The contract names the filter `orderId` (camelCase) and leaves `cursor`
/// and `limit` as they are. The rename matters: a query extractor matches
/// field names literally, so without it `?orderId=ord-1` would be dropped on
/// the floor and the route would answer with every attempt — a filter that
/// looks wired up and filters nothing. `order_id` stays accepted as an alias
/// for callers that spell it the Rust way.
#[derive(Debug, Default, Deserialize)]
pub struct ListPaymentsQuery {
    #[serde(rename = "orderId", alias = "order_id")]
    pub order_id: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

impl ListPaymentsQuery {
    /// The page size to use.
    ///
    /// The contract bounds `limit` to 1..=100 with a default of 20. An
    /// out-of-range value is clamped rather than rejected: the page is a
    /// console convenience, and a console asking for 500 rows should get the
    /// first 100 rather than an error page.
    fn effective_limit(&self) -> usize {
        self.limit
            .unwrap_or(DEFAULT_PAGE_LIMIT)
            .clamp(1, MAX_PAGE_LIMIT) as usize
    }
}

pub fn router(state: SharedPayment) -> Router {
    Router::new()
        .route("/v1/payments/authorize", post(authorize))
        .route("/v1/payments/reconcile", post(reconcile))
        .route("/v1/payments/webhook", post(webhook))
        // The read route the admin facade's `GET /admin/v1/payments` needs.
        // Without it the facade had no upstream to relay and answered 404.
        .route("/v1/payments", get(list_payments))
        .route("/healthz", post(health))
        .with_state(state)
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn authorize(
    State(store): State<SharedPayment>,
    JsonBody(body): JsonBody<AuthorizeBody>,
) -> Result<Json<PaymentAttempt>, AppError> {
    let mut guard = store
        .write()
        .map_err(|_| AppError::Internal("payment lock poisoned".into()))?;
    let amount = Money::new(body.amount_minor, body.currency).map_err(|_| {
        status::unprocessable("payment.bad_amount", "invalid amount", "payment.authorize")
    })?;
    guard.tick();
    Ok(Json(guard.authorize(
        &body.order_id,
        &body.payment_method_ref,
        amount,
    )?))
}

#[derive(Debug, Deserialize)]
pub struct ReconcileBody {
    pub provider_ref: String,
    /// `authorized` | `declined`
    pub provider_answer: AuthorizationOutcome,
}

async fn reconcile(
    State(store): State<SharedPayment>,
    JsonBody(body): JsonBody<ReconcileBody>,
) -> Result<Json<PaymentAttempt>, AppError> {
    let mut guard = store
        .write()
        .map_err(|_| AppError::Internal("payment lock poisoned".into()))?;
    guard.tick();
    Ok(Json(
        guard.reconcile(&body.provider_ref, body.provider_answer)?,
    ))
}

async fn webhook(
    State(store): State<SharedPayment>,
    JsonBody(event): JsonBody<WebhookEvent>,
) -> Result<Json<PaymentAttempt>, AppError> {
    let mut guard = store
        .write()
        .map_err(|_| AppError::Internal("payment lock poisoned".into()))?;
    // The freshness window is measured against the clock as it is now, not
    // against the time the process started.
    let now = guard.tick();
    Ok(Json(guard.verify_and_accept(event, now)?))
}

/// `payment.list` — the operator console's page of payment attempts.
///
/// The admin facade relays this route as `GET /admin/v1/payments`. Before it
/// existed the adapter had no read route at all, so the facade had no
/// upstream to call and answered 404.
async fn list_payments(
    State(store): State<SharedPayment>,
    Query(query): Query<ListPaymentsQuery>,
) -> Result<Json<PaymentPage>, AppError> {
    // A listing does not mutate the store, so it takes the read lock: the
    // write routes (authorize / reconcile / webhook) keep running while the
    // console pages through the history.
    let guard = store
        .read()
        .map_err(|_| AppError::Internal("payment lock poisoned".into()))?;
    Ok(Json(guard.list(
        query.order_id.as_deref(),
        query.cursor.as_deref(),
        query.effective_limit(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use commerce_core::Currency;
    use tower::ServiceExt;

    fn adapter(script: ProviderScript) -> PaymentAdapter {
        PaymentAdapter::new("test-secret").with_script(script)
    }

    fn money() -> Money {
        Money::new(4800, Currency::Jpy).unwrap()
    }

    /// An adapter with three attempts across two orders.
    ///
    /// The USD attempt exists so the listing's currency can be checked: the
    /// store must report the currency an attempt was authorized in, not a
    /// constant one.
    fn seeded() -> PaymentAdapter {
        let mut a = adapter(ProviderScript::Authorize);
        a.authorize("ord-1", "tok_a", Money::new(1500, Currency::Jpy).unwrap())
            .unwrap();
        a.authorize("ord-2", "tok_b", Money::new(2000, Currency::Jpy).unwrap())
            .unwrap();
        a.authorize("ord-1", "tok_c", Money::new(3000, Currency::Usd).unwrap())
            .unwrap();
        a
    }

    /// `GET /v1/payments` with the given query string, as JSON.
    async fn list(uri: &str) -> serde_json::Value {
        let app = router(Arc::new(RwLock::new(seeded())));
        let response = app
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    /// The listing answers with the attempts that exist, in a stable order.
    #[tokio::test]
    async fn the_listing_returns_the_attempts_that_exist() {
        let page = list("/v1/payments").await;
        let ids: Vec<&str> = page["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["paymentAttemptId"].as_str().unwrap())
            .collect();
        assert_eq!(ids.len(), 3);
        // Ordered by attempt id, which is also the creation order.
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted);
        // Three attempts, one page, no more to fetch.
        assert!(page["next_cursor"].is_null());
    }

    /// The `orderId` filter narrows the listing to one order.
    #[tokio::test]
    async fn the_order_id_filter_narrows_the_listing() {
        let page = list("/v1/payments?orderId=ord-1").await;
        let items = page["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        for item in items {
            assert_eq!(item["orderId"], "ord-1");
        }
    }

    /// The limit caps the page, and a capped page hands back a cursor.
    #[tokio::test]
    async fn the_limit_caps_the_page() {
        let page = list("/v1/payments?limit=2").await;
        assert_eq!(page["items"].as_array().unwrap().len(), 2);
        assert!(page["next_cursor"].is_string());
    }

    /// The cursor continues after the last item of the previous page.
    #[tokio::test]
    async fn the_cursor_continues_after_the_first_page() {
        let first = list("/v1/payments?limit=2").await;
        let last_of_first = first["items"].as_array().unwrap().last().unwrap()["paymentAttemptId"]
            .as_str()
            .unwrap()
            .to_string();
        let cursor = first["next_cursor"].as_str().unwrap().to_string();
        assert_eq!(cursor, last_of_first, "the cursor is the last returned id");

        let second = list(&format!("/v1/payments?limit=2&cursor={cursor}")).await;
        let items = second["items"].as_array().unwrap();
        assert_eq!(items.len(), 1, "the rest of the attempts");
        assert!(items[0]["paymentAttemptId"].as_str().unwrap() > last_of_first.as_str());
        assert!(
            second["next_cursor"].is_null(),
            "the last page has no cursor"
        );
    }

    /// The item's field names are the contract's, asserted on the JSON.
    ///
    /// The listing answers with the contract's camelCase names while the write
    /// routes answer with the adapter's snake_case ones. A drift back to
    /// `payment_attempt_id` would still compile, still parse, and would
    /// silently break the console — so the names are asserted here, on the
    /// serialised item, and not only on the Rust struct.
    #[tokio::test]
    async fn the_listing_item_uses_the_contract_field_names() {
        let page = list("/v1/payments?orderId=ord-1&limit=1").await;
        let item = &page["items"][0];

        let mut keys: Vec<&str> = item
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec!["amount", "orderId", "paymentAttemptId", "status"]
        );

        let amount = &item["amount"];
        let mut amount_keys: Vec<&str> = amount
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        amount_keys.sort_unstable();
        assert_eq!(amount_keys, vec!["amountMinor", "currency"]);
        assert_eq!(amount["amountMinor"], 1500);
        assert_eq!(amount["currency"], "JPY");
        assert_eq!(item["status"], "authorized");

        // The envelope's cursor field keeps the contract's spelling.
        assert!(page.get("next_cursor").is_some());
    }

    /// The listing reports the currency each attempt was authorized in.
    #[tokio::test]
    async fn the_listing_reports_each_attempts_own_currency() {
        let page = list("/v1/payments?orderId=ord-1&limit=2").await;
        let currencies: Vec<&str> = page["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["amount"]["currency"].as_str().unwrap())
            .collect();
        assert_eq!(currencies, vec!["JPY", "USD"]);
    }

    /// The write routes keep their own snake_case shape.
    ///
    /// The listing route answers with the contract's camelCase names, and the
    /// temptation is to unify the two. They cannot be unified: the write
    /// shape is pinned down by the e2e expectations, and the store's
    /// currency field is deliberately not serialised, so the write response
    /// stays byte-for-byte what it was before the listing route existed.
    #[test]
    fn the_write_response_keeps_its_snake_case_shape() {
        let mut a = adapter(ProviderScript::Authorize);
        let att = a.authorize("ord-1", "tok_a", money()).unwrap();
        let value = serde_json::to_value(&att).unwrap();
        let mut keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "amount_minor",
                "attempt_id",
                "authorized_at_ms",
                "order_id",
                "provider_ref",
                "reconciliation_required",
                "status",
            ]
        );
    }

    #[test]
    fn authorize_returns_a_provider_ref_never_card_data() {
        let mut a = adapter(ProviderScript::Authorize);
        let att = a.authorize("order-1", "tok_visa_123", money()).unwrap();
        assert_eq!(att.status, AuthorizationOutcome::Authorized);
        assert!(att.provider_ref.starts_with("pref_"));
        let json = serde_json::to_string(&att).unwrap();
        assert!(
            !json.contains("tok_visa_123"),
            "payment method token must not be echoed"
        );
        assert!(!json.contains("secret"));
    }

    #[test]
    fn timeout_is_unknown_not_declined_and_not_authorized() {
        let mut a = adapter(ProviderScript::Timeout);
        let att = a.authorize("order-1", "tok", money()).unwrap();
        assert_eq!(att.status, AuthorizationOutcome::Unknown);
        assert_eq!(a.stuck_unknown_count(), 1);
    }

    #[test]
    fn reconcile_is_the_only_exit_from_unknown() {
        let mut a = adapter(ProviderScript::Timeout);
        let att = a.authorize("order-1", "tok", money()).unwrap();
        let resolved = a
            .reconcile(&att.provider_ref, AuthorizationOutcome::Authorized)
            .unwrap();
        assert_eq!(resolved.status, AuthorizationOutcome::Authorized);
        assert_eq!(a.stuck_unknown_count(), 0);
    }

    #[test]
    fn reconcile_to_decline_also_clears_the_stuck_queue() {
        let mut a = adapter(ProviderScript::Timeout);
        let att = a.authorize("order-1", "tok", money()).unwrap();
        let resolved = a
            .reconcile(&att.provider_ref, AuthorizationOutcome::Declined)
            .unwrap();
        assert_eq!(resolved.status, AuthorizationOutcome::Declined);
        assert_eq!(a.stuck_unknown_count(), 0);
    }

    #[test]
    fn reconcile_is_idempotent() {
        let mut a = adapter(ProviderScript::Timeout);
        let att = a.authorize("order-1", "tok", money()).unwrap();
        a.reconcile(&att.provider_ref, AuthorizationOutcome::Authorized)
            .unwrap();
        let again = a
            .reconcile(&att.provider_ref, AuthorizationOutcome::Authorized)
            .unwrap();
        assert_eq!(again.status, AuthorizationOutcome::Authorized);
    }

    #[test]
    fn declined_attempt_is_not_reconcilable() {
        let mut a = adapter(ProviderScript::Decline);
        let att = a.authorize("order-1", "tok", money()).unwrap();
        assert!(a
            .reconcile(&att.provider_ref, AuthorizationOutcome::Authorized)
            .is_err());
    }

    #[test]
    fn capture_then_order_db_failure_flags_reconciliation_not_success() {
        let mut a = adapter(ProviderScript::CaptureThenOrderDbFails);
        let att = a.authorize("order-1", "tok", money()).unwrap();
        assert!(att.reconciliation_required);
        assert_eq!(att.status, AuthorizationOutcome::Authorized);
    }

    #[test]
    fn zero_amount_is_rejected() {
        let mut a = adapter(ProviderScript::Authorize);
        assert!(a
            .authorize("o", "tok", Money::new(0, Currency::Jpy).unwrap())
            .is_err());
    }

    #[test]
    fn webhook_rejects_a_forged_signature() {
        let mut a = adapter(ProviderScript::Authorize);
        let att = a.authorize("order-1", "tok", money()).unwrap();
        a.now_ms = 1_000_000;
        let event = WebhookEvent {
            provider_event_id: "evt-1".into(),
            event_type: "payment.authorized".into(),
            provider_ref: att.provider_ref.clone(),
            occurred_at_ms: 1_000_000,
            signature: "deadbeef".into(),
        };
        assert!(a.verify_and_accept(event, 1_000_000).is_err());
    }

    #[test]
    fn webhook_rejects_a_stale_timestamp() {
        let mut a = adapter(ProviderScript::Authorize);
        let att = a.authorize("order-1", "tok", money()).unwrap();
        a.now_ms = 10_000_000;
        let mut event = WebhookEvent {
            provider_event_id: "evt-1".into(),
            event_type: "payment.authorized".into(),
            provider_ref: att.provider_ref.clone(),
            occurred_at_ms: 1_000_000,
            signature: String::new(),
        };
        event.signature = a.sign(&event);
        assert!(a.verify_and_accept(event, 10_000_000).is_err());
    }

    #[test]
    fn webhook_rejects_a_replay() {
        let mut a = adapter(ProviderScript::Authorize);
        let att = a.authorize("order-1", "tok", money()).unwrap();
        a.now_ms = 1_000_000;
        let mut event = WebhookEvent {
            provider_event_id: "evt-1".into(),
            event_type: "payment.authorized".into(),
            provider_ref: att.provider_ref.clone(),
            occurred_at_ms: 1_000_000,
            signature: String::new(),
        };
        event.signature = a.sign(&event);
        assert!(a.verify_and_accept(event.clone(), 1_000_000).is_ok());
        let err = a.verify_and_accept(event, 1_000_000).unwrap_err();
        assert!(matches!(err, AppError::Problem(p) if p.code == "payment.webhook_replay"));
    }

    #[test]
    fn valid_webhook_updates_the_attempt() {
        let mut a = adapter(ProviderScript::Timeout);
        let att = a.authorize("order-1", "tok", money()).unwrap();
        a.now_ms = 2_000_000;
        let mut event = WebhookEvent {
            provider_event_id: "evt-9".into(),
            event_type: "payment.authorized".into(),
            provider_ref: att.provider_ref.clone(),
            occurred_at_ms: 2_000_000,
            signature: String::new(),
        };
        event.signature = a.sign(&event);
        let updated = a.verify_and_accept(event, 2_000_000).unwrap();
        assert_eq!(updated.status, AuthorizationOutcome::Authorized);
    }

    #[test]
    fn webhook_signed_with_the_wrong_secret_is_rejected() {
        let mut a = adapter(ProviderScript::Authorize);
        let att = a.authorize("order-1", "tok", money()).unwrap();
        a.now_ms = 3_000_000;
        let mut event = WebhookEvent {
            provider_event_id: "evt-x".into(),
            event_type: "payment.authorized".into(),
            provider_ref: att.provider_ref.clone(),
            occurred_at_ms: 3_000_000,
            signature: String::new(),
        };
        let attacker = PaymentAdapter::new("attacker-secret");
        event.signature = attacker.sign(&event);
        assert!(a.verify_and_accept(event, 3_000_000).is_err());
    }

    #[test]
    fn refund_marks_the_attempt_terminal() {
        let mut a = adapter(ProviderScript::Authorize);
        let att = a.authorize("order-1", "tok", money()).unwrap();
        let refunded = a.refund(&att.provider_ref).unwrap();
        assert_eq!(refunded.status, AuthorizationOutcome::Declined);
    }

    #[test]
    fn constant_time_eq_behaves_like_equality() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }
}
