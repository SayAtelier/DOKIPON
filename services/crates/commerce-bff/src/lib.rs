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
    body::Bytes,
    extract::{Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use commerce_core::error::{status, AppError};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
    time::Instant,
};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

mod upstream;

pub use upstream::{build_client, Upstreams};

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
            status::unauthorized(
                "identity.missing_session",
                "x-session-id header is required",
                "bff.auth",
            )
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

/// `POST /v1/checkouts` — the BFF enforces the write contract, then forwards
/// the checkout to the order service.
///
/// The edge rules come first: a session and an idempotency key must be
/// present. The request body is then handed to the order service unchanged,
/// and the upstream's answer is passed back verbatim — including its status
/// code, because 409 (idempotency conflict), 422 (unpriceable line) and 503
/// (pricing unavailable) are three different things the client must be able
/// to tell apart.
async fn submit_checkout(
    State((_limiter, upstreams, client)): State<(SharedBff, Arc<Upstreams>, reqwest::Client)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AppError> {
    let _session = require_session(&headers)?;
    let _key = require_idempotency_key(&headers)?;
    Ok(upstreams
        .forward(
            &client,
            axum::http::Method::POST,
            "/v1/checkouts",
            &headers,
            body,
        )
        .await)
}

/// Forward a request the BFF does not own to the service that does.
///
/// This is what makes the BFF a real gateway in the containerized topology.
/// The k3s deployment runs one container per service and the BFF is the public
/// origin, so every contract path the BFF does not implement itself has to
/// travel upstream. Before this, six of the contract's twelve paths answered
/// `404` at the only origin the client was given — including
/// `/v1/sessions/line`, which made the LINE login path unreachable.
///
/// The forwarded path, query string, method and body are the client's, and
/// the upstream's status code and body come back unchanged. The only headers
/// that travel are the ones the contract defines (`x-session-id`,
/// `idempotency-key`, `x-channel`) plus the correlation id; a client-supplied
/// header that could change an outcome (`x-simulated-payment`) stays here.
async fn forward_unowned(
    State((_limiter, upstreams, client)): State<(SharedBff, Arc<Upstreams>, reqwest::Client)>,
    request: Request,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();
    let target = match uri.query() {
        Some(query) => format!("{path}?{query}"),
        None => path.clone(),
    };
    let headers = request.headers().clone();

    let body = match axum::body::to_bytes(request.into_body(), MAX_FORWARDED_BODY_BYTES).await {
        Ok(body) => body,
        Err(e) => {
            tracing::error!(error = %e, path = %path, "could not read the request body");
            return (
                StatusCode::BAD_REQUEST,
                [("content-type", "application/problem+json")],
                serde_json::json!({
                    "type": "https://api.gemarc.example/problems/bad-request",
                    "title": "Bad Request",
                    "status": 400,
                    "code": "gateway.unreadable_body",
                    "detail": "the request body could not be read",
                    "traceId": "",
                    "retryable": false,
                })
                .to_string(),
            )
                .into_response();
        }
    };

    upstreams
        .forward(&client, method, &target, &headers, body)
        .await
}

/// The largest request body the BFF forwards upstream.
///
/// The contract's request bodies are small — a checkout is a list of lines —
/// so a megabyte is generous. The limit exists so a hostile client cannot
/// make the edge buffer an unbounded body.
const MAX_FORWARDED_BODY_BYTES: usize = 1024 * 1024;

/// Rate limiting is built as a closure that captures the shared limiter, which
/// keeps the router state and the middleware state independent.
pub fn router(state: SharedBff) -> Router {
    let limiter = state.clone();
    let upstreams = Arc::new(Upstreams::from_env());
    let client = build_client();
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/storefront", get(storefront))
        .route("/v1/content", get(content))
        .route("/v1/checkouts", post(submit_checkout))
        // Everything the BFF does not implement itself belongs to another
        // service, and the client is given one origin. The fallback forwards
        // it; it does not answer it.
        //
        // `/v1/sessions/guest` used to be a route of its own here, minting a
        // local id. The identity service owns guest sessions, and it owns the
        // LINE exchange and its nonce replay guard; a second, different
        // implementation of the same path meant the behaviour depended on
        // which origin the client was pointed at. Now the path is forwarded,
        // so the identity service is the only implementation of it.
        .fallback(forward_unowned)
        .with_state((state, upstreams, client))
        .layer(middleware::from_fn(move |req: Request, next: Next| {
            let limiter = limiter.clone();
            async move { enforce_rate_limit(&limiter, req, next).await }
        }))
        .layer(middleware::from_fn(correlation))
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

/// `GET /v1/content` — the editable content document.
///
/// The collections, series, trends, topics, pages, FAQ, banners and size
/// guides that make the storefront a shop rather than a grid. The operator
/// edits them in the admin console; the storefront reads them here.
async fn content(
    State((_limiter, upstreams, client)): State<(SharedBff, Arc<Upstreams>, reqwest::Client)>,
) -> Response {
    upstreams.fetch_content(&client).await
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
        return status::too_many_requests("rate limit exceeded for this window", &key)
            .into_response();
    }
    next.run(req).await
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
        let mut rl = RateLimiter {
            limit: 3,
            ..Default::default()
        };
        assert!(rl.allow("a"));
        assert!(rl.allow("a"));
        assert!(rl.allow("a"));
        assert!(!rl.allow("a"), "4th call must be refused");
        assert!(rl.allow("b"), "a different key has its own budget");
    }

    #[test]
    fn rate_limiter_window_resets() {
        let mut rl = RateLimiter {
            limit: 1,
            ..Default::default()
        };
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
