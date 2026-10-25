//! Order ledger — serves `CASE-OPS-001` (backup / restore).
//!
//! # What it actually proves
//!
//! The RPO/RTO targets in `08-deployment/運用設計書.md` are **design values that
//! have never been exercised**. This mock makes the *shape* of that claim
//! testable without pretending a disaster drill happened:
//!
//! - `snapshot` produces a self-describing export of every order with its full
//!   transition trail — the artifact a backup would contain.
//! - `restore` rebuilds state from that export and a caller can assert the
//!   rebuilt trail is byte-identical to the original.
//!
//! That proves **the data is recoverable and the trail is complete enough to
//! replay**. It does *not* prove RPO 24h, RTO 4h, or that a real backup ran —
//! those stay `[Designed]` until an actual drill is run.
//!
//! A restore that silently drops orders or reorders the timeline would be the
//! failure mode worth catching here, because that is exactly what makes a
//! recovered system lie to a customer.

use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrailStep {
    pub sequence: u32,
    pub event: String,
    pub from_state: String,
    pub to_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LedgerEntry {
    pub order_id: String,
    pub buyer_id: String,
    pub state: String,
    pub currency: String,
    pub amount_minor: i64,
    /// Present while the provider verdict is still outstanding. A restore that
    /// loses this flag would re-charge or hide a stuck payment.
    pub awaiting_reconciliation: bool,
    pub trail: Vec<TrailStep>,
}

impl LedgerEntry {
    /// A stable digest of the *semantic* content, ignoring field order.
    ///
    /// `fingerprint` is what a test compares before and after a restore. It
    /// deliberately covers the trail, because a restore that preserved the
    /// final state but lost the history would be worse than useless during an
    /// audit.
    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(self.order_id.as_bytes());
        h.update(b"|");
        h.update(self.buyer_id.as_bytes());
        h.update(b"|");
        h.update(self.state.as_bytes());
        h.update(b"|");
        h.update(self.amount_minor.to_string().as_bytes());
        h.update(b"|");
        h.update(self.awaiting_reconciliation.to_string().as_bytes());
        h.update(b"|");
        for step in &self.trail {
            h.update(step.sequence.to_string().as_bytes());
            h.update(b",");
            h.update(step.event.as_bytes());
            h.update(b",");
            h.update(step.from_state.as_bytes());
            h.update(b">");
            h.update(step.to_state.as_bytes());
            h.update(b";");
        }
        format!("{:x}", h.finalize())
    }

    /// Timeline consistency: sequence must be 1..N with no gaps or repeats, and
    /// each step's `from_state` must match the previous step's `to_state`.
    ///
    /// A restored order that fails this is corrupt even if its final state
    /// looks right.
    pub fn trail_is_contiguous(&self) -> bool {
        for (i, step) in self.trail.iter().enumerate() {
            if step.sequence != (i as u32) + 1 {
                return false;
            }
            if i > 0 {
                let prev = &self.trail[i - 1];
                if prev.to_state != step.from_state {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RestoreReport {
    pub orders_seen: usize,
    pub orders_restored: usize,
    /// Non-empty means the export was incomplete. A partial restore that
    /// reports success is the worst outcome this mock exists to catch.
    pub missing: Vec<String>,
    pub fingerprints_match: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Default)]
pub struct LedgerMock {
    entries: Arc<RwLock<Vec<LedgerEntry>>>,
}

impl LedgerMock {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn seed_demo(&self) {
        let mut g = self.entries.write().expect("ledger lock");
        *g = vec![
            happy_order("ord-000001", "u-1", 9600),
            unknown_payment_order("ord-000002", "u-1", 4800),
            declined_order("ord-000003", "u-2", 12800),
        ];
    }

    pub async fn record(&self, entry: LedgerEntry) {
        let mut g = self.entries.write().expect("ledger lock");
        g.push(entry);
    }

    /// The backup artifact.
    pub async fn snapshot(&self) -> Json<serde_json::Value> {
        let g = self.entries.read().expect("ledger lock");
        Json(serde_json::json!({
            "exportedAt": chrono::Utc::now().to_rfc3339(),
            "schemaVersion": 1,
            "count": g.len(),
            "orders": *g,
            "fingerprints": g.iter().map(|e| e.fingerprint()).collect::<Vec<_>>(),
        }))
    }

    /// Rebuild from an export. Reports honestly rather than always claiming
    /// success.
    pub async fn restore(&self, orders: Vec<LedgerEntry>, expected: &[String]) -> RestoreReport {
        let mut warnings = Vec::new();
        let mut missing = Vec::new();

        let mut fingerprints = Vec::new();
        for e in &orders {
            if !e.trail_is_contiguous() {
                warnings.push(format!(
                    "{}: transition trail has a gap or discontinuity",
                    e.order_id
                ));
            }
            if e.state == "payment_unknown" && !e.awaiting_reconciliation {
                warnings.push(format!(
                    "{}: state is payment_unknown but the reconciliation flag was lost",
                    e.order_id
                ));
            }
            fingerprints.push(e.fingerprint());
        }

        for f in expected {
            if !fingerprints.contains(f) {
                missing.push(f.clone());
            }
        }

        {
            let mut g = self.entries.write().expect("ledger lock");
            *g = orders;
        }

        let fingerprints_match = missing.is_empty();
        RestoreReport {
            orders_seen: fingerprints.len(),
            // Every order in the export was restored. The orders that were
            // *absent* are already named in `missing`; subtracting them a
            // second time here would double-count the loss and report one
            // fewer restored order than actually came back.
            orders_restored: fingerprints.len(),
            missing,
            fingerprints_match,
            warnings,
        }
    }

    pub async fn dump(&self) -> Json<serde_json::Value> {
        let g = self.entries.read().expect("ledger lock");
        let mut entries: Vec<serde_json::Value> = g
            .iter()
            .map(|e| {
                serde_json::json!({
                    "orderId": e.order_id,
                    "state": e.state,
                    "awaitingReconciliation": e.awaiting_reconciliation,
                    "trailLength": e.trail.len(),
                    "trailContiguous": e.trail_is_contiguous(),
                    "fingerprint": e.fingerprint(),
                })
            })
            .collect();
        entries.sort_by(|a, b| {
            a["orderId"]
                .as_str()
                .unwrap()
                .cmp(b["orderId"].as_str().unwrap())
        });
        Json(serde_json::json!({ "orders": entries }))
    }

    pub async fn reset(&self) {
        self.entries.write().expect("ledger lock").clear();
    }
}

fn happy_order(id: &str, buyer: &str, amount: i64) -> LedgerEntry {
    LedgerEntry {
        order_id: id.into(),
        buyer_id: buyer.into(),
        state: "confirmed".into(),
        currency: "JPY".into(),
        amount_minor: amount,
        awaiting_reconciliation: false,
        trail: vec![
            TrailStep {
                sequence: 1,
                event: "Requested".into(),
                from_state: "new".into(),
                to_state: "new".into(),
            },
            TrailStep {
                sequence: 2,
                event: "BeginReserve".into(),
                from_state: "new".into(),
                to_state: "reserving".into(),
            },
            TrailStep {
                sequence: 3,
                event: "StockReserved".into(),
                from_state: "reserving".into(),
                to_state: "stock_reserved".into(),
            },
            TrailStep {
                sequence: 4,
                event: "RequestPayment".into(),
                from_state: "stock_reserved".into(),
                to_state: "awaiting_payment".into(),
            },
            TrailStep {
                sequence: 5,
                event: "PaymentAuthorized".into(),
                from_state: "awaiting_payment".into(),
                to_state: "confirming".into(),
            },
            TrailStep {
                sequence: 6,
                event: "Confirmed".into(),
                from_state: "confirming".into(),
                to_state: "confirmed".into(),
            },
        ],
    }
}

fn unknown_payment_order(id: &str, buyer: &str, amount: i64) -> LedgerEntry {
    LedgerEntry {
        order_id: id.into(),
        buyer_id: buyer.into(),
        state: "payment_unknown".into(),
        currency: "JPY".into(),
        amount_minor: amount,
        awaiting_reconciliation: true,
        trail: vec![
            TrailStep {
                sequence: 1,
                event: "Requested".into(),
                from_state: "new".into(),
                to_state: "new".into(),
            },
            TrailStep {
                sequence: 2,
                event: "BeginReserve".into(),
                from_state: "new".into(),
                to_state: "reserving".into(),
            },
            TrailStep {
                sequence: 3,
                event: "StockReserved".into(),
                from_state: "reserving".into(),
                to_state: "stock_reserved".into(),
            },
            TrailStep {
                sequence: 4,
                event: "RequestPayment".into(),
                from_state: "stock_reserved".into(),
                to_state: "awaiting_payment".into(),
            },
            TrailStep {
                sequence: 5,
                event: "PaymentTimedOut".into(),
                from_state: "awaiting_payment".into(),
                to_state: "payment_unknown".into(),
            },
        ],
    }
}

fn declined_order(id: &str, buyer: &str, amount: i64) -> LedgerEntry {
    LedgerEntry {
        order_id: id.into(),
        buyer_id: buyer.into(),
        state: "rejected".into(),
        currency: "JPY".into(),
        amount_minor: amount,
        awaiting_reconciliation: false,
        trail: vec![
            TrailStep {
                sequence: 1,
                event: "Requested".into(),
                from_state: "new".into(),
                to_state: "new".into(),
            },
            TrailStep {
                sequence: 2,
                event: "BeginReserve".into(),
                from_state: "new".into(),
                to_state: "reserving".into(),
            },
            TrailStep {
                sequence: 3,
                event: "StockReserved".into(),
                from_state: "reserving".into(),
                to_state: "stock_reserved".into(),
            },
            TrailStep {
                sequence: 4,
                event: "RequestPayment".into(),
                from_state: "stock_reserved".into(),
                to_state: "awaiting_payment".into(),
            },
            TrailStep {
                sequence: 5,
                event: "PaymentDeclined".into(),
                from_state: "awaiting_payment".into(),
                to_state: "rejected".into(),
            },
        ],
    }
}

#[derive(Debug, Deserialize)]
struct RestoreBody {
    orders: Vec<LedgerEntry>,
    #[serde(default)]
    expected_fingerprints: Vec<String>,
}

pub fn router(ledger: LedgerMock) -> Router {
    Router::new()
        .route(
            "/mockledger/snapshot",
            get({
                let l = ledger.clone();
                move || {
                    let l = l.clone();
                    async move { l.snapshot().await }
                }
            }),
        )
        .route(
            "/mockledger/restore",
            post({
                let l = ledger.clone();
                move |Json(b): Json<RestoreBody>| {
                    let l = l.clone();
                    async move {
                        let r = l.restore(b.orders, &b.expected_fingerprints).await;
                        (StatusCode::OK, Json(r))
                    }
                }
            }),
        )
        .route(
            "/mockledger/dump",
            get({
                let l = ledger.clone();
                move || {
                    let l = l.clone();
                    async move { l.dump().await }
                }
            }),
        )
        .route(
            "/mockledger/seed",
            post({
                let l = ledger.clone();
                move || {
                    let l = l.clone();
                    async move {
                        l.seed_demo().await;
                        Json(serde_json::json!({ "ok": true }))
                    }
                }
            }),
        )
        .route(
            "/__mock/reset",
            post({
                let l = ledger.clone();
                move || {
                    let l = l.clone();
                    async move {
                        l.reset().await;
                        Json(serde_json::json!({ "ok": true }))
                    }
                }
            }),
        )
        .route("/healthz", get(|| async { StatusCode::OK }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_fresh_export_restores_identically() {
        let l = LedgerMock::new();
        l.seed_demo().await;
        let export = l.snapshot().await;
        let orders: Vec<LedgerEntry> = serde_json::from_value(export.0["orders"].clone()).unwrap();
        let expected: Vec<String> = export.0["fingerprints"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();

        l.reset().await;
        let report = l.restore(orders, &expected).await;

        assert_eq!(report.orders_restored, 3);
        assert!(report.fingerprints_match);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }

    #[tokio::test]
    async fn a_partial_restore_is_reported_not_swallowed() {
        // The failure this catches: restoring "successfully" while silently
        // losing an order the customer was charged for.
        let l = LedgerMock::new();
        l.seed_demo().await;
        let export = l.snapshot().await;
        let mut orders: Vec<LedgerEntry> =
            serde_json::from_value(export.0["orders"].clone()).unwrap();
        let expected: Vec<String> = export.0["fingerprints"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();

        orders.retain(|o| o.order_id != "ord-000002"); // drop the unknown-payment one
        let report = l.restore(orders, &expected).await;

        assert!(!report.fingerprints_match);
        assert_eq!(report.missing.len(), 1, "the lost order must be named");
        assert_eq!(report.orders_restored, 2);
    }

    #[tokio::test]
    async fn losing_the_reconciliation_flag_is_a_warning() {
        let l = LedgerMock::new();
        l.seed_demo().await;
        let export = l.snapshot().await;
        let mut orders: Vec<LedgerEntry> =
            serde_json::from_value(export.0["orders"].clone()).unwrap();
        orders
            .iter_mut()
            .find(|o| o.order_id == "ord-000002")
            .unwrap()
            .awaiting_reconciliation = false;

        let report = l.restore(orders, &[]).await;
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("reconciliation flag was lost")));
    }

    #[tokio::test]
    async fn a_gap_in_the_trail_is_detected() {
        let l = LedgerMock::new();
        l.seed_demo().await;
        let export = l.snapshot().await;
        let mut orders: Vec<LedgerEntry> =
            serde_json::from_value(export.0["orders"].clone()).unwrap();
        orders[0].trail.remove(2); // punch a hole in the sequence

        let report = l.restore(orders, &[]).await;
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("gap or discontinuity")));
    }

    #[tokio::test]
    async fn a_discontinuous_trail_is_detected_even_with_a_full_sequence() {
        let e = LedgerEntry {
            order_id: "ord-x".into(),
            buyer_id: "u".into(),
            state: "confirmed".into(),
            currency: "JPY".into(),
            amount_minor: 100,
            awaiting_reconciliation: false,
            trail: vec![
                TrailStep {
                    sequence: 1,
                    event: "a".into(),
                    from_state: "new".into(),
                    to_state: "reserving".into(),
                },
                // jumps away from "reserving" without saying so
                TrailStep {
                    sequence: 2,
                    event: "b".into(),
                    from_state: "cancelled".into(),
                    to_state: "confirmed".into(),
                },
            ],
        };
        assert!(!e.trail_is_contiguous());
    }

    #[test]
    fn fingerprint_is_stable_and_sensitive() {
        let a = happy_order("o1", "u1", 100);
        let b = happy_order("o1", "u1", 100);
        assert_eq!(a.fingerprint(), b.fingerprint());

        let mut changed = happy_order("o1", "u1", 100);
        changed.amount_minor = 101;
        assert_ne!(a.fingerprint(), changed.fingerprint());
    }

    #[test]
    fn fingerprint_covers_the_trail_not_just_the_final_state() {
        // A restore that kept the final state but lost history must be caught.
        let full = happy_order("o1", "u1", 100);
        let mut truncated = full.clone();
        truncated.trail.pop();
        truncated.state = "confirmed".into();
        assert_ne!(full.fingerprint(), truncated.fingerprint());
    }

    #[test]
    fn seeded_trails_are_all_contiguous() {
        for e in [
            happy_order("a", "u", 1),
            unknown_payment_order("b", "u", 1),
            declined_order("c", "u", 1),
        ] {
            assert!(e.trail_is_contiguous(), "{} has a broken trail", e.order_id);
        }
    }

    #[tokio::test]
    async fn dump_exposes_per_order_health() {
        let l = LedgerMock::new();
        l.seed_demo().await;
        let d = l.dump().await;
        assert_eq!(d.0["orders"].as_array().unwrap().len(), 3);
        for o in d.0["orders"].as_array().unwrap() {
            assert_eq!(o["trailContiguous"], true);
        }
    }
}
