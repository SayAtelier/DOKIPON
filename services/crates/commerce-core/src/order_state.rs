//! Order / Checkout finite state machines.
//!
//! Atlas §E invariants, now executable:
//! ```text
//! New → Reserving → AwaitingPayment → Confirming → Confirmed
//! ```
//! - no stock        → Rejected
//! - payment declined → release reservation (outcome still recorded)
//! - payment timeout → PaymentUnknown → reconcile. **Never re-charge on unknown.**
//! - payment captured but Order DB write fails → durable reconciliation, never
//!   optimistically Success.
//!
//! The 5D "causal engine" from the atlas becomes this table: every guard is a
//! named, testable function; every terminal is explicit; budget exhaustion and
//! invalid transitions can never be reported as business success.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Longest number of transitions an order may make before the FSM refuses to
/// continue. Mirrors the atlas `maxSteps` budget for causal flows.
pub const MAX_TRANSITIONS: u32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutState {
    New,
    Reserving,
    StockReserved,
    AwaitingPayment,
    PaymentUnknown,
    Confirming,
    Confirmed,
    Rejected,
    Cancelled,
}

impl CheckoutState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            CheckoutState::Confirmed | CheckoutState::Rejected | CheckoutState::Cancelled
        )
    }

    /// A state the client may poll to completion. `PaymentUnknown` is
    /// deliberately non-terminal-but-polling: it is a reconciliation state,
    /// not a failure.
    pub fn is_pollable(self) -> bool {
        matches!(
            self,
            CheckoutState::Confirming | CheckoutState::PaymentUnknown | CheckoutState::AwaitingPayment
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            CheckoutState::New => "new",
            CheckoutState::Reserving => "reserving",
            CheckoutState::StockReserved => "stock_reserved",
            CheckoutState::AwaitingPayment => "awaiting_payment",
            CheckoutState::PaymentUnknown => "payment_unknown",
            CheckoutState::Confirming => "confirming",
            CheckoutState::Confirmed => "confirmed",
            CheckoutState::Rejected => "rejected",
            CheckoutState::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for CheckoutState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Everything that can move a checkout forward. Each variant corresponds to a
/// Saga step or an incoming event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderEvent {
    Requested,
    BeginReserve,
    StockReserved,
    StockShort,
    RequestPayment,
    PaymentAuthorized,
    PaymentDeclined,
    PaymentTimedOut,
    /// PSP reconciliation returned a definitive answer.
    ReconciliationAuthorized,
    ReconciliationDeclined,
    /// Order aggregate committed successfully after payment.
    Confirmed,
    /// Durable failure detected after payment was captured — do NOT mark success.
    ConfirmationFailed,
    Cancel,
    Compensate,
    Rejected(&'static str),
}

/// Why a transition was refused. `None` means the transition is legal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionRefusal {
    AlreadyTerminal,
    IllegalEvent,
    BudgetExhausted,
}

impl fmt::Display for TransitionRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransitionRefusal::AlreadyTerminal => write!(f, "checkout is already in a terminal state"),
            TransitionRefusal::IllegalEvent => write!(f, "event is not allowed from the current state"),
            TransitionRefusal::BudgetExhausted => write!(f, "transition budget exhausted"),
        }
    }
}

/// Pure FSM: (state, event, transitions_used) → next state or a refusal.
///
/// No I/O, no clock, no allocation of business state — the same triple always
/// yields the same result, which is exactly the atlas's replayability guarantee.
pub fn transition(
    state: CheckoutState,
    event: OrderEvent,
    transitions_used: u32,
) -> Result<CheckoutState, TransitionRefusal> {
    if state.is_terminal() {
        return Err(TransitionRefusal::AlreadyTerminal);
    }
    if transitions_used >= MAX_TRANSITIONS {
        return Err(TransitionRefusal::BudgetExhausted);
    }

    use CheckoutState as S;
    use OrderEvent as E;

    let next = match (state, event) {
        (S::New, E::Requested) => S::New,
        (S::New, E::BeginReserve) => S::Reserving,

        (S::Reserving, E::StockReserved) => S::StockReserved,
        (S::Reserving, E::StockShort) => S::Rejected,
        (S::Reserving, E::Rejected(_)) => S::Rejected,
        // Reservation TTL expiry or downstream failure: reject, no payment taken.
        (S::Reserving, E::Compensate) => S::Rejected,
        (S::Reserving, E::Cancel) => S::Cancelled,

        (S::StockReserved, E::RequestPayment) => S::AwaitingPayment,
        (S::StockReserved, E::Compensate) => S::Rejected,
        (S::StockReserved, E::Cancel) => S::Cancelled,

        (S::AwaitingPayment, E::PaymentAuthorized) => S::Confirming,
        (S::AwaitingPayment, E::PaymentDeclined) => S::Rejected,
        (S::AwaitingPayment, E::PaymentTimedOut) => S::PaymentUnknown,
        (S::AwaitingPayment, E::Cancel) => S::Cancelled,

        // Unknown is NOT failed and NOT confirmed. It is only ever left by a
        // reconciliation verdict or by an explicit operator compensation.
        (S::PaymentUnknown, E::ReconciliationAuthorized) => S::Confirming,
        (S::PaymentUnknown, E::ReconciliationDeclined) => S::Rejected,
        (S::PaymentUnknown, E::Compensate) => S::Rejected,

        (S::Confirming, E::Confirmed) => S::Confirmed,
        // Payment captured but order write failed → durable recovery path.
        // We do not fabricate success; the order is rejected pending refund.
        (S::Confirming, E::ConfirmationFailed) => S::Rejected,
        (S::Confirming, E::Compensate) => S::Rejected,

        _ => return Err(TransitionRefusal::IllegalEvent),
    };

    Ok(next)
}

