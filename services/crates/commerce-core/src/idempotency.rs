//! Idempotency semantics from the atlas contract:
//!
//! > Idempotency-Key = UUID, scoped to (actor, operation), expiring duplicate control.
//! > A different payload under the same key returns 409.
//!
//! This module is pure: it decides what to do given a stored record and an
//! incoming request. Persistence is the caller's job (Postgres in the BFF).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdempotencyState {
    InProgress,
    Completed { status_code: u16, response_body: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    /// Scoped key: `sha256(actor_id || operation || client_key)`.
    pub scoped_key: String,
    pub request_fingerprint: String,
    pub state: IdempotencyState,
    pub created_at_ms: i64,
    pub expires_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyDecision {
    /// No prior record — caller must execute and then store a record.
    Execute,
    /// Same key, same payload, still in flight → 409 (do NOT re-execute).
    InFlightConflict,
    /// Same key, same payload, completed → replay the stored response.
    Replay { status_code: u16, body: serde_json::Value },
    /// Same key, different payload → 409 idempotency-key reuse.
    KeyReuseConflict,
    /// Record expired → caller may execute fresh.
    Expired,
}

const DEFAULT_TTL: Duration = Duration::from_secs(24 * 60 * 60);

pub fn hash_key(actor_id: &str, operation: &str, client_key: &str) -> String {
    let mut h = Sha256::new();
    h.update(actor_id.as_bytes());
    h.update(b"\x1f");
    h.update(operation.as_bytes());
    h.update(b"\x1f");
    h.update(client_key.as_bytes());
    format!("{:x}", h.finalize())
}

/// Canonical request fingerprint. Field order is fixed by this function,
/// so the same logical request always hashes the same regardless of JSON key
/// order on the wire.
pub fn fingerprint(parts: &[&str]) -> String {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p.as_bytes());
        h.update(b"\x1e");
    }
    format!("{:x}", h.finalize())
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

pub fn decide(
    existing: Option<&IdempotencyRecord>,
    incoming_fingerprint: &str,
    now: i64,
    ttl: Duration,
) -> IdempotencyDecision {
    // The stored `expires_at_ms` is authoritative (written when the record was
    // created); `ttl` is kept in the signature so callers can express an
    // explicit policy window and is asserted as a sanity bound below.
    debug_assert!(ttl.as_millis() as i64 > 0, "idempotency ttl must be positive");
    match existing {
        None => IdempotencyDecision::Execute,
        Some(rec) => {
            if rec.expires_at_ms <= now {
                return IdempotencyDecision::Expired;
            }
            if rec.request_fingerprint != incoming_fingerprint {
                return IdempotencyDecision::KeyReuseConflict;
            }
            match &rec.state {
                IdempotencyState::InProgress => IdempotencyDecision::InFlightConflict,
                IdempotencyState::Completed { status_code, response_body } => {
                    IdempotencyDecision::Replay {
                        status_code: *status_code,
                        body: response_body.clone(),
                    }
                }
            }
        }
    }
}

impl IdempotencyRecord {
    pub fn in_progress(scoped_key: String, request_fingerprint: String, now: i64) -> Self {
        Self {
            scoped_key,
            request_fingerprint,
            state: IdempotencyState::InProgress,
            created_at_ms: now,
            expires_at_ms: now + DEFAULT_TTL.as_millis() as i64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(fp: &str, state: IdempotencyState, expires: i64) -> IdempotencyRecord {
        IdempotencyRecord {
            scoped_key: "k".into(),
            request_fingerprint: fp.into(),
            state,
            created_at_ms: 0,
            expires_at_ms: expires,
        }
    }

    #[test]
    fn no_record_executes() {
        assert_eq!(decide(None, "fp", 1000, DEFAULT_TTL), IdempotencyDecision::Execute);
    }

    #[test]
    fn same_key_different_payload_is_conflict() {
        let r = rec("fp-a", IdempotencyState::InProgress, 10_000);
        assert_eq!(
            decide(Some(&r), "fp-b", 1000, DEFAULT_TTL),
            IdempotencyDecision::KeyReuseConflict
        );
    }

    #[test]
    fn in_progress_same_payload_is_conflict_not_replay() {
        let r = rec("fp", IdempotencyState::InProgress, 10_000);
        assert_eq!(
            decide(Some(&r), "fp", 1000, DEFAULT_TTL),
            IdempotencyDecision::InFlightConflict
        );
    }

    #[test]
    fn completed_replays_stored_response() {
        let body = serde_json::json!({"checkoutId": "c1"});
        let r = rec("fp", IdempotencyState::Completed { status_code: 201, response_body: body.clone() }, 10_000);
        assert_eq!(
            decide(Some(&r), "fp", 1000, DEFAULT_TTL),
            IdempotencyDecision::Replay { status_code: 201, body }
        );
    }

    #[test]
    fn expired_record_executes_again() {
        let r = rec("fp", IdempotencyState::InProgress, 500);
        assert_eq!(decide(Some(&r), "fp", 1000, DEFAULT_TTL), IdempotencyDecision::Expired);
    }

    #[test]
    fn scoped_key_separates_actors_and_operations() {
        let a = hash_key("user-1", "checkout.submit", "k");
        let b = hash_key("user-2", "checkout.submit", "k");
        let c = hash_key("user-1", "cart.update", "k");
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_eq!(a, hash_key("user-1", "checkout.submit", "k"));
    }

    #[test]
    fn fingerprint_is_order_stable_and_separator_safe() {
        assert_eq!(fingerprint(&["a", "b"]), fingerprint(&["a", "b"]));
        assert_ne!(fingerprint(&["a", "b"]), fingerprint(&["b", "a"]));
        assert_ne!(fingerprint(&["ab", "c"]), fingerprint(&["a", "bc"]));
    }
}
