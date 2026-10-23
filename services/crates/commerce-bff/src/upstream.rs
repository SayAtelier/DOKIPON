//! Upstream forwarding for the BFF.
//!
//! ## The hole this fills
//!
//! `POST /v1/checkouts` used to validate the session header and the
//! idempotency key, then answer `202 Accepted` — without ever reading the
//! request body and without contacting the order service. A shopper pressing
//! "buy" got a success response and no order. The gateway was the *only*
//! public entry point, so nothing else could have created the order either.
//!
//! The BFF is the edge, so forwarding is its job: enforce the edge rules
//! (session present, idempotency key present, rate limit), then hand the
//! request to the order service and pass the answer back.
//!
//! ## What the BFF does NOT do
//!
//! It does not price anything and it does not decide the order's state. The
//! order service owns both, and the order service re-prices every line from
//! the catalogue. If the BFF tried to be clever here it would become a
//! second source of truth about money, which is exactly the class of bug this
//! codebase has already been bitten by twice.

use std::time::Duration;

use axum::{
    body::Bytes,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};

/// Where the order service lives.
#[derive(Debug, Clone)]
pub struct Upstreams {
    pub order_base_url: String,

    /// Where the content document lives.
    pub content_base_url: String,
}

/// The request headers the BFF forwards to the order service.
///
/// The session id and the idempotency key are the two that matter: the order
/// service scopes orders by session and dedupes by key, and both are
/// carried in headers rather than in the body. Everything else is dropped,
/// because forwarding a client-supplied header like `x-simulated-payment`
/// would let a browser choose the payment outcome.
const FORWARDED_REQUEST_HEADERS: [&str; 4] = [
    "x-session-id",
    "idempotency-key",
    "x-correlation-id",
    "content-type",
];

/// The response headers the BFF passes back to the client.
const FORWARDED_RESPONSE_HEADERS: [&str; 3] = [
    "content-type",
    "x-correlation-id",
    "retry-after",
];

impl Upstreams {
    pub fn from_env() -> Self {
        Self {
            order_base_url: std::env::var("ORDER_URL")
                .unwrap_or_else(|_| "http://commerce-order:8085".to_string())
                .trim_end_matches('/')
                .to_string(),
            content_base_url: std::env::var("CONTENT_URL")
                .unwrap_or_else(|_| "http://commerce-content:8096".to_string())
                .trim_end_matches('/')
                .to_string(),
        }
    }

