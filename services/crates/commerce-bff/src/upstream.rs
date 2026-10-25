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
//! request to the service that owns the path and pass the answer back.
//!
//! ## Why the BFF forwards at all
//!
//! There are two deployments, and both need one origin:
//!
//! - **one process**: `commerce-edge` mounts every service router and
//!   dispatches in-process. Nothing crosses a network.
//! - **one container per service**: the k3s deployment. The BFF is the public
//!   origin and the other services are private, so the BFF has to reach them
//!   over HTTP.
//!
//! In the second deployment the BFF used to answer `404` for every path it
//! did not implement — `/v1/carts/{id}`, `/v1/orders`, `/v1/sessions/line`,
//! `/v1/checkouts/{id}`. Six of the contract's twelve paths were unreachable
//! from the only origin the client was given.
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
use commerce_core::routes::{owner_of, ServiceId};

/// Where the commerce services live.
///
/// The k3s ConfigMap already wires these names
/// (`services/deploy/k3s/00-namespace-configmap.yaml`), which is why the
/// variable names here match it exactly.
#[derive(Debug, Clone)]
pub struct Upstreams {
    pub catalog_base_url: String,
    pub cart_base_url: String,
    pub order_base_url: String,
    pub identity_base_url: String,
    pub inventory_base_url: String,
    pub payment_base_url: String,
    pub content_base_url: String,
}

/// The request headers the BFF forwards to an upstream service.
///
/// The session id and the idempotency key are the two that matter: the order
/// service scopes orders by session and dedupes by key, and both are
/// carried in headers rather than in the body. Everything else is dropped,
/// because forwarding a client-supplied header like `x-simulated-payment`
/// would let a browser choose the payment outcome.
const FORWARDED_REQUEST_HEADERS: [&str; 5] = [
    "x-session-id",
    "idempotency-key",
    "x-correlation-id",
    "content-type",
    // The contract's `ChannelHeader`. The catalogue filters its content per
    // channel, so dropping it would hand a LINE client web-only content.
    "x-channel",
];

/// The response headers the BFF passes back to the client.
const FORWARDED_RESPONSE_HEADERS: [&str; 3] = ["content-type", "x-correlation-id", "retry-after"];

impl Upstreams {
    pub fn from_env() -> Self {
        Self {
            catalog_base_url: base_url("CATALOG_URL", "http://commerce-catalog:8082"),
            cart_base_url: base_url("CART_URL", "http://commerce-cart:8083"),
            order_base_url: base_url("ORDER_URL", "http://commerce-order:8085"),
            identity_base_url: base_url("IDENTITY_URL", "http://commerce-identity:8081"),
            inventory_base_url: base_url("INVENTORY_URL", "http://commerce-inventory:8084"),
            payment_base_url: base_url(
                "PAYMENT_ADAPTER_URL",
                "http://commerce-payment-adapter:8086",
            ),
            content_base_url: base_url("CONTENT_URL", "http://commerce-content:8096"),
        }
    }

