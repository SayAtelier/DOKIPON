//! The function topology: one Function per commerce service, a gateway in front.
//!
//! ## What this is
//!
//! The other deployment of the commerce stack is the **cluster**: one
//! Deployment per service behind an Envoy gateway (see
//! `services/deploy/k3s/`). This module is the same topology expressed in
//! Vercel Functions — one Function per service, one gateway Function in front
//! of them — and it runs the same service code against the same contract.
//!
//! ## Why the machinery is here and not in the deployment directory
//!
//! The Functions themselves live in `services/deploy/vercel/src/`, because that
//! is where the deployment's wiring belongs. The machinery they are built from
//! — who owns which public path, where a service's Function lives, how the
//! price authority reaches the catalogue — lives here, in the workspace, for
//! one reason: **it has to be testable by the repository's own test
//! commands**. The files under `services/deploy/vercel/src/` are compiled only
//! by the root package, which has no test suite; a routing table that only the
//! deployment can exercise is a routing table that drifts silently.
//!
//! ## The two topologies must agree
//!
//! The in-process edge ([`crate::build`]) dispatches a request to the service
//! that owns its path, in memory. The gateway dispatches the same request to
//! the same service, over HTTP. The table that decides is the same table —
//! [`commerce_core::routes`], the contract of record — so a path that is
//! served in one topology is served in the other.
//!
//! ## The path prefixes
//!
//! Each service's Function is mounted under its own path, and the service's
//! own routes are rooted at `/`:
//!
//! | Service | Function | Owns |
//! |---|---|---|
//! | gateway | `/api/gateway` | every public path |
//! | BFF | *(the gateway itself)* | `/v1/storefront` |
//! | catalogue | `/api/svc/catalog` | `/v1/products` |
//! | cart | `/api/svc/cart` | `/v1/carts` |
//! | order | `/api/svc/order` | `/v1/checkouts`, `/v1/orders` |
//! | content | `/api/svc/content` | `/v1/content` |
//! | identity | `/api/svc/identity` | `/v1/sessions` |
//! | inventory | `/api/svc/inventory` | `/v1/inventory` |
//! | payment | `/api/svc/payment` | `/v1/payments` |
//! | admin | `/api/svc/admin` | `/admin/v1` |
//!
//! The gateway never calls itself: every service it forwards to is under
//! `/api/svc/`, and the gateway's own route is `/api/gateway`.

pub mod gateway;
pub mod routing;
pub mod service;
pub mod upstreams;

pub use routing::{owner_of, route_for, service_prefix, Route, ServiceId};
pub use upstreams::Upstreams;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use commerce_core::error::ProblemDetails;

/// A ProblemDetails the function topology generates itself.
///
/// The same shape as every other failure in the codebase: an RFC 9457
/// ProblemDetails, so a client parses one error format regardless of which
/// layer failed.
pub(crate) fn problem(status: StatusCode, code: &str, detail: &str) -> Response {
    ProblemDetails::new(
        format!(
            "https://api.gemarc.example/problems/{}",
            code.replace('.', "-")
        ),
        status.canonical_reason().unwrap_or("Error"),
        status,
        code,
        detail,
        "",
        status == StatusCode::SERVICE_UNAVAILABLE,
    )
    .into_response()
}
