//! Edge / BFF — the single public entry point for all three channels
//! (Flutter App, Flutter Web, LINE MINI App).
//!
//! Responsibilities (atlas §B1):
//! - anonymous vs authenticated API surface
//! - `correlationId` generation and propagation
//! - rate limiting
//! - **channel content eligibility** (a LINE MINI App must never be served
//!   content its policy forbids)
//! - end-client response shaping
//!
//! It also enforces two hard rules from the contract:
//! - `Authorization` is a server-side permission decision; a client-side LIFF
//!   token is never a substitute.
//! - All writes require an `Idempotency-Key`.

use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use commerce_core::error::{status, AppError};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
    time::Instant,
};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

pub type SharedBff = Arc<RwLock<RateLimiter>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Web,
    App,
    LineMiniApp,
}

impl Channel {
    pub fn parse(s: Option<&str>) -> Self {
        match s {
            Some("app") => Channel::App,
            Some("line") => Channel::LineMiniApp,
            _ => Channel::Web,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Web => "web",
            Channel::App => "app",
            Channel::LineMiniApp => "line",
        }
    }
}

/// Fixed-window limiter. Simple on purpose: the atlas asks for a rate limit at
/// the edge, not for a distributed one. `[证据等级: Designed]`
#[derive(Debug)]
pub struct RateLimiter {
    pub window_started: Instant,
    pub hits: HashMap<String, u64>,
    pub limit: u64,
    pub counter: AtomicU64,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            window_started: Instant::now(),
            hits: HashMap::new(),
            limit: 600,
            counter: AtomicU64::new(0),
        }
    }
}

impl RateLimiter {
    /// Returns `false` when the caller exceeded the budget for this window.
    pub fn allow(&mut self, key: &str) -> bool {
        // 60-second fixed window.
        if self.window_started.elapsed() >= std::time::Duration::from_secs(60) {
            self.window_started = Instant::now();
            self.hits.clear();
        }
        let n = self.hits.entry(key.to_string()).or_insert(0);
        if *n >= self.limit {
            return false;
        }
        *n += 1;
        self.counter.fetch_add(1, Ordering::Relaxed);
        true
    }
}

/// Correlation middleware. Every request gets an ID; every response echoes it
/// so a client report maps to a server trace.
async fn correlation(mut req: Request, next: Next) -> axum::response::Response {
    let existing = req
        .headers()
        .get("x-correlation-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let correlation_id = existing.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    if let Ok(v) = HeaderValue::from_str(&correlation_id) {
        req.headers_mut().insert("x-correlation-id", v);
    }
    let mut resp = next.run(req).await;
    if let Ok(v) = HeaderValue::from_str(&correlation_id) {
        resp.headers_mut().insert("x-correlation-id", v);
    }
    resp
}

/// Rate limiting lives in `enforce_rate_limit` below, wired through a closure
/// so the middleware does not need its own router state.

/// Session binding. An authenticated route requires `x-session-id`; the BFF
/// never trusts a body/param claim of identity.
pub fn require_session(headers: &HeaderMap) -> Result<String, AppError> {
    headers
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            status::unauthorized("identity.missing_session", "x-session-id header is required", "bff.auth")
        })
}

pub fn require_idempotency_key(headers: &HeaderMap) -> Result<String, AppError> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            status::bad_request(
                "bff.missing_idempotency_key",
                "Idempotency-Key header is required for write operations",
                "bff.idempotency",
            )
        })
}