    /// The base URL of the service that owns `path`, when the BFF forwards it.
    ///
    /// `None` means "the BFF answers this itself, or nothing does": the
    /// storefront config and the content document are the BFF's own routes,
    /// and the admin facade is a separate public entry point with its own
    /// authentication.
    pub fn base_url_for(&self, path: &str) -> Option<&str> {
        match owner_of(path)? {
            ServiceId::Catalog => Some(&self.catalog_base_url),
            ServiceId::Cart => Some(&self.cart_base_url),
            ServiceId::Order => Some(&self.order_base_url),
            ServiceId::Identity => Some(&self.identity_base_url),
            ServiceId::Inventory => Some(&self.inventory_base_url),
            ServiceId::Payment => Some(&self.payment_base_url),
            ServiceId::Bff | ServiceId::Content | ServiceId::Admin => None,
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

    /// Forward a request to the service that owns its path, and return that
    /// service's answer unchanged.
    ///
    /// The upstream's status code is preserved, so the client sees the real
    /// outcome: 409 for an idempotency conflict, 422 for a line that cannot
    /// be priced, 503 when pricing is unavailable. Collapsing those into a
    /// blanket 502 would hide exactly the information the client needs.
    ///
    /// `target` is the path, optionally with its query string — the same
    /// path the client asked for, handed to the service unchanged.
    pub async fn forward(
        &self,
        client: &reqwest::Client,
        method: http::Method,
        target: &str,
        headers: &HeaderMap,
        body: Bytes,
    ) -> Response {
        let path = target.split('?').next().unwrap_or(target);
        let Some(base) = self.base_url_for(path) else {
            return problem(
                StatusCode::NOT_FOUND,
                "gateway.no_such_route",
                &format!("no service handles {path}"),
            );
        };
        let url = format!("{base}{target}");

        let mut request = client.request(method, &url);
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
                tracing::error!(error = %e, url = %url, "upstream service unreachable from the BFF");
                return problem(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "gateway.upstream_unavailable",
                    "the service handling this path is temporarily unavailable",
                );
            }
        };

        let status = response.status();
        let upstream_headers = response.headers().clone();
        let bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!(error = %e, "could not read the upstream response");
                return problem(
                    StatusCode::BAD_GATEWAY,
                    "gateway.upstream_unreadable",
                    "the upstream service returned an unreadable response",
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
        out.body(axum::body::Body::from(bytes)).unwrap_or_else(|_| {
            problem(
                StatusCode::BAD_GATEWAY,
                "gateway.upstream_invalid",
                "bad upstream response",
            )
        })
    }
}

/// The base URL of an upstream, from the environment.
///
/// The trailing slash is stripped so a URL can be built by concatenation
/// without producing a double slash.
fn base_url(variable: &str, fallback: &str) -> String {
    std::env::var(variable)
        .unwrap_or_else(|_| fallback.to_string())
        .trim_end_matches('/')
        .to_string()
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

    fn upstreams() -> Upstreams {
        Upstreams {
            catalog_base_url: "http://catalog".into(),
            cart_base_url: "http://cart".into(),
            order_base_url: "http://order".into(),
            identity_base_url: "http://identity".into(),
            inventory_base_url: "http://inventory".into(),
            payment_base_url: "http://payment".into(),
            content_base_url: "http://content".into(),
        }
    }

    #[test]
    fn only_the_safe_headers_are_forwarded() {
        // A client must not be able to choose the payment outcome by sending
        // `x-simulated-payment`, and must not smuggle a different session.
        assert!(FORWARDED_REQUEST_HEADERS.contains(&"x-session-id"));
        assert!(FORWARDED_REQUEST_HEADERS.contains(&"idempotency-key"));
        assert!(FORWARDED_REQUEST_HEADERS.contains(&"x-channel"));
        assert!(!FORWARDED_REQUEST_HEADERS.contains(&"x-simulated-payment"));
        assert!(!FORWARDED_REQUEST_HEADERS.contains(&"authorization"));
        assert!(!FORWARDED_REQUEST_HEADERS.contains(&"host"));
    }

    #[test]
    fn each_contract_path_resolves_to_its_owning_service() {
        let u = upstreams();
        assert_eq!(u.base_url_for("/v1/products"), Some("http://catalog"));
        assert_eq!(u.base_url_for("/v1/products/p-1"), Some("http://catalog"));
        assert_eq!(u.base_url_for("/v1/carts/c-1"), Some("http://cart"));
        assert_eq!(
            u.base_url_for("/v1/carts/c-1/items/sku-1"),
            Some("http://cart")
        );
        assert_eq!(u.base_url_for("/v1/checkouts"), Some("http://order"));
        assert_eq!(u.base_url_for("/v1/checkouts/chk-1"), Some("http://order"));
        assert_eq!(u.base_url_for("/v1/orders"), Some("http://order"));
        assert_eq!(
            u.base_url_for("/v1/orders/ord-1/reconcile"),
            Some("http://order")
        );
        assert_eq!(u.base_url_for("/v1/sessions/line"), Some("http://identity"));
        assert_eq!(
            u.base_url_for("/v1/sessions/guest"),
            Some("http://identity")
        );
    }

    #[test]
    fn the_bffs_own_paths_are_not_forwarded_to_anything() {
        let u = upstreams();
        assert_eq!(u.base_url_for("/v1/storefront"), None);
        assert_eq!(u.base_url_for("/v1/content"), None);
        assert_eq!(u.base_url_for("/admin/v1/orders"), None);
        assert_eq!(u.base_url_for("/v1/unknown"), None);
    }

    #[tokio::test]
    async fn an_unreachable_order_service_is_a_503_not_a_panic() {
        let u = Upstreams {
            order_base_url: "http://127.0.0.1:1".into(),
            ..upstreams()
        };
        let client = build_client();
        let response = u
            .forward(
                &client,
                http::Method::POST,
                "/v1/checkouts",
                &HeaderMap::new(),
                Bytes::new(),
            )
            .await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn a_path_with_no_owner_is_a_404() {
        let u = upstreams();
        let client = build_client();
        let response = u
            .forward(
                &client,
                reqwest::Method::GET,
                "/v1/nope",
                &HeaderMap::new(),
                Bytes::new(),
            )
            .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
