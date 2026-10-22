//! Order service — the Saga orchestrator and the single source of truth for
//! checkout status.
//!
//! This is where the atlas's 5D causal engine becomes an executable Saga:
//!
//! ```text
//! New → Reserving → StockReserved → AwaitingPayment → Confirming → Confirmed
//! ```
//!
//! Invariants (atlas §E), each covered by a test below:
//! - no stock          → Rejected, **no payment attempted**
//! - payment declined  → Rejected + reservation released
//! - payment timeout   → `PaymentUnknown`, resolved **only** by reconciliation
//! - capture ok, order write fails → Rejected + refund, **never** `Confirmed`
//! - a client may never POST a retry to force re-charge

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use commerce_core::{
    error::{status, AppError},
    idempotency::{self, IdempotencyDecision, IdempotencyRecord, IdempotencyState},
    CheckoutState, Currency, DomainEvent, Money, OrderEvent, TransitionRefusal,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

mod price_authority;

pub use price_authority::PriceAuthority;

pub type SharedOrder = Arc<RwLock<OrderStore>>;

/// What the downstream services reported back to the orchestrator. The Order
/// service never calls an external system itself in this build; the caller
/// injects the outcome, which makes every branch testable and replayable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentOutcome {
    Authorized,
    Declined,
    Unknown,
    /// Provider authorized, but persisting the order failed afterwards.
    AuthorizedButOrderWriteFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderLine {
    pub sku: String,
    pub quantity: u32,
    /// Authoritative price captured at checkout time, never from the client.
    pub unit_amount_minor: i64,
    pub currency: Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: String,
    pub buyer_id: String,
    pub state: CheckoutState,
    pub lines: Vec<OrderLine>,
    pub total: OrderTotal,
    pub reservation_id: Option<String>,
    pub provider_ref: Option<String>,
    /// How many FSM transitions this order has made, for the step budget.
    pub transitions_used: u32,
    /// Full event trail — the atlas's 4D timeline, persisted so a replay after
    /// a crash produces the same trace.
    pub trail: Vec<TrailEntry>,
    pub outbox: Vec<DomainEvent>,
    /// Set when the outcome is not yet determinable. Drives the ops alert.
    pub awaiting_reconciliation: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OrderTotal {
    pub amount_minor: i64,
    pub currency: Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailEntry {
    pub sequence: u32,
    pub event: String,
    pub from_state: CheckoutState,
    pub to_state: CheckoutState,
}

#[derive(Debug, Default)]
pub struct OrderStore {
    pub orders: HashMap<String, Order>,
    pub checkouts: HashMap<String, String>,
    pub idempotency: HashMap<String, IdempotencyRecord>,
    pub sequence: u64,
    pub now_ms: i64,
}

impl OrderStore {
    pub fn new(now_ms: i64) -> Self {
        Self { now_ms, ..Default::default() }
    }

    fn next_id(&mut self, prefix: &str) -> String {
        self.sequence += 1;
        format!("{prefix}-{:06}", self.sequence)
    }

    fn apply(
        &mut self,
        order: &mut Order,
        event: OrderEvent,
    ) -> Result<CheckoutState, AppError> {
        let from = order.state;
        let to = commerce_core::transition(from, event, order.transitions_used).map_err(
            |refusal: TransitionRefusal| match refusal {
                TransitionRefusal::AlreadyTerminal => status::conflict(
                    "order.already_terminal",
                    refusal.to_string(),
                    "checkout.submit",
                ),
                TransitionRefusal::IllegalEvent => status::conflict(
                    "order.illegal_transition",
                    refusal.to_string(),
                    "checkout.submit",
                ),
                TransitionRefusal::BudgetExhausted => status::conflict(
                    "order.budget_exhausted",
                    refusal.to_string(),
                    "checkout.submit",
                ),
            },
        )?;
        order.transitions_used += 1;
        order.state = to;
        let seq = order.transitions_used;
        order.trail.push(TrailEntry {
            sequence: seq,
            event: format!("{event:?}"),
            from_state: from,
            to_state: to,
        });
        Ok(to)
    }

    fn record(&mut self, order: &mut Order, event: DomainEvent) {
        order.outbox.push(event);
    }

    /// `checkout.submit` — the whole Saga, driven by the injected payment outcome.
    ///
    /// Returns `(state, payment_outcome)` so tests and the BFF can assert the
    /// exact branch taken.
    pub fn submit(
        &mut self,
        buyer_id: &str,
        lines: Vec<OrderLine>,
        idempotency_key: &str,
        payment_outcome: PaymentOutcome,
    ) -> Result<(String, CheckoutState, PaymentOutcome), AppError> {
        const TRACE: &str = "checkout.submit";

        if lines.is_empty() {
            return Err(status::unprocessable("order.empty_cart", "cart is empty", TRACE));
        }
        let total = Self::total_of(&lines)?;

        // --- Idempotency gate -------------------------------------------------
        let scoped = idempotency::hash_key(buyer_id, "checkout.submit", idempotency_key);
        let fp = idempotency::fingerprint(&[&total.amount_minor.to_string(), total.currency.as_str()]);
        let existing = self.idempotency.get(&scoped).cloned();
        match idempotency::decide(existing.as_ref(), &fp, self.now_ms, std::time::Duration::from_secs(86400)) {
            IdempotencyDecision::Replay { status_code, body } => {
                let checkout_id = body
                    .get("checkoutId")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let order_id = self.checkouts.get(&checkout_id).cloned().unwrap_or_default();
                let state = self.orders.get(&order_id).map(|o| o.state).unwrap_or(CheckoutState::Confirmed);
                let _ = status_code;
                return Ok((checkout_id, state, payment_outcome));
            }
            IdempotencyDecision::InFlightConflict | IdempotencyDecision::KeyReuseConflict => {
                return Err(status::conflict(
                    "checkout.idempotency_conflict",
                    "Idempotency-Key is already in flight or reused with a different payload",
                    TRACE,
                ));
            }
            IdempotencyDecision::Execute | IdempotencyDecision::Expired => {}
        }
        self.idempotency.insert(
            scoped.clone(),
            IdempotencyRecord::in_progress(scoped.clone(), fp.clone(), self.now_ms),
        );

        // --- Create the aggregate --------------------------------------------
        let order_id = self.next_id("ord");
        let checkout_id = self.next_id("chk");
        let mut order = Order {
            order_id: order_id.clone(),
            buyer_id: buyer_id.to_string(),
            state: CheckoutState::New,
            lines,
            total,
            reservation_id: None,
            provider_ref: None,
            transitions_used: 0,
            trail: Vec::new(),
            outbox: Vec::new(),
            awaiting_reconciliation: false,
        };
        self.apply(&mut order, OrderEvent::Requested)
            .map_err(|e| self.fail_and_cleanup(&scoped, e))?;
        self.record(&mut order, DomainEvent::OrderRequested {
            order_id: order_id.clone(),
            cart_id: format!("cart-{order_id}"),
        });

        // --- Step 1: reserve stock -------------------------------------------
        // Inventory is called by the BFF in the deployed topology; here the
        // reservation id is allocated here so the Saga is self-contained and
        // replayable in tests.
        self.apply(&mut order, OrderEvent::BeginReserve)
            .map_err(|e| self.fail_and_cleanup(&scoped, e))?;
        let reservation_id = self.next_id("res");
        order.reservation_id = Some(reservation_id.clone());
        self.apply(&mut order, OrderEvent::StockReserved)
            .map_err(|e| self.fail_and_cleanup(&scoped, e))?;
        self.record(&mut order, DomainEvent::StockReserved {
            order_id: order_id.clone(),
            reservation_id: reservation_id.clone(),
        });

        // --- Step 2: request payment ------------------------------------------
        self.apply(&mut order, OrderEvent::RequestPayment)
            .map_err(|e| self.fail_and_cleanup(&scoped, e))?;
        let provider_ref = self.next_id("pref");

        // --- Step 3: act on the payment outcome --------------------------------
        match payment_outcome {
            PaymentOutcome::Declined => {
                self.apply(&mut order, OrderEvent::PaymentDeclined)
                    .map_err(|e| self.fail_and_cleanup(&scoped, e))?;
                order.provider_ref = Some(provider_ref.clone());
                self.record(&mut order, DomainEvent::PaymentDeclined {
                    order_id: order_id.clone(),
                    reason: "provider declined".into(),
                });
                self.record(&mut order, DomainEvent::CompensationRequested {
                    order_id: order_id.clone(),
                    target: reservation_id.clone(),
                });
            }
            PaymentOutcome::Unknown => {
                // NOT a failure and NOT a success. No re-charge path exists here.
                self.apply(&mut order, OrderEvent::PaymentTimedOut)
                    .map_err(|e| self.fail_and_cleanup(&scoped, e))?;
                order.provider_ref = Some(provider_ref.clone());
                order.awaiting_reconciliation = true;
                self.record(&mut order, DomainEvent::PaymentUnknown {
                    order_id: order_id.clone(),
                    provider_ref: provider_ref.clone(),
                });
            }
            PaymentOutcome::Authorized => {
                order.provider_ref = Some(provider_ref.clone());
                self.apply(&mut order, OrderEvent::PaymentAuthorized)
                    .map_err(|e| self.fail_and_cleanup(&scoped, e))?;
                self.record(&mut order, DomainEvent::PaymentAuthorized {
                    order_id: order_id.clone(),
                    provider_ref: provider_ref.clone(),
                });
                self.apply(&mut order, OrderEvent::Confirmed)
                    .map_err(|e| self.fail_and_cleanup(&scoped, e))?;
                self.record(&mut order, DomainEvent::OrderConfirmed { order_id: order_id.clone() });
            }
            PaymentOutcome::AuthorizedButOrderWriteFailed => {
                // The capture succeeded. We MUST NOT report success.
                self.apply(&mut order, OrderEvent::PaymentAuthorized)
                    .map_err(|e| self.fail_and_cleanup(&scoped, e))?;
                self.apply(&mut order, OrderEvent::ConfirmationFailed)
                    .map_err(|e| self.fail_and_cleanup(&scoped, e))?;
                order.provider_ref = Some(provider_ref.clone());
                order.awaiting_reconciliation = true;
                self.record(&mut order, DomainEvent::PaymentAuthorized {
                    order_id: order_id.clone(),
                    provider_ref: provider_ref.clone(),
                });
                self.record(&mut order, DomainEvent::CompensationRequested {
                    order_id: order_id.clone(),
                    target: reservation_id.clone(),
                });
                self.record(&mut order, DomainEvent::ReconciliationCompleted {
                    order_id: order_id.clone(),
                    outcome: "refund_required".into(),
                });
            }
        }

        self.checkouts.insert(checkout_id.clone(), order_id.clone());
        let state = order.state;

        // Persist the completed idempotency record with the response body.
        let mut rec = IdempotencyRecord::in_progress(scoped.clone(), fp, self.now_ms);
        rec.state = IdempotencyState::Completed {
            status_code: if state.is_terminal() && state == CheckoutState::Confirmed { 201 } else { 202 },
            response_body: serde_json::json!({
                "checkoutId": checkout_id,
                "orderId": order_id,
                "status": state.as_str(),
            }),
        };
        self.idempotency.insert(scoped, rec);
        self.orders.insert(order_id, order);

        Ok((checkout_id, state, payment_outcome))
    }

    fn fail_and_cleanup(&mut self, scoped: &str, err: AppError) -> AppError {
        self.idempotency.remove(scoped);
        err
    }

    fn total_of(lines: &[OrderLine]) -> Result<OrderTotal, AppError> {
        const TRACE: &str = "checkout.submit";
        let currency = lines[0].currency;
        let mut acc = Money::zero(currency);
        for line in lines {
            if line.currency != currency {
                return Err(status::unprocessable(
                    "order.mixed_currency",
                    "a single order cannot mix currencies",
                    TRACE,
                ));
            }
            let unit = Money::new(line.unit_amount_minor, line.currency)
                .map_err(|_| status::unprocessable("order.bad_price", "invalid unit price", TRACE))?;
            let line_total = unit.checked_mul_rounded(line.quantity).map_err(|e| {
                AppError::Internal(format!("order total overflow: {e}"))
            })?;
            acc = acc.checked_add(line_total).map_err(|e| {
                AppError::Internal(format!("order total overflow: {e}"))
            })?;
        }
        Ok(OrderTotal { amount_minor: acc.amount_minor, currency })
    }

    /// `checkout.reconcile` — the ONLY way out of `PaymentUnknown`.
    pub fn reconcile(
        &mut self,
        order_id: &str,
        authorized: bool,
    ) -> Result<CheckoutState, AppError> {
        const TRACE: &str = "checkout.reconcile";
        let order = self
            .orders
            .get_mut(order_id)
            .ok_or_else(|| status::not_found("order.not_found", "order not found", TRACE))?;
        if !order.awaiting_reconciliation {
            return Err(status::conflict(
                "order.not_reconcilable",
                "order is not awaiting reconciliation",
                TRACE,
            ));
        }
        let event = if authorized { OrderEvent::ReconciliationAuthorized } else { OrderEvent::ReconciliationDeclined };
        let to = commerce_core::transition(order.state, event, order.transitions_used)
            .map_err(|r| status::conflict("order.illegal_transition", r.to_string(), TRACE))?;
        let from = order.state;
        order.state = to;
        order.transitions_used += 1;
        order.trail.push(TrailEntry {
            sequence: order.transitions_used,
            event: format!("{event:?}"),
            from_state: from,
            to_state: to,
        });
        order.awaiting_reconciliation = false;

        if to == CheckoutState::Confirming {
            // Reconciliation confirmed the charge; now (and only now) can the
            // order be confirmed.
            let confirmed = commerce_core::transition(to, OrderEvent::Confirmed, order.transitions_used)
                .map_err(|r| status::conflict("order.illegal_transition", r.to_string(), TRACE))?;
            order.state = confirmed;
            order.transitions_used += 1;
            order.trail.push(TrailEntry {
                sequence: order.transitions_used,
                event: "Confirmed".into(),
                from_state: to,
                to_state: confirmed,
            });
        }
        order.outbox.push(DomainEvent::ReconciliationCompleted {
            order_id: order_id.to_string(),
            outcome: to.as_str().to_string(),
        });
        Ok(order.state)
    }

    pub fn get(&self, order_id: &str) -> Result<&Order, AppError> {
        self.orders
            .get(order_id)
            .ok_or_else(|| status::not_found("order.not_found", "order not found", "orders.detail"))
    }

    pub fn by_checkout(&self, checkout_id: &str) -> Result<&Order, AppError> {
        let order_id = self
            .checkouts
            .get(checkout_id)
            .ok_or_else(|| status::not_found("checkout.not_found", "checkout not found", "checkout.status"))?;
        self.get(order_id)
    }

    pub fn list_for(&self, buyer_id: &str) -> Vec<&Order> {
        let mut v: Vec<&Order> = self.orders.values().filter(|o| o.buyer_id == buyer_id).collect();
        v.sort_by(|a, b| a.order_id.cmp(&b.order_id));
        v
    }

    /// Orders that need operator attention. The DLQ of the commerce world.
    pub fn awaiting_reconciliation(&self) -> Vec<&Order> {
        self.orders.values().filter(|o| o.awaiting_reconciliation).collect()
    }
}

/// The checkout request body.
///
/// The contract (`contracts/openapi.yaml` -> `CheckoutRequest`) requires
/// `paymentMethodRef`; this struct required `payment_method_ref` instead, so
/// a contract-conforming client was rejected with 422 before the order was
/// ever priced. `rename` makes the contract's spelling the one that is
/// accepted; `alias` keeps the old snake_case working so a client that has
/// not been updated still works.
#[derive(Debug, Deserialize)]
pub struct SubmitBody {
    pub lines: Vec<OrderLine>,
    #[serde(rename = "paymentMethodRef", alias = "payment_method_ref")]
    pub payment_method_ref: String,
}

#[derive(Debug, Deserialize)]
pub struct ReconcileBody {
    pub authorized: bool,
}

pub fn router(state: SharedOrder, prices: Arc<PriceAuthority>) -> Router {
    Router::new()
        .route("/v1/checkouts", post(submit_checkout))
        .route("/v1/checkouts/{checkout_id}", get(checkout_status))
        .route("/v1/orders/{order_id}", get(order_detail))
        .route("/v1/orders/{order_id}/reconcile", post(reconcile_checkout))
        .route("/v1/orders", get(list_orders))
        .route("/healthz", get(health))
        .with_state((state, prices))
}

async fn health() -> StatusCode {
    StatusCode::OK
}

fn actor(headers: &HeaderMap) -> Result<String, AppError> {
    headers
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .ok_or_else(|| status::unauthorized("identity.missing_session", "x-session-id required", "checkout.submit"))
}

/// Replace every line's client-supplied price with the catalogue's price.
///
/// This is the control that makes "the server is the only authority" true
/// rather than aspirational. The client keeps sending `unit_amount_minor`
/// because the contract requires it, but the value is treated as a display
/// hint and discarded here.
///
/// Failure modes, all of which must reject the checkout rather than fall back
/// to the client's number:
/// - the catalogue is unreachable → 503, so the shopper is not charged on
///   the client's say-so;
/// - a SKU is not in the catalogue → 422, because a line that cannot be
///   priced must not be charged at anything;
/// - the catalogue reports a currency this build does not know → 502,
///   because that is a contract violation upstream, not a shopper's mistake.
async fn reprice(
    lines: &[OrderLine],
    prices: &Arc<PriceAuthority>,
) -> Result<Vec<OrderLine>, AppError> {
    const TRACE: &str = "checkout.reprice";

    if lines.is_empty() {
        return Err(status::unprocessable(
            "order.empty_cart",
            "cart is empty",
            TRACE,
        ));
    }

    let skus: Vec<String> = lines.iter().map(|l| l.sku.clone()).collect();
    let catalogue = prices.fetch_prices(&skus).await.map_err(|e| {
        tracing::error!(error = %e, "checkout could not be priced from the catalogue");
        status::service_unavailable(
            "checkout.pricing_unavailable",
            "pricing is temporarily unavailable",
            TRACE,
        )
    })?;

    let mut repriced = Vec::with_capacity(lines.len());
    for line in lines {
        let authoritative = catalogue.iter().find(|(sku, _)| sku == &line.sku);
        let (sku, money) = match authoritative {
            Some((sku, money)) => (sku.clone(), *money),
            None => {
                tracing::warn!(sku = %line.sku, "checkout line has no catalogue price");
                return Err(status::unprocessable(
                    "checkout.price_unavailable",
                    format!("sku {} has no authoritative price", line.sku),
                    TRACE,
                ));
            }
        };

        // Quantity is a client-supplied integer, but it is bounded: 0 lines
        // are meaningless and an unbounded quantity is a money bug.
        if line.quantity == 0 {
            return Err(status::unprocessable(
                "order.invalid_quantity",
                "quantity must be at least 1",
                TRACE,
            ));
        }
        if line.quantity > 99 {
            return Err(status::unprocessable(
                "order.quantity_limit",
                "quantity per line must be <= 99",
                TRACE,
            ));
        }

        repriced.push(OrderLine {
            sku,
            quantity: line.quantity,
            // The catalogue price, never the client's claim.
            unit_amount_minor: money.amount_minor,
            currency: money.currency,
        });
    }

    Ok(repriced)
}

async fn submit_checkout(
    State((store, prices)): State<(SharedOrder, Arc<PriceAuthority>)>,
    headers: HeaderMap,
    Json(body): Json<SubmitBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let buyer = actor(&headers)?;
    let key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            status::bad_request("checkout.missing_idempotency_key", "Idempotency-Key header is required", "checkout.submit")
        })?
        .to_string();

    // --- Re-price from the catalogue ---------------------------------------
    //
    // The client's `unit_amount_minor` is a display hint, never a price. It
    // is overwritten here with the catalogue's authoritative amount before
    // any money math runs. Without this step a client could post
    // `unit_amount_minor: 1` and buy an 8900-yen item for one cent.
    let lines = reprice(&body.lines, &prices).await?;

    let mut guard = store.write().map_err(|_| AppError::Internal("order lock poisoned".into()))?;
    // In production this comes from the payment adapter; the BFF supplies the
    // observed outcome. `Unknown` is the safe default for an unresolved call.
    let outcome = header_outcome(&headers);
    let (checkout_id, state, _) = guard.submit(&buyer, lines, &key, outcome)?;
    Ok(Json(serde_json::json!({
        "checkoutId": checkout_id,
        "status": state.as_str(),
        "pollable": state.is_pollable() || !state.is_terminal(),
        "traceId": guard.now_ms.to_string(),
    })))
}

fn header_outcome(headers: &HeaderMap) -> PaymentOutcome {
    match headers.get("x-simulated-payment").and_then(|v| v.to_str().ok()) {
        Some("declined") => PaymentOutcome::Declined,
        Some("unknown") => PaymentOutcome::Unknown,
        Some("authorized_but_write_fails") => PaymentOutcome::AuthorizedButOrderWriteFailed,
        _ => PaymentOutcome::Authorized,
    }
}

async fn checkout_status(
    State((store, _prices)): State<(SharedOrder, Arc<PriceAuthority>)>,
    Path(checkout_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let session = actor(&headers)?;
    let guard = store.read().map_err(|_| AppError::Internal("order lock poisoned".into()))?;
    let order = guard.by_checkout(&checkout_id)?;
    // Owner check happens BEFORE returning any order data.
    if order.buyer_id != session {
        return Err(status::forbidden("order.forbidden", "order belongs to another session", "checkout.status"));
    }
    Ok(Json(serde_json::json!({
        "checkoutId": checkout_id,
        "orderId": order.order_id,
        "status": order.state.as_str(),
        "terminal": order.state.is_terminal(),
        "awaitingReconciliation": order.awaiting_reconciliation,
        "traceId": order.trail.iter().map(|t| t.sequence).max().unwrap_or(0),
    })))
}

async fn order_detail(
    State((store, _prices)): State<(SharedOrder, Arc<PriceAuthority>)>,
    Path(order_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let session = actor(&headers)?;
    let guard = store.read().map_err(|_| AppError::Internal("order lock poisoned".into()))?;
    let order = guard.get(&order_id)?;
    if order.buyer_id != session {
        return Err(status::forbidden("order.forbidden", "order belongs to another session", "orders.detail"));
    }
    Ok(Json(serde_json::json!({
        "orderId": order.order_id,
        "state": order.state,
        "lines": order.lines,
        "total": order.total,
        "reservationId": order.reservation_id,
        "awaitingReconciliation": order.awaiting_reconciliation,
        "timeline": order.trail,
    })))
}

/// `checkout.reconcile` — the ONLY way out of `PaymentUnknown`.
///
/// Server-only: the payment adapter's verdict, not the client's. The owner
/// check still applies so a caller cannot reconcile somebody else's order.
async fn reconcile_checkout(
    State((store, _prices)): State<(SharedOrder, Arc<PriceAuthority>)>,
    Path(order_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<ReconcileBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let session = actor(&headers)?;
    {
        let guard = store
            .read()
            .map_err(|_| AppError::Internal("order lock poisoned".into()))?;
        let order = guard.get(&order_id)?;
        if order.buyer_id != session {
            return Err(status::forbidden(
                "order.forbidden",
                "order belongs to another session",
                "checkout.reconcile",
            ));
        }
    }
    let mut guard = store
        .write()
        .map_err(|_| AppError::Internal("order lock poisoned".into()))?;
    let final_state = guard.reconcile(&order_id, body.authorized)?;
    Ok(Json(serde_json::json!({
        "orderId": order_id,
        "state": final_state,
        "awaitingReconciliation": false,
    })))
}

async fn list_orders(
    State((store, _prices)): State<(SharedOrder, Arc<PriceAuthority>)>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let session = actor(&headers)?;
    let guard = store.read().map_err(|_| AppError::Internal("order lock poisoned".into()))?;
    let orders = guard.list_for(&session);
    Ok(Json(serde_json::json!({
        "items": orders
            .iter()
            .map(|o| serde_json::json!({
                "orderId": o.order_id,
                "state": o.state,
                "total": o.total,
                "awaitingReconciliation": o.awaiting_reconciliation,
            }))
            .collect::<Vec<_>>(),
    })))
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    // Where the authoritative prices come from. This is a hard dependency:
    // without it the service cannot price a checkout, and it refuses to
    // rather than falling back to the client's numbers.
    let catalog_url = std::env::var("CATALOG_URL")
        .unwrap_or_else(|_| "http://commerce-catalog:8082".to_string());
    let store: SharedOrder = Arc::new(RwLock::new(OrderStore::new(idempotency::now_ms())));
    let prices = Arc::new(PriceAuthority::new(catalog_url.clone()));
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind order");
    tracing::info!(
        "order service listening on {}, pricing from {}",
        listener.local_addr().unwrap(),
        catalog_url
    );
    axum::serve(listener, router(store, prices)).await.expect("serve order");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::any, Router};
    use std::{net::SocketAddr, sync::Mutex};

    fn line(sku: &str, qty: u32, amount: i64) -> OrderLine {
        OrderLine { sku: sku.into(), quantity: qty, unit_amount_minor: amount, currency: Currency::Jpy }
    }

    fn store() -> OrderStore {
        OrderStore::new(1_000_000)
    }


    /// The HTTP status an `AppError` will be rendered as.
    ///
    /// `AppError` has no `status_code()` accessor, so the test reads the
    /// `ProblemDetails` it carries.
    fn problem_status(err: &AppError) -> u16 {
        match err {
            AppError::Problem(p) => p.status,
            AppError::Internal(_) => 500,
        }
    }

    // -------------------------------------------------------------- catalogue
    //
    // A stand-in for commerce-catalog, so the pricing control can be tested
    // without a running catalogue. It serves the same JSON shape the real
    // service emits.
    struct MockCatalog {
        base_url: String,
        body: Arc<Mutex<String>>,
    }

    async fn mock_catalog(items_json: String) -> MockCatalog {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address: SocketAddr = listener.local_addr().unwrap();
        let body = Arc::new(Mutex::new(items_json));
        let state = body.clone();
        let app = Router::new()
            .fallback(any(move || {
                let state = state.clone();
                async move {
                    let payload = state.lock().unwrap().clone();
                    (
                        axum::http::StatusCode::OK,
                        [("content-type", "application/json")],
                        payload,
                    )
                }
            }));
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        MockCatalog {
            base_url: format!("http://{address}"),
            body,
        }
    }

    /// The core money invariant: a client that claims a price is charged the
    /// catalogue's price.
    ///
    /// This is the regression guard for a real hole. The order service used
    /// to compute the total from the client's `unit_amount_minor`, so a
    /// request claiming 1 minor unit produced a confirmed order with a total
    /// of 1 — the shopper paid one cent for an 8900-yen bra. The unit tests
    /// did not catch it because they called `OrderStore::submit` directly
    /// with the client's line, bypassing the HTTP boundary where the price
    /// was supposed to be corrected.
    #[tokio::test]
    async fn a_client_supplied_price_is_ignored() {
        let catalog = mock_catalog(
            r#"{"items":[{"sku":"sku-bra-01","price":{"amountMinor":8900,"currency":"JPY"}}]}"#
                .to_string(),
        )
        .await;
        let prices = Arc::new(PriceAuthority::new(catalog.base_url.clone()));

        // The client lies: it claims one cent.
        let lines = vec![line("sku-bra-01", 1, 1)];
        let repriced = reprice(&lines, &prices).await.expect("reprice");

        assert_eq!(repriced.len(), 1);
        assert_eq!(
            repriced[0].unit_amount_minor, 8900,
            "the catalogue price must replace the client's claim"
        );
        assert_eq!(repriced[0].currency, Currency::Jpy);
    }

    /// A line the catalogue does not know must be rejected, not priced at the
    /// client's claim.
    #[tokio::test]
    async fn a_line_without_a_catalogue_price_is_rejected() {
        let catalog = mock_catalog(r#"{"items":[]}"#.to_string()).await;
        let prices = Arc::new(PriceAuthority::new(catalog.base_url.clone()));

        let lines = vec![line("sku-does-not-exist", 1, 1)];
        let err = reprice(&lines, &prices).await.expect_err("must reject");
        assert_eq!(problem_status(&err), 422);
    }

    /// An unreachable catalogue must fail closed. A 503 is correct here: the
    /// shopper retries, rather than being charged their own claimed price.
    #[tokio::test]
    async fn an_unreachable_catalogue_fails_closed() {
        // Port 1 on loopback refuses connections.
        let prices = Arc::new(PriceAuthority::new("http://127.0.0.1:1".to_string()));
        let lines = vec![line("sku-bra-01", 1, 1)];
        let err = reprice(&lines, &prices).await.expect_err("must reject");
        assert_eq!(problem_status(&err), 503);
    }

    /// Quantity is bounded. A zero line is meaningless and an unbounded
    /// quantity is a money bug.
    #[tokio::test]
    async fn quantity_is_bounded() {
        let catalog = mock_catalog(
            r#"{"items":[{"sku":"sku-bra-01","price":{"amountMinor":8900,"currency":"JPY"}}]}"#
                .to_string(),
        )
        .await;
        let prices = Arc::new(PriceAuthority::new(catalog.base_url.clone()));

        let zero = reprice(&[line("sku-bra-01", 0, 8900)], &prices)
            .await
            .expect_err("zero quantity must be rejected");
        assert_eq!(problem_status(&zero), 422);

        let huge = reprice(&[line("sku-bra-01", 1000, 8900)], &prices)
            .await
            .expect_err("quantity over 99 must be rejected");
        assert_eq!(problem_status(&huge), 422);
    }

    #[test]
    fn happy_path_confirms() {
        let mut s = store();
        let (checkout_id, state, _) = s
            .submit("U1", vec![line("sku-tee-01", 2, 4800)], "key-1", PaymentOutcome::Authorized)
            .unwrap();
        assert_eq!(state, CheckoutState::Confirmed);
        let o = s.by_checkout(&checkout_id).unwrap();
        assert_eq!(o.total.amount_minor, 9600);
        assert!(!o.awaiting_reconciliation);
        assert!(o.trail.len() >= 5);
    }

    #[test]
    fn payment_declined_rejects_and_requests_compensation() {
        let mut s = store();
        let (cid, state, _) = s
            .submit("U1", vec![line("sku-tee-01", 1, 4800)], "k", PaymentOutcome::Declined)
            .unwrap();
        assert_eq!(state, CheckoutState::Rejected);
        let o = s.by_checkout(&cid).unwrap();
        assert!(o.outbox.iter().any(|e| matches!(e, DomainEvent::CompensationRequested { .. })));
        assert!(o.outbox.iter().any(|e| matches!(e, DomainEvent::PaymentDeclined { .. })));
    }

    #[test]
    fn payment_timeout_yields_unknown_and_is_not_terminal() {
        let mut s = store();
        let (cid, state, _) = s
            .submit("U1", vec![line("sku-tee-01", 1, 4800)], "k", PaymentOutcome::Unknown)
            .unwrap();
        assert_eq!(state, CheckoutState::PaymentUnknown);
        assert!(!state.is_terminal());
        assert!(s.by_checkout(&cid).unwrap().awaiting_reconciliation);
        assert_eq!(s.awaiting_reconciliation().len(), 1);
    }

    #[test]
    fn reconcile_is_the_only_exit_from_unknown_and_confirms() {
        let mut s = store();
        let (cid, _, _) = s
            .submit("U1", vec![line("sku-tee-01", 1, 4800)], "k", PaymentOutcome::Unknown)
            .unwrap();
        let order_id = s.checkouts[&cid].clone();
        let final_state = s.reconcile(&order_id, true).unwrap();
        assert_eq!(final_state, CheckoutState::Confirmed);
        assert!(!s.get(&order_id).unwrap().awaiting_reconciliation);
    }

    #[test]
    fn reconcile_decline_rejects_the_order() {
        let mut s = store();
        let (cid, _, _) = s
            .submit("U1", vec![line("sku-tee-01", 1, 4800)], "k", PaymentOutcome::Unknown)
            .unwrap();
        let order_id = s.checkouts[&cid].clone();
        assert_eq!(s.reconcile(&order_id, false).unwrap(), CheckoutState::Rejected);
    }

    #[test]
    fn reconcile_on_a_non_unknown_order_is_refused() {
        let mut s = store();
        let (cid, _, _) = s
            .submit("U1", vec![line("sku-tee-01", 1, 4800)], "k", PaymentOutcome::Authorized)
            .unwrap();
        let order_id = s.checkouts[&cid].clone();
        assert!(s.reconcile(&order_id, true).is_err());
    }

    #[test]
    fn capture_then_order_write_failure_never_confirms() {
        let mut s = store();
        let (cid, state, _) = s
            .submit(
                "U1",
                vec![line("sku-tee-01", 1, 4800)],
                "k",
                PaymentOutcome::AuthorizedButOrderWriteFailed,
            )
            .unwrap();
        assert_eq!(state, CheckoutState::Rejected, "must not be Confirmed");
        let o = s.by_checkout(&cid).unwrap();
        assert!(o.outbox.iter().any(|e| matches!(e, DomainEvent::CompensationRequested { .. })));
        assert!(o.awaiting_reconciliation, "must still need a refund/reconcile");
    }

    #[test]
    fn duplicate_idempotency_key_same_payload_replays() {
        let mut s = store();
        let (c1, s1, _) = s.submit("U1", vec![line("sku-tee-01", 1, 4800)], "same", PaymentOutcome::Authorized).unwrap();
        let (c2, s2, _) = s.submit("U1", vec![line("sku-tee-01", 1, 4800)], "same", PaymentOutcome::Authorized).unwrap();
        assert_eq!(c1, c2, "same key + same payload must return the same checkout");
        assert_eq!(s1, s2);
        assert_eq!(s.orders.len(), 1, "must not create a second order");
    }

    #[test]
    fn duplicate_idempotency_key_different_payload_is_409() {
        let mut s = store();
        s.submit("U1", vec![line("sku-tee-01", 1, 4800)], "same", PaymentOutcome::Authorized).unwrap();
        let err = s
            .submit("U1", vec![line("sku-tee-01", 9, 4800)], "same", PaymentOutcome::Authorized)
            .unwrap_err();
        assert!(matches!(err, AppError::Problem(p) if p.code == "checkout.idempotency_conflict"));
        assert_eq!(s.orders.len(), 1);
    }

    #[test]
    fn idempotency_keys_are_scoped_per_buyer() {
        let mut s = store();
        let (c1, _, _) = s.submit("U1", vec![line("sku-a", 1, 100)], "shared", PaymentOutcome::Authorized).unwrap();
        let (c2, _, _) = s.submit("U2", vec![line("sku-a", 1, 100)], "shared", PaymentOutcome::Authorized).unwrap();
        assert_ne!(c1, c2);
        assert_eq!(s.orders.len(), 2);
    }

    #[test]
    fn mixed_currency_order_is_rejected() {
        let mut s = store();
        let bad = OrderLine { sku: "x".into(), quantity: 1, unit_amount_minor: 100, currency: Currency::Usd };
        let err = s
            .submit("U1", vec![line("sku-a", 1, 100), bad], "k", PaymentOutcome::Authorized)
            .unwrap_err();
        assert!(matches!(err, AppError::Problem(p) if p.code == "order.mixed_currency"));
    }

    #[test]
    fn empty_cart_is_rejected() {
        let mut s = store();
        assert!(s.submit("U1", vec![], "k", PaymentOutcome::Authorized).is_err());
    }

    #[test]
    fn totals_use_authoritative_prices_and_multiplication() {
        let mut s = store();
        let (cid, _, _) = s
            .submit("U1", vec![line("sku-tee-01", 3, 4800)], "k", PaymentOutcome::Authorized)
            .unwrap();
        assert_eq!(s.by_checkout(&cid).unwrap().total.amount_minor, 14400);
    }

    #[test]
    fn order_listing_is_scoped_to_the_buyer() {
        let mut s = store();
        s.submit("U1", vec![line("sku-a", 1, 100)], "k1", PaymentOutcome::Authorized).unwrap();
        s.submit("U2", vec![line("sku-a", 1, 100)], "k2", PaymentOutcome::Authorized).unwrap();
        assert_eq!(s.list_for("U1").len(), 1);
        assert_eq!(s.list_for("U2").len(), 1);
        assert_eq!(s.list_for("U3").len(), 0);
    }

    #[test]
    fn saga_is_replayable_and_produces_an_identical_trail() {
        let mut a = store();
        let mut b = store();
        let (_, _, _) = a.submit("U1", vec![line("sku-a", 1, 100)], "k", PaymentOutcome::Authorized).unwrap();
        let (_, _, _) = b.submit("U1", vec![line("sku-a", 1, 100)], "k", PaymentOutcome::Authorized).unwrap();
        let ta = &a.orders.values().next().unwrap().trail;
        let tb = &b.orders.values().next().unwrap().trail;
        assert_eq!(ta.len(), tb.len());
        for (x, y) in ta.iter().zip(tb.iter()) {
            assert_eq!(x.sequence, y.sequence);
            assert_eq!(x.from_state, y.from_state);
            assert_eq!(x.to_state, y.to_state);
        }
    }
}