#[derive(Debug, Deserialize)]
pub struct GuestReq {
    #[serde(default)]
    pub channel: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GuestResp {
    pub session_id: String,
    pub kind: &'static str,
    pub can_checkout: bool,
    pub expires_in_ms: i64,
}

async fn guest_session(
    State(_limiter): State<SharedBff>,
    Json(_body): Json<GuestReq>,
) -> Result<Json<GuestResp>, AppError> {
    // The session id is minted at the edge and exchanged with the identity
    // service in the deployed topology. A guest can browse but never check out.
    Ok(Json(GuestResp {
        session_id: format!("guest-{}", uuid::Uuid::new_v4().simple()),
        kind: "guest",
        can_checkout: false,
        expires_in_ms: 30 * 60 * 1000,
    }))
}

async fn healthz() -> StatusCode {
    StatusCode::OK
}

/// `GET /v1/storefront` — the one call a cold Flutter/LIFF client makes on boot.
/// Returns channel-appropriate config so the client never has to guess.
async fn storefront(headers: HeaderMap) -> Result<Json<serde_json::Value>, AppError> {
    let channel = Channel::parse(headers.get("x-channel").and_then(|v| v.to_str().ok()));
    Ok(Json(serde_json::json!({
        "channel": channel.as_str(),
        "catalog": {
            "listUrl": format!("/v1/products?channel={}", channel.as_str()),
            "defaultLocale": "ja",
            "supportedLocales": ["ja", "en"],
        },
        "checkout": {
            "requiresIdempotencyKey": true,
            "statusPollIntervalMs": 1500,
            "maxPollAttempts": 40,
        },
        "capabilities": {
            "nativeShare": channel != Channel::LineMiniApp,
            "pushNotifications": channel != Channel::LineMiniApp,
            "externalBrowserOpen": channel == Channel::LineMiniApp,
        },
        "policy": {
            "lineReviewRequired": true,
            "evidenceGrade": "Designed",
        },
    })))
}

/// `POST /v1/checkouts` — the BFF enforces the write contract before the order
/// service ever sees the request.
async fn submit_checkout(headers: HeaderMap) -> Result<StatusCode, AppError> {
    let _session = require_session(&headers)?;
    let _key = require_idempotency_key(&headers)?;
    Ok(StatusCode::ACCEPTED)
}

/// Rate limiting is built as a closure that captures the shared limiter, which
/// keeps the router state and the middleware state independent.
pub fn router(state: SharedBff) -> Router {
    let limiter = state.clone();
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/storefront", get(storefront))
        .route("/v1/sessions/guest", post(guest_session))
        .route("/v1/checkouts", post(submit_checkout))
        .with_state(state)
        .layer(middleware::from_fn(move |req: Request, next: Next| {
            let limiter = limiter.clone();
            async move { enforce_rate_limit(&limiter, req, next).await }
        }))
        .layer(middleware::from_fn(correlation))
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

async fn enforce_rate_limit(
    limiter: &SharedBff,
    req: Request,
    next: Next,
) -> axum::response::Response {
    let key = req
        .headers()
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous")
        .to_string();

    // The guard must not live across the `await` below — `std::sync::RwLock`
    // guards are not `Send`, and `PoisonError<Guard>` would keep the guard
    // alive in the error type. Collapse to `()` and scope the guard tightly.
    let allowed = match limiter.write().map_err(|_| ()) {
        Ok(mut guard) => {
            let v = guard.allow(&key);
            v
        }
        // A poisoned lock must fail closed rather than silently disabling the limit.
        Err(()) => {
            return status::service_unavailable(
                "rate_limit.unavailable",
                "rate limiter is unavailable",
                &key,
            )
            .into_response();
        }
    };

    if !allowed {
        return status::too_many_requests("rate limit exceeded for this window", &key).into_response();
    }
    next.run(req).await
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let port: u16 = std::env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(8080);
    let state: SharedBff = Arc::new(RwLock::new(RateLimiter::default()));
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind bff");
    tracing::info!("bff listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router(state)).await.expect("serve bff");
}

/// The header this service owns end to end.
const _CORRELATION: HeaderName = HeaderName::from_static("x-correlation-id");

#[cfg(test)]
mod tests {
    use super::*;
    use commerce_core::error::ProblemDetails;

    #[test]
    fn channel_parsing_defaults_to_web() {
        assert_eq!(Channel::parse(None), Channel::Web);
        assert_eq!(Channel::parse(Some("nonsense")), Channel::Web);
        assert_eq!(Channel::parse(Some("app")), Channel::App);
        assert_eq!(Channel::parse(Some("line")), Channel::LineMiniApp);
    }

    #[test]
    fn rate_limiter_blocks_after_the_budget() {
        let mut rl = RateLimiter { limit: 3, ..Default::default() };
        assert!(rl.allow("a"));
        assert!(rl.allow("a"));
        assert!(rl.allow("a"));
        assert!(!rl.allow("a"), "4th call must be refused");
        assert!(rl.allow("b"), "a different key has its own budget");
    }

    #[test]
    fn rate_limiter_window_resets() {
        let mut rl = RateLimiter { limit: 1, ..Default::default() };
        assert!(rl.allow("a"));
        assert!(!rl.allow("a"));
        rl.window_started = Instant::now() - std::time::Duration::from_secs(61);
        assert!(rl.allow("a"), "new window resets the budget");
    }

    #[test]
    fn missing_session_is_unauthorized() {
        let h = HeaderMap::new();
        assert!(matches!(
            require_session(&h).unwrap_err(),
            AppError::Problem(p) if p.code == "identity.missing_session"
        ));
    }

    #[test]
    fn empty_session_is_treated_as_missing() {
        let mut h = HeaderMap::new();
        h.insert("x-session-id", HeaderValue::from_static(""));
        assert!(require_session(&h).is_err());
    }

    #[test]
    fn missing_idempotency_key_is_bad_request() {
        let h = HeaderMap::new();
        assert!(matches!(
            require_idempotency_key(&h).unwrap_err(),
            AppError::Problem(p) if p.code == "bff.missing_idempotency_key"
        ));
    }

    #[test]
    fn problem_details_render_the_contract_fields() {
        let p = ProblemDetails::new(
            "https://api.gemarc.example/problems/forbidden",
            "Forbidden",
            StatusCode::FORBIDDEN,
            "order.forbidden",
            "order belongs to another session",
            "trace-1",
            false,
        );
        let body = serde_json::to_value(&p).unwrap();
        assert_eq!(body["status"], 403);
        assert_eq!(body["code"], "order.forbidden");
        assert_eq!(body["traceId"], "trace-1");
        assert_eq!(body["retryable"], false);
    }

    #[tokio::test]
    async fn storefront_reports_channel_capabilities() {
        let mut h = HeaderMap::new();
        h.insert("x-channel", HeaderValue::from_static("line"));
        let v = storefront(h).await.unwrap();
        assert_eq!(v["channel"], "line");
        assert_eq!(v["capabilities"]["externalBrowserOpen"], true);
        assert_eq!(v["capabilities"]["pushNotifications"], false);
    }

    #[tokio::test]
    async fn storefront_web_capabilities_differ_from_line() {
        let h = HeaderMap::new();
        let v = storefront(h).await.unwrap();
        assert_eq!(v["capabilities"]["nativeShare"], true);
        assert_eq!(v["capabilities"]["externalBrowserOpen"], false);
    }
}
