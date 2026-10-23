//! Payment-provider mock — serves `CASE-CHAOS-001`.
//!
//! # What this mock is for
//!
//! The commerce invariants state that a payment attempt which *may* have
//! captured must never be reported as failed, and must never be retried blind.
//! The only way to prove that is to be able to produce a provider that has
//! actually taken the money and then fails to answer. This mock can do that via
//! [`Fault::drop_response`](crate::fault::Fault): the side effect is recorded,
//! the HTTP answer is discarded.
//!
//! # What this mock is NOT
//!
//! It is not a payment provider. It holds no keys, moves no money, and must
//! never be pointed at a production PSP. `[证据等级: Observed]` for its own
//! behaviour, `[Hypothetical]` for anything about real PSP semantics.

use crate::fault::{Fault, FaultController, FaultMode};
use axum::{extract::State, http::StatusCode, routing::{get, post}, Json, Router};
use commerce_core::error::AppError;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// The internal state of one charge.
///
/// `captured` is the truth. `response_delivered` records whether the caller
/// ever learned about it — the gap between the two is exactly `payment_unknown`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargeRecord {
    pub provider_ref: String,
    pub order_id: String,
    pub amount_minor: i64,
    pub currency: String,
    /// The provider definitely took the money.
    pub captured: bool,
    /// The caller received an answer. When false and `captured` is true, the
    /// caller is in `payment_unknown` and MUST reconcile.
    pub response_delivered: bool,
    /// How many times reconcile has been called.
    pub reconcile_calls: u64,
    pub refunded: bool,
    /// Tokenized reference the caller sent, echoed so a test can assert the
    /// mock never received card data.
    pub payment_method_ref_seen: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PspOutcome {
    /// Capture succeeds and the answer is returned.
    Authorize,
    /// The provider declines.
    Decline,
    /// The provider does not answer at all (timeout at the socket level).
    Timeout,
    /// Capture succeeds, then the connection drops before the answer is read.
    /// **The money is taken.** This is the case the whole design exists for.
    CaptureThenLoseResponse,
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeRequest {
    pub order_id: String,
    pub payment_method_ref: String,
    pub amount_minor: i64,
    pub currency: String,
    /// Selects the outcome. Defaults to `authorize`.
    #[serde(default)]
    pub outcome: Option<PspOutcome>,
}

#[derive(Debug, Deserialize)]
pub struct ReconcileRequest {
    pub provider_ref: String,
}

#[derive(Debug, Deserialize)]
pub struct RefundRequest {
    pub provider_ref: String,
}

#[derive(Debug, Default)]
pub struct PspState {
    charges: HashMap<String, ChargeRecord>,
    sequence: u64,
    faults: FaultController,
}

/// The shared, cloneable state for the mock PSP. Mirrors `SharedOrder` in
/// commerce-order so that `State<PspMock>` works identically to `State<SharedOrder>`.
#[derive(Clone)]
pub struct PspMock(pub Arc<RwLock<PspState>>);

impl PspMock {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(PspState::default())))
    }

    fn state(&self) -> &Arc<RwLock<PspState>> {
        &self.0
    }

    pub fn faults(&self) -> FaultController {
        self.state().read().expect("psp lock").faults.clone()
    }

    /// Test helper: the outcome the next authorize will produce unless a fault
    /// overrides it.
    pub async fn authorize(
        &self,
        req: AuthorizeRequest,
    ) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
        let outcome = req.outcome.unwrap_or(PspOutcome::Authorize);
        // The guard lives in its own scope: its lifetime ends before the
        // first await below. A `std::sync` guard that is alive across an
        // await makes the future !Send, and axum's `Handler` bound requires
        // Send — so "release the lock before you await" is not a style
        // preference here, it is the difference between compiling and not.
        let (provider_ref, record) = {
            let mut g = self
                .state()
                .write()
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({}))))?;

            g.sequence += 1;
            let provider_ref = format!("pspref_{:012}", g.sequence);

            let captured = matches!(
                outcome,
                PspOutcome::Authorize | PspOutcome::CaptureThenLoseResponse
            );
            let record = ChargeRecord {
                provider_ref: provider_ref.clone(),
                order_id: req.order_id.clone(),
                amount_minor: req.amount_minor,
                currency: req.currency.clone(),
                captured,
                // `CaptureThenLoseResponse` records the charge but never answers.
                response_delivered: !matches!(outcome, PspOutcome::CaptureThenLoseResponse),
                reconcile_calls: 0,
                refunded: false,
                payment_method_ref_seen: req.payment_method_ref.clone(),
            };
            g.charges.insert(provider_ref.clone(), record.clone());
            (provider_ref, record)
        };

        // A live fault overrides the scripted outcome — this is how a test
        // simulates the provider going down mid-scenario.
        //
        // The fault is bound to a variable **before** the `if let`: a
        // temporary in the `if let` scrutinee lives until the end of the
        // block, so the read guard inside `faults()` would still be held
        // across the `sleep().await` below — and a `std::sync` guard held
        // across an await makes the whole future `!Send`, which is exactly
        // what axum's `Handler` bound rejects.
        let live_fault = self.faults().take("psp.authorize");
        if let Some(f) = live_fault {
            if f.latency_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(f.latency_ms)).await;
            }
            if f.drop_response {
                // Side effect already recorded; the caller learns nothing.
                return Err((
                    StatusCode::from_u16(f.http_status).unwrap_or(StatusCode::SERVICE_UNAVAILABLE),
                    Json(serde_json::json!({ "code": "psp.unreachable", "captured": true })),
                ));
            }
            return Err((
                StatusCode::from_u16(f.http_status).unwrap_or(StatusCode::SERVICE_UNAVAILABLE),
                Json(serde_json::json!({ "code": "psp.down" })),
            ));
        }

        match outcome {
            PspOutcome::Decline => Ok(Json(serde_json::json!({
                "providerRef": provider_ref,
                "status": "declined",
                "reason": "mock_decline",
            }))),
            PspOutcome::Timeout => Err((
                StatusCode::GATEWAY_TIMEOUT,
                Json(serde_json::json!({ "code": "psp.timeout" })),
            )),
            _ => Ok(Json(serde_json::json!({
                "providerRef": provider_ref,
                "status": "authorized",
                "amountMinor": req.amount_minor,
                "currency": req.currency,
            }))),
        }
    }

    /// The authoritative query. A caller that lost a response calls this
    /// instead of re-authorizing.
    pub async fn reconcile(
        &self,
        provider_ref: &str,
    ) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
        let mut g = self
            .state()
            .write()
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({}))))?;
        let rec = g.charges.get_mut(provider_ref).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "code": "psp.unknown_ref" })),
            )
        })?;
        rec.reconcile_calls += 1;
        let status = if rec.captured { "authorized" } else { "declined" };
        Ok(Json(serde_json::json!({
            "providerRef": provider_ref,
            "status": status,
            "reconcileCalls": rec.reconcile_calls,
            "refunded": rec.refunded,
        })))
    }

    pub async fn refund(
        &self,
        provider_ref: &str,
    ) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
        let mut g = self
            .state()
            .write()
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({}))))?;
        let rec = g.charges.get_mut(provider_ref).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "code": "psp.unknown_ref" })),
            )
        })?;
        rec.refunded = true;
        rec.captured = false;
        Ok(Json(serde_json::json!({
            "providerRef": provider_ref,
            "status": "refunded",
        })))
    }

    /// Test-only inspection endpoint. Lets a test assert on what the mock
    /// really recorded instead of trusting the SUT's own report.
    pub async fn charges_dump(&self) -> Json<serde_json::Value> {
        let g = self.0.read().expect("psp lock");
        let mut v: Vec<_> = g.charges.values().cloned().collect();
        v.sort_by(|a, b| a.provider_ref.cmp(&b.provider_ref));
        Json(serde_json::json!({ "charges": v }))
    }

    pub async fn reset(&self) {
        self.0.write().expect("psp lock").charges.clear();
    }
}

