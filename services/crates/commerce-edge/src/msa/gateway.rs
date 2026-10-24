//! The gateway Function: it routes a public path to the service that owns it.
//!
//! ## What the gateway is
//!
//! The gateway is the public origin of the function topology, the way the
//! Envoy gateway is the public origin of the cluster topology. It answers the
//! paths it owns itself and forwards the rest to the owning service's
//! Function, over HTTP.
//!
//! It owns no state. The services keep their state in their own Functions,
//! exactly as they keep it in their own containers in the cluster topology.
//!
//! ## What it forwards
//!
//! The forwarded request is the client's: the method, the path, the query
//! string, the body and the headers travel unchanged, and the service's
//! answer — status code, headers and body — comes back unchanged. That is
//! what keeps the two topologies observationally identical: a client cannot
//! tell whether the catalogue answered from a container or from a Function.
//!
//! The headers that belong to one hop and not to the next (`host`,
//! `content-length`, `connection`, …) are not carried across; everything
//! else is, including `authorization` and `x-session-id`, because the
//! services decide what to do with them.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use axum::{
    extract::{Request, State},
    http::{header::HeaderName, HeaderValue, StatusCode},
    response::Response,
    routing::get,
    Router,
};
use tower::ServiceExt;
use super::routing::{route_for, Route, ServiceId};
use super::upstreams::Upstreams;

/// The largest request body the gateway forwards.
///
/// The contract's request bodies are small — a checkout is a list of lines —
/// so a megabyte is generous. The limit exists so a hostile client cannot
/// make the gateway buffer an unbounded body.
const MAX_FORWARDED_BODY_BYTES: usize = 1024 * 1024;

