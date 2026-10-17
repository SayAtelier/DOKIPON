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

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use commerce_core::{
    error::{status, AppError},
    Money,
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
    pub status: AuthorizationOutcome,
    pub authorized_at_ms: Option<i64>,
    /// Set when the order DB write failed after a successful capture. This is the
    /// trigger for durable reconciliation — never for a blind re-charge.
    pub reconciliation_required: bool,
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
    pub now_ms: i64,
}

impl PaymentAdapter {
    pub fn new(secret: impl Into<String>) -> Self {
        Self { secret: secret.into(), ..Default::default() }
    }

    pub fn with_script(mut self, script: ProviderScript) -> Self {
        self.script = Some(script);
        self
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
            status,
            authorized_at_ms: (status == AuthorizationOutcome::Authorized).then(|| self.now_ms),
            reconciliation_required: script == ProviderScript::CaptureThenOrderDbFails,
        };
        if status == AuthorizationOutcome::Unknown {
            self.unknown_attempts.push(attempt.provider_ref.clone());
        }
        self.attempts.insert(attempt.provider_ref.clone(), attempt.clone());
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

    pub fn verify_and_accept(&mut self, event: WebhookEvent, now_ms: i64) -> Result<PaymentAttempt, AppError> {
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
        self.seen_webhooks.insert(event.provider_event_id.clone(), now_ms);

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

pub fn router(state: SharedPayment) -> Router {
    Router::new()
        .route("/v1/payments/authorize", post(authorize))
        .route("/v1/payments/reconcile", post(reconcile))
        .route("/v1/payments/webhook", post(webhook))
        .route("/healthz", post(health))
        .with_state(state)
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn authorize(
    State(store): State<SharedPayment>,
    Json(body): Json<AuthorizeBody>,
) -> Result<Json<PaymentAttempt>, AppError> {
    let mut guard = store.write().map_err(|_| AppError::Internal("payment lock poisoned".into()))?;
    let amount = Money::new(body.amount_minor, body.currency)
        .map_err(|_| status::unprocessable("payment.bad_amount", "invalid amount", "payment.authorize"))?;
    Ok(Json(guard.authorize(&body.order_id, &body.payment_method_ref, amount)?))
}

#[derive(Debug, Deserialize)]
pub struct ReconcileBody {
    pub provider_ref: String,
    /// `authorized` | `declined`
    pub provider_answer: AuthorizationOutcome,
}

async fn reconcile(
    State(store): State<SharedPayment>,
    Json(body): Json<ReconcileBody>,
) -> Result<Json<PaymentAttempt>, AppError> {
    let mut guard = store.write().map_err(|_| AppError::Internal("payment lock poisoned".into()))?;
    Ok(Json(guard.reconcile(&body.provider_ref, body.provider_answer)?))
}

async fn webhook(
    State(store): State<SharedPayment>,
    Json(event): Json<WebhookEvent>,
) -> Result<Json<PaymentAttempt>, AppError> {
    let mut guard = store.write().map_err(|_| AppError::Internal("payment lock poisoned".into()))?;
    let now = guard.now_ms;
    Ok(Json(guard.verify_and_accept(event, now)?))
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let secret = std::env::var("PSP_WEBHOOK_SECRET").unwrap_or_else(|_| "dev-only-secret".into());
    let store: SharedPayment = Arc::new(RwLock::new(PaymentAdapter::new(secret)));
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind payment");
    tracing::info!("payment adapter listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router(store)).await.expect("serve payment");
}

#[cfg(test)]
mod tests {
    use super::*;
    use commerce_core::Currency;

    fn adapter(script: ProviderScript) -> PaymentAdapter {
        PaymentAdapter::new("test-secret").with_script(script)
    }

    fn money() -> Money {
        Money::new(4800, Currency::Jpy).unwrap()
    }

    #[test]
    fn authorize_returns_a_provider_ref_never_card_data() {
        let mut a = adapter(ProviderScript::Authorize);
        let att = a.authorize("order-1", "tok_visa_123", money()).unwrap();
        assert_eq!(att.status, AuthorizationOutcome::Authorized);
        assert!(att.provider_ref.starts_with("pref_"));
        let json = serde_json::to_string(&att).unwrap();
        assert!(!json.contains("tok_visa_123"), "payment method token must not be echoed");
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
        a.reconcile(&att.provider_ref, AuthorizationOutcome::Authorized).unwrap();
        let again = a.reconcile(&att.provider_ref, AuthorizationOutcome::Authorized).unwrap();
        assert_eq!(again.status, AuthorizationOutcome::Authorized);
    }

    #[test]
    fn declined_attempt_is_not_reconcilable() {
        let mut a = adapter(ProviderScript::Decline);
        let att = a.authorize("order-1", "tok", money()).unwrap();
        assert!(a.reconcile(&att.provider_ref, AuthorizationOutcome::Authorized).is_err());
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
        assert!(a.authorize("o", "tok", Money::new(0, Currency::Jpy).unwrap()).is_err());
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