/// The mock's internal `(StatusCode, Json<Value>)` error becomes an `AppError`
/// so axum can render it, matching what every real service already does.
fn psp_error(code: StatusCode, body: Json<serde_json::Value>) -> AppError {
    let detail = body
        .0
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or("psp.error")
        .to_string();
    let full = format!("{detail} {}", body.0);
    match code {
        StatusCode::NOT_FOUND => commerce_core::error::status::not_found(&detail, full, "mock-psp"),
        StatusCode::GATEWAY_TIMEOUT | StatusCode::SERVICE_UNAVAILABLE => {
            commerce_core::error::status::service_unavailable(&detail, full, "mock-psp")
        }
        _ => commerce_core::error::status::bad_request(&detail, full, "mock-psp"),
    }
}

// Signature shape mirrors `commerce-order`'s working handlers: `State` first,
// body extractor last, and a `Result` return so axum derives the Response.

async fn handle_authorize(
    State(mock): State<PspMock>,
    Json(req): Json<AuthorizeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    match mock.authorize(req).await {
        Ok(body) => Ok(body),
        Err((code, body)) => Err(psp_error(code, body)),
    }
}

async fn handle_reconcile(
    State(mock): State<PspMock>,
    Json(req): Json<ReconcileRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    match mock.reconcile(&req.provider_ref).await {
        Ok(body) => Ok(body),
        Err((code, body)) => Err(psp_error(code, body)),
    }
}