/// The headers that belong to one hop and must not be carried to the next.
///
/// `host` and `content-length` are set by the HTTP client from the URL and
/// the body; the rest are connection-scoped.
const HOP_BY_HOP: [&str; 10] = [
    "connection",
    "content-length",
    "host",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

/// The gateway.
#[derive(Clone)]
pub struct Gateway {
    upstreams: Upstreams,
    /// The BFF's router.
    ///
    /// In the cluster topology the BFF is a Deployment of its own; here the
    /// gateway *is* the BFF, so the paths the BFF owned are the gateway's own
    /// and are served without an HTTP hop.
    bff: Router,
    client: reqwest::Client,
}

impl Gateway {
    /// Build the gateway with an explicit set of upstreams.
    pub fn with_upstreams(upstreams: Upstreams) -> Self {
        let bff = commerce_bff::router(Arc::new(RwLock::new(
            commerce_bff::RateLimiter::default(),
        )));
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            upstreams,
            bff,
            client,
        }
    }

    /// Build the gateway from the environment.
    pub fn from_env() -> Self {
        Self::with_upstreams(Upstreams::from_env())
    }

    /// The gateway's routing decision for `path`.
    pub fn route_for(&self, path: &str) -> Route {
        route_for(path)
    }

    /// The base URL of the Function that serves `path`.
    ///
    /// `None` for a path the gateway answers itself, and for a path no
    /// service owns.
    pub fn base_url_for(&self, path: &str) -> Option<&str> {
        match route_for(path) {
            Route::Forward(id) => self.upstreams.base_url_for(id),
            _ => None,
        }
    }

    /// Forward a request to the Function that owns its path.
    pub async fn forward(&self, id: ServiceId, request: Request) -> Response {
        let path = request.uri().path().to_string();
        let Some(base) = self.upstreams.base_url_for(id) else {
            return not_found(&path);
        };

        let method = request.method().clone();
        let query = request.uri().query().map(|q| q.to_string());
        let target = match &query {
            Some(query) => format!("{path}?{query}"),
            None => path.clone(),
        };
        let url = format!("{base}{target}");

        let headers = request.headers().clone();
        let body = match axum::body::to_bytes(request.into_body(), MAX_FORWARDED_BODY_BYTES).await {
            Ok(body) => body,
            Err(error) => {
                tracing::error!(error = %error, path = %path, "could not read the request body");
                return super::problem(
                    StatusCode::BAD_REQUEST,
                    "gateway.unreadable_body",
                    "the request body could not be read",
                );
            }
        };

        let mut outgoing = self.client.request(method, &url);
        for (name, value) in headers.iter() {
            if is_hop_by_hop(name) {
                continue;
            }
            outgoing = outgoing.header(name, value);
        }

        let response = match outgoing.body(body).send().await {
            Ok(response) => response,
            Err(error) => {
                tracing::error!(error = %error, url = %url, "the service function is unreachable");
                return super::problem(
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
            Err(error) => {
                tracing::error!(error = %error, url = %url, "could not read the upstream response");
                return super::problem(
                    StatusCode::BAD_GATEWAY,
                    "gateway.upstream_unreadable",
                    "the upstream service returned an unreadable response",
                );
            }
        };

        let mut out = Response::builder().status(status);
        for (name, value) in upstream_headers.iter() {
            if is_hop_by_hop(name) {
                continue;
            }
            if let Ok(value) = HeaderValue::from_bytes(value.as_bytes()) {
                out = out.header(name, value);
            }
        }
        out.body(axum::body::Body::from(bytes)).unwrap_or_else(|_| {
            super::problem(
                StatusCode::BAD_GATEWAY,
                "gateway.upstream_invalid",
                "the upstream service returned an invalid response",
            )
        })
    }

    /// Answer a request the gateway owns, without an HTTP hop.
    async fn serve_local(&self, request: Request) -> Response {
        match self.bff.clone().oneshot(request).await {
            Ok(response) => response,
            Err(error) => {
                // The gateway's own router failed to produce a response. This
                // is a bug in the gateway, not a client error, and it must not
                // be reported as a 404 — that would send the client looking
                // for a route that exists.
                tracing::error!(%error, "the gateway's own router failed");
                super::problem(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "gateway.dispatch_failed",
                    "the gateway could not answer this path",
                )
            }
        }
    }
}

/// The gateway's router.
pub fn router(gateway: Gateway) -> Router {
    Router::new()
        // The health check answers for the whole deployment.
        .route("/healthz", get(health))
        // Everything else is dispatched to the service that owns the path.
        .fallback(dispatch)
        .with_state(gateway)
}

/// The gateway's own health check.
///
/// It answers 200 when the gateway is up. It does **not** try to reach every
/// service: they are separate Functions, and a failing dependency must not
/// make the gateway look dead.
async fn health() -> StatusCode {
    StatusCode::OK
}

/// Route a request to the service that owns its path.
async fn dispatch(State(gateway): State<Gateway>, request: Request) -> Response {
    let path = request.uri().path().to_string();
    match gateway.route_for(&path) {
        Route::Local => gateway.serve_local(request).await,
        Route::Forward(id) => gateway.forward(id, request).await,
        Route::Unknown => not_found(&path),
    }
}

/// Whether a header belongs to one hop only.
fn is_hop_by_hop(name: &HeaderName) -> bool {
    HOP_BY_HOP.contains(&name.as_str())
}

/// A path that belongs to no service.
///
/// The same shape as every other failure in the codebase: an RFC 9457
/// ProblemDetails, so a client parses one error format regardless of which
/// layer failed.
fn not_found(path: &str) -> Response {
    super::problem(
        StatusCode::NOT_FOUND,
        "gateway.no_such_route",
        &format!("no service handles {path}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, extract::Request, response::IntoResponse};
    use tokio::net::TcpListener;

    /// A stand-in for a service's Function: it echoes what it received.
    ///
    /// The point is not the echo — it is that the gateway really did send the
    /// client's request, over HTTP, to the service that owns the path.
    async fn spawn_echo() -> String {
        let app = Router::new().fallback(|request: Request| async move {
            let method = request.method().to_string();
            let path = request.uri().path().to_string();
            let query = request.uri().query().unwrap_or_default().to_string();
            let session = request
                .headers()
                .get("x-session-id")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string();
            let body = axum::body::to_bytes(request.into_body(), usize::MAX)
                .await
                .unwrap_or_default();
            (
                StatusCode::CREATED,
                axum::Json(serde_json::json!({
                    "method": method,
                    "path": path,
                    "query": query,
                    "session": session,
                    "body": String::from_utf8_lossy(&body),
                })),
            )
                .into_response()
        });
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        format!("http://{address}")
    }

    async fn body_of(response: Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the body is readable");
        serde_json::from_slice(&bytes).expect("the body is json")
    }

    #[tokio::test]
    async fn a_request_reaches_the_service_that_owns_its_path() {
        let base = spawn_echo().await;
        let app = router(Gateway::with_upstreams(Upstreams::all_to(&base)));

        let request = Request::builder()
            .method("POST")
            .uri("/v1/checkouts?channel=web")
            .header("x-session-id", "sess-000001")
            .header("content-type", "application/json")
            .body(Body::from("{\"lines\":[]}"))
            .expect("the request builds");

        let response = app.oneshot(request).await.expect("the gateway answers");

        // The service's status code is the client's status code: a 409 for an
        // idempotency conflict must not be flattened into a 502.
        assert_eq!(response.status(), StatusCode::CREATED);

        let echoed = body_of(response).await;
        assert_eq!(echoed["method"], "POST");
        assert_eq!(echoed["path"], "/v1/checkouts");
        assert_eq!(echoed["query"], "channel=web");
        assert_eq!(echoed["session"], "sess-000001");
        assert_eq!(echoed["body"], "{\"lines\":[]}");
    }

    #[tokio::test]
    async fn a_path_no_service_owns_is_a_404_and_is_not_forwarded() {
        let app = router(Gateway::with_upstreams(Upstreams::all_to(
            "http://127.0.0.1:1",
        )));

        let request = Request::builder()
            .uri("/v1/nope")
            .body(Body::empty())
            .expect("the request builds");
        let response = app.oneshot(request).await.expect("the gateway answers");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = body_of(response).await;
        assert_eq!(body["code"], "gateway.no_such_route");
    }

    #[tokio::test]
    async fn an_unreachable_service_is_a_503_and_not_a_panic() {
        // Port 1 refuses the connection, so this is the case where the
        // service's Function is not there.
        let app = router(Gateway::with_upstreams(Upstreams::all_to(
            "http://127.0.0.1:1",
        )));

        let request = Request::builder()
            .uri("/v1/products")
            .body(Body::empty())
            .expect("the request builds");
        let response = app.oneshot(request).await.expect("the gateway answers");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = body_of(response).await;
        assert_eq!(body["code"], "gateway.upstream_unavailable");
    }
}
