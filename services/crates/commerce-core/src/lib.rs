//! Shared domain primitives: money, ids, problem details, ordering state machines.
//!
//! Derived from GemArc 5D Design Atlas `DATA_CONTRACT.md` / `AI_IMPLEMENTATION_BLUEPRINT.md` §C.
//! Every invariant documented in the atlas is enforced here as a Rust type or a
//! pure function with unit tests, so the contract cannot drift from the prose.

use serde::{Deserialize, Serialize};

pub mod error;
pub mod money;
pub mod order_state;
pub mod idempotency;
pub mod price_authority;
pub mod json_body;

pub use error::{AppError, ProblemDetails};
pub use money::{Money, Currency};
pub use order_state::{OrderEvent, transition, CheckoutState, ReservationState, TransitionRefusal};
pub use idempotency::{IdempotencyRecord, IdempotencyDecision, fingerprint, hash_key};
pub use price_authority::{PriceAuthority, PriceSource};
pub use json_body::{JsonBody, JsonBodyStrict};

/// Correlation / trace identifiers threaded through every request and event.
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    pub correlation_id: String,
    pub causation_id: Option<String>,
    pub traceparent: Option<String>,
}

impl RequestContext {
    pub fn new(correlation_id: impl Into<String>) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            causation_id: None,
            traceparent: None,
        }
    }
}

/// Envelope headers every domain event must carry (`API_EVENT_DATA_CONTRACTS.md`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventEnvelope<T: serde::Serialize> {
    pub event_id: uuid::Uuid,
    pub schema_version: u32,
    pub correlation_id: String,
    pub causation_id: Option<String>,
    pub traceparent: Option<String>,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    pub aggregate_id: String,
    pub payload: T,
}

pub const SCHEMA_VERSION: u32 = 1;

/// Domain events from the atlas Event/Outbox table, plus the commerce additions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum DomainEvent {
    OrderRequested { order_id: String, cart_id: String },
    StockReserved { order_id: String, reservation_id: String },
    StockRejected { order_id: String, reason: String },
    PaymentAuthorized { order_id: String, provider_ref: String },
    PaymentDeclined { order_id: String, reason: String },
    /// NOT a failure — an unresolvable PSP state. Never blind-retry; reconcile.
    PaymentUnknown { order_id: String, provider_ref: String },
    OrderConfirmed { order_id: String },
    OrderRejected { order_id: String, reason: String },
    CompensationRequested { order_id: String, target: String },
    ReconciliationCompleted { order_id: String, outcome: String },
}

impl DomainEvent {
    pub fn topic(&self) -> &'static str {
        match self {
            DomainEvent::OrderRequested { .. } => "order.requested",
            DomainEvent::StockReserved { .. } => "inventory.stock_reserved",
            DomainEvent::StockRejected { .. } => "inventory.stock_rejected",
            DomainEvent::PaymentAuthorized { .. } => "payment.authorized",
            DomainEvent::PaymentDeclined { .. } => "payment.declined",
            DomainEvent::PaymentUnknown { .. } => "payment.unknown",
            DomainEvent::OrderConfirmed { .. } => "order.confirmed",
            DomainEvent::OrderRejected { .. } => "order.rejected",
            DomainEvent::CompensationRequested { .. } => "order.compensation_requested",
            DomainEvent::ReconciliationCompleted { .. } => "payment.reconciled",
        }
    }

    pub fn aggregate_id(&self) -> &str {
        match self {
            DomainEvent::OrderRequested { order_id, .. }
            | DomainEvent::StockReserved { order_id, .. }
            | DomainEvent::StockRejected { order_id, .. }
            | DomainEvent::PaymentAuthorized { order_id, .. }
            | DomainEvent::PaymentDeclined { order_id, .. }
            | DomainEvent::PaymentUnknown { order_id, .. }
            | DomainEvent::OrderConfirmed { order_id }
            | DomainEvent::OrderRejected { order_id, .. }
            | DomainEvent::CompensationRequested { order_id, .. }
            | DomainEvent::ReconciliationCompleted { order_id, .. } => order_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_topics_are_stable_and_distinct() {
        let all = [
            DomainEvent::OrderRequested { order_id: "o".into(), cart_id: "c".into() },
            DomainEvent::StockReserved { order_id: "o".into(), reservation_id: "r".into() },
            DomainEvent::StockRejected { order_id: "o".into(), reason: "x".into() },
            DomainEvent::PaymentAuthorized { order_id: "o".into(), provider_ref: "p".into() },
            DomainEvent::PaymentDeclined { order_id: "o".into(), reason: "x".into() },
            DomainEvent::PaymentUnknown { order_id: "o".into(), provider_ref: "p".into() },
            DomainEvent::OrderConfirmed { order_id: "o".into() },
            DomainEvent::OrderRejected { order_id: "o".into(), reason: "x".into() },
            DomainEvent::CompensationRequested { order_id: "o".into(), target: "t".into() },
            DomainEvent::ReconciliationCompleted { order_id: "o".into(), outcome: "x".into() },
        ];
        let mut topics: Vec<_> = all.iter().map(|e| e.topic()).collect();
        topics.sort_unstable();
        let before = topics.len();
        topics.dedup();
        assert_eq!(before, topics.len(), "event topics must be unique");
        assert_eq!(SCHEMA_VERSION, 1);
    }
}