    /// Fetch the editable content document.
    ///
    /// The storefront renders its collections, series, trends, topics, pages,
    /// FAQ, banners and size guides from this one document. Fetching it
    /// through the edge keeps the browser on a single origin, so the page
    /// does not need a CORS preflight to render its navigation.
    ///
    /// A content service that is down must not take the catalogue down with
    /// it: an empty document is returned and the storefront falls back to its
    /// own defaults.
    pub async fn fetch_content(&self, client: &reqwest::Client) -> Response {
        let url = format!("{}/v1/content", self.content_base_url);
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => match response.bytes().await {
                Ok(bytes) => (
                    StatusCode::OK,
                    [("content-type", "application/json")],
                    bytes.to_vec(),
                )
                    .into_response(),
                Err(e) => {
                    tracing::error!(error = %e, "content response was unreadable");
                    empty_content()
                }
            },
            Ok(response) => {
                tracing::warn!(status = %response.status(), "content service returned an error");
                empty_content()
            }
            Err(e) => {
                tracing::error!(error = %e, "content service unreachable");
                empty_content()
            }
        }
    }

    /// Forward a checkout to the order service and return its answer.
    ///
    /// The upstream's status code is preserved, so the client sees the real
    /// outcome: 409 for an idempotency conflict, 422 for a line that cannot
    /// be priced, 503 when pricing is unavailable. Collapsing those into a
    /// blanket 502 would hide exactly the information the client needs.
    pub async fn forward_checkout(
        &self,
        client: &reqwest::Client,
        headers: &HeaderMap,
        body: Bytes,
    ) -> Response {
        let url = format!("{}/v1/checkouts", self.order_base_url);

        let mut request = client.post(&url);
        for name in FORWARDED_REQUEST_HEADERS {
            if let Some(value) = headers.get(name) {
                if let Ok(value) = value.to_str() {
                    request = request.header(name, value);
                }
            }
        }
        let request = request.body(body);

        let response = match request.send().await {
            Ok(response) => response,
            Err(e) => {
                tracing::error!(error = %e, "order service unreachable from the BFF");
                return problem(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "checkout.unavailable",
                    "the order service is temporarily unavailable",
                );
            }
        };

        let status = response.status();
        let upstream_headers = response.headers().clone();
        let bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!(error = %e, "could not read the order service response");
                return problem(
                    StatusCode::BAD_GATEWAY,
                    "checkout.upstream_unreadable",
                    "the order service returned an unreadable response",
                );
            }
        };

        let mut out = Response::builder().status(status);
        for name in FORWARDED_RESPONSE_HEADERS {
            if let (Some(value), Some(builder)) = (upstream_headers.get(name), out.headers_mut()) {
                if let Ok(value) = HeaderValue::from_bytes(value.as_bytes()) {
                    builder.insert(HeaderName::from_bytes(name.as_bytes()).unwrap(), value);
                }
            }
        }
        out.body(axum::body::Body::from(bytes))
            .unwrap_or_else(|_| problem(StatusCode::BAD_GATEWAY, "checkout.upstream_invalid", "bad upstream response"))
    }
}

/// A minimal ProblemDetails for the errors the BFF generates itself.
///
/// The shape matches RFC 9457 and the rest of the codebase, so the client
/// parses one error format regardless of which layer failed.
fn problem(status: StatusCode, code: &str, detail: &str) -> Response {
    let body = serde_json::json!({
        "type": format!("https://api.gemarc.example/problems/{}", code.replace('.', "-")),
        "title": status.canonical_reason().unwrap_or("Error"),
        "status": status.as_u16(),
        "code": code,
        "detail": detail,
        "traceId": "",
        "retryable": status == StatusCode::SERVICE_UNAVAILABLE,
    });
    (
        status,
        [("content-type", "application/problem+json")],
        body.to_string(),
    )
        .into_response()
}

/// An empty content document, used when the content service cannot be
/// reached. The storefront falls back to its own defaults, so the shop keeps
/// selling even when the operator's content is unavailable.
fn empty_content() -> Response {
    (
        StatusCode::OK,
        [("content-type", "application/json")],
        r#"{"collections":[],"series":[],"trends":[],"topics":[],"pages":[],"faqs":[],"banners":[],"size_guides":[],"revision":0}"#,
    )
        .into_response()
}

/// Build the HTTP client the BFF uses to reach its upstreams.
pub fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_safe_headers_are_forwarded() {
        // A client must not be able to choose the payment outcome by sending
        // `x-simulated-payment`, and must not smuggle a different session.
        assert!(FORWARDED_REQUEST_HEADERS.contains(&"x-session-id"));
        assert!(FORWARDED_REQUEST_HEADERS.contains(&"idempotency-key"));
        assert!(!FORWARDED_REQUEST_HEADERS.contains(&"x-simulated-payment"));
        assert!(!FORWARDED_REQUEST_HEADERS.contains(&"authorization"));
        assert!(!FORWARDED_REQUEST_HEADERS.contains(&"host"));
    }

    #[tokio::test]
    async fn an_unreachable_order_service_is_a_503_not_a_panic() {
        let upstreams = Upstreams {
            order_base_url: "http://127.0.0.1:1".to_string(),
            content_base_url: "http://127.0.0.1:1".to_string(),
        };
        let client = build_client();
        let response = upstreams
            .forward_checkout(&client, &HeaderMap::new(), Bytes::new())
            .await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