async fn handle_refund(
    State(mock): State<PspMock>,
    Json(req): Json<RefundRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    match mock.refund(&req.provider_ref).await {
        Ok(body) => Ok(body),
        Err((code, body)) => Err(psp_error(code, body)),
    }
}

async fn handle_charges(State(mock): State<PspMock>) -> Json<serde_json::Value> {
    mock.charges_dump().await
}

async fn handle_reset(State(mock): State<PspMock>) -> Json<serde_json::Value> {
    mock.reset().await;
    Json(serde_json::json!({ "ok": true }))
}

pub fn router(mock: PspMock) -> Router {
    Router::new()
        .route("/v1/psp/authorize", post(handle_authorize))
        .route("/v1/psp/reconcile", post(handle_reconcile))
        .route("/v1/psp/refund", post(handle_refund))
        .route("/__mock/charges", get(handle_charges))
        .route("/__mock/reset", post(handle_reset))
        .with_state(mock)
}

/// Convenience for tests: inject a fault without going through HTTP.
pub fn fault(target: &str, mode: FaultMode, count: u32, drop_response: bool) -> Fault {
    Fault {
        target: target.to_string(),
        mode,
        count,
        latency_ms: 0,
        http_status: 503,
        drop_response,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(outcome: PspOutcome) -> AuthorizeRequest {
        AuthorizeRequest {
            order_id: "ord-1".into(),
            payment_method_ref: "tok_mock".into(),
            amount_minor: 4800,
            currency: "JPY".into(),
            outcome: Some(outcome),
        }
    }

    #[tokio::test]
    async fn authorize_records_a_tokenized_ref_and_never_card_data() {
        let m = PspMock::new();
        let out = m.authorize(req(PspOutcome::Authorize)).await.unwrap();
        let body = out.0;
        let s = serde_json::to_string(&body).unwrap();
        assert!(!s.contains("4111"), "no card data may appear");
        assert!(body["providerRef"].as_str().unwrap().starts_with("pspref_"));
    }

    #[tokio::test]
    async fn capture_then_lose_response_leaves_the_money_taken_and_unreported() {
        let m = PspMock::new();
        // The caller gets an error...
        let err = m.authorize(req(PspOutcome::CaptureThenLoseResponse)).await.unwrap_err();
        assert_eq!(err.0, StatusCode::GATEWAY_TIMEOUT);
        // ...but the mock recorded that the charge happened.
        let dump = m.charges_dump().await;
        let charges = dump.0["charges"].as_array().unwrap();
        assert_eq!(charges.len(), 1);
        assert_eq!(charges[0]["captured"], true);
        assert_eq!(charges[0]["response_delivered"], false);
    }

    #[tokio::test]
    async fn reconcile_recovers_the_truth_the_caller_never_received() {
        let m = PspMock::new();
        let _ = m.authorize(req(PspOutcome::CaptureThenLoseResponse)).await;
        let dump = m.charges_dump().await;
        let psp_ref = dump.0["charges"][0]["providerRef"].as_str().unwrap().to_string();

        let out = m.reconcile(&psp_ref).await.unwrap();
        assert_eq!(out.0["status"], "authorized");
        assert_eq!(out.0["reconcileCalls"], 1);
    }

    #[tokio::test]
    async fn reconcile_of_an_unknown_ref_is_404_not_a_silent_success() {
        let m = PspMock::new();
        let err = m.reconcile("pspref_nope").await.unwrap_err();
        assert_eq!(err.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn decline_is_recorded_as_not_captured() {
        let m = PspMock::new();
        let out = m.authorize(req(PspOutcome::Decline)).await.unwrap();
        assert_eq!(out.0["status"], "declined");
        let dump = m.charges_dump().await;
        assert_eq!(dump.0["charges"][0]["captured"], false);
    }

    #[tokio::test]
    async fn refund_reverses_the_capture() {
        let m = PspMock::new();
        let out = m.authorize(req(PspOutcome::Authorize)).await.unwrap();
        let psp_ref = out.0["providerRef"].as_str().unwrap().to_string();
        let r = m.refund(&psp_ref).await.unwrap();
        assert_eq!(r.0["status"], "refunded");
        let dump = m.charges_dump().await;
        assert_eq!(dump.0["charges"][0]["captured"], false);
        assert_eq!(dump.0["charges"][0]["refunded"], true);
    }

    #[tokio::test]
    async fn injected_fault_overrides_the_scripted_outcome() {
        // CASE-CHAOS-001: the provider is up, then it goes down mid-scenario.
        let m = PspMock::new();
        m.faults().inject(fault("psp.authorize", FaultMode::NextN, 1, false));

        let err = m.authorize(req(PspOutcome::Authorize)).await.unwrap_err();
        assert_eq!(err.0, StatusCode::SERVICE_UNAVAILABLE);

        // Recovery: the next call succeeds with no intervention.
        let ok = m.authorize(req(PspOutcome::Authorize)).await.unwrap();
        assert_eq!(ok.0["status"], "authorized");
    }

    #[tokio::test]
    async fn drop_response_fault_takes_the_money_and_loses_the_answer() {
        let m = PspMock::new();
        m.faults()
            .inject(fault("psp.authorize", FaultMode::Always, 1, true));
        let err = m.authorize(req(PspOutcome::Authorize)).await.unwrap_err();
        assert_eq!(err.0, StatusCode::SERVICE_UNAVAILABLE);
        let dump = m.charges_dump().await;
        assert_eq!(dump.0["charges"][0]["captured"], true);
        assert_eq!(dump.0["charges"][0]["response_delivered"], false);
    }

    #[tokio::test]
    async fn reconcile_counts_calls_so_a_test_can_assert_no_retry_storm() {
        let m = PspMock::new();
        let out = m.authorize(req(PspOutcome::Authorize)).await.unwrap();
        let psp_ref = out.0["providerRef"].as_str().unwrap().to_string();
        for _ in 0..3 {
            m.reconcile(&psp_ref).await.unwrap();
        }
        let dump = m.charges_dump().await;
        assert_eq!(dump.0["charges"][0]["reconcile_calls"], 3);
        assert_eq!(
            dump.0["charges"][0]["reconcileCalls"], 3,
            "the camelCase projection is for HTTP callers"
        );
    }

    #[tokio::test]
    async fn reset_clears_recorded_charges() {
        let m = PspMock::new();
        m.authorize(req(PspOutcome::Authorize)).await.unwrap();
        m.reset().await;
        assert!(m.charges_dump().await.0["charges"].as_array().unwrap().is_empty());
    }
}