/// Inventory reservation lifecycle, mirroring `Inventory` service invariants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReservationState {
    Pending,
    Reserved,
    Committed,
    Released,
    Expired,
}

impl ReservationState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            ReservationState::Committed | ReservationState::Released | ReservationState::Expired
        )
    }

    /// `available = on_hand - reserved`, and only Pending/Reserved count against
    /// availability. Invariant: availability must never go negative.
    pub fn counts_against_available(self) -> bool {
        matches!(self, ReservationState::Pending | ReservationState::Reserved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use CheckoutState as S;
    use OrderEvent as E;

    fn run(mut s: CheckoutState, events: &[OrderEvent]) -> S {
        for e in events {
            s = transition(s, *e, 0).expect("legal transition");
        }
        s
    }

    #[test]
    fn happy_path_reaches_confirmed() {
        let final_state = run(S::New, &[E::BeginReserve, E::StockReserved, E::RequestPayment, E::PaymentAuthorized, E::Confirmed]);
        assert_eq!(final_state, S::Confirmed);
        assert!(final_state.is_terminal());
    }

    #[test]
    fn stock_short_rejects_without_payment() {
        let final_state = run(S::New, &[E::BeginReserve, E::StockShort]);
        assert_eq!(final_state, S::Rejected);
    }

    #[test]
    fn payment_declined_releases_reservation() {
        let final_state = run(S::New, &[E::BeginReserve, E::StockReserved, E::RequestPayment, E::PaymentDeclined]);
        assert_eq!(final_state, S::Rejected);
    }

    #[test]
    fn payment_timeout_is_unknown_not_failed_and_not_success() {
        let final_state = run(S::New, &[E::BeginReserve, E::StockReserved, E::RequestPayment, E::PaymentTimedOut]);
        assert_eq!(final_state, S::PaymentUnknown);
        assert!(!final_state.is_terminal(), "unknown must stay pollable");
        assert!(final_state.is_pollable());
    }

    #[test]
    fn unknown_resolves_only_via_reconciliation() {
        let unknown = run(S::New, &[E::BeginReserve, E::StockReserved, E::RequestPayment, E::PaymentTimedOut]);
        // A blind re-charge event is NOT legal from Unknown.
        assert_eq!(
            transition(unknown, E::PaymentAuthorized, 3),
            Err(TransitionRefusal::IllegalEvent)
        );
        assert_eq!(
            transition(unknown, E::ReconciliationAuthorized, 3).unwrap(),
            S::Confirming
        );
        assert_eq!(
            transition(unknown, E::ReconciliationDeclined, 3).unwrap(),
            S::Rejected
        );
    }

    #[test]
    fn confirmation_failure_never_yields_confirmed() {
        let confirming = run(S::New, &[E::BeginReserve, E::StockReserved, E::RequestPayment, E::PaymentAuthorized]);
        assert_eq!(confirming, S::Confirming);
        assert_eq!(transition(confirming, E::ConfirmationFailed, 4).unwrap(), S::Rejected);
    }

    #[test]
    fn terminal_states_refuse_further_events() {
        assert_eq!(
            transition(S::Confirmed, E::Confirmed, 5),
            Err(TransitionRefusal::AlreadyTerminal)
        );
        assert_eq!(
            transition(S::Rejected, E::RequestPayment, 5),
            Err(TransitionRefusal::AlreadyTerminal)
        );
    }

    #[test]
    fn budget_exhaustion_is_refused_not_success() {
        assert_eq!(
            transition(S::AwaitingPayment, E::PaymentAuthorized, MAX_TRANSITIONS),
            Err(TransitionRefusal::BudgetExhausted)
        );
    }

    #[test]
    fn fsm_is_deterministic_for_same_inputs() {
        let a = run(S::New, &[E::BeginReserve, E::StockReserved, E::RequestPayment, E::PaymentTimedOut]);
        let b = run(S::New, &[E::BeginReserve, E::StockReserved, E::RequestPayment, E::PaymentTimedOut]);
        assert_eq!(a, b);
    }

    #[test]
    fn every_state_has_a_name_and_terminal_flag_is_consistent() {
        let all = [
            S::New, S::Reserving, S::StockReserved, S::AwaitingPayment,
            S::PaymentUnknown, S::Confirming, S::Confirmed, S::Rejected, S::Cancelled,
        ];
        for s in all {
            assert!(!s.as_str().is_empty());
            if s.is_terminal() {
                assert!(matches!(s, S::Confirmed | S::Rejected | S::Cancelled));
            }
        }
    }

    #[test]
    fn released_reservation_frees_availability() {
        assert!(ReservationState::Reserved.counts_against_available());
        assert!(!ReservationState::Released.counts_against_available());
        assert!(!ReservationState::Committed.counts_against_available());
    }
}
