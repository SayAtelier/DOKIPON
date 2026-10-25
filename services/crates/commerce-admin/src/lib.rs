/*
@cypher schema=1 source_sha256=314d4d6b7db8990dc1d2e14af1c1fba0e712da98fd2ef3d2c77fb3fcd7e1ead3
MERGE (self:File {path:"services/crates/commerce-admin/src/main.rs"})
MERGE (role:Symbol {id:"services/crates/commerce-admin/src/main.rs::Role",kind:"enum"})
MERGE (session:Symbol {id:"services/crates/commerce-admin/src/main.rs::Session",kind:"struct"})
MERGE (credential:Symbol {id:"services/crates/commerce-admin/src/main.rs::Credential",kind:"struct"})
MERGE (auth_store:Symbol {id:"services/crates/commerce-admin/src/main.rs::AuthStore",kind:"struct"})
MERGE (state:Symbol {id:"services/crates/commerce-admin/src/main.rs::AdminState",kind:"struct"})
MERGE (upstreams:Symbol {id:"services/crates/commerce-admin/src/main.rs::Upstreams",kind:"struct"})
MERGE (authz_state:Symbol {id:"services/crates/commerce-admin/src/main.rs::AuthorizationState",kind:"struct"})
MERGE (correlation_id:Symbol {id:"services/crates/commerce-admin/src/main.rs::CorrelationId",kind:"struct"})
MERGE (login_request:Symbol {id:"services/crates/commerce-admin/src/main.rs::LoginRequest",kind:"struct"})
MERGE (session_response:Symbol {id:"services/crates/commerce-admin/src/main.rs::SessionResponse",kind:"struct"})
MERGE (session_info:Symbol {id:"services/crates/commerce-admin/src/main.rs::SessionInfo",kind:"struct"})
MERGE (orders_query:Symbol {id:"services/crates/commerce-admin/src/main.rs::OrdersQuery",kind:"struct"})
MERGE (products_query:Symbol {id:"services/crates/commerce-admin/src/main.rs::ProductsQuery",kind:"struct"})
MERGE (payments_query:Symbol {id:"services/crates/commerce-admin/src/main.rs::PaymentsQuery",kind:"struct"})
MERGE (reconcile_request:Symbol {id:"services/crates/commerce-admin/src/main.rs::ReconcileRequest",kind:"struct"})
MERGE (reservation_request:Symbol {id:"services/crates/commerce-admin/src/main.rs::ReservationRequest",kind:"struct"})
MERGE (mock_state:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.MockState",kind:"struct"})
MERGE (observed_request:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.ObservedRequest",kind:"struct"})
MERGE (mock_upstream_type:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.MockUpstream",kind:"struct"})
MERGE (session_response_test:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.SessionResponseForTest",kind:"struct"})
MERGE (role_permits:Symbol {id:"services/crates/commerce-admin/src/main.rs::Role.permits",kind:"method"})
MERGE (role_rank:Symbol {id:"services/crates/commerce-admin/src/main.rs::Role.rank",kind:"method"})
MERGE (auth_store_new:Symbol {id:"services/crates/commerce-admin/src/main.rs::AuthStore.new",kind:"method"})
MERGE (upstreams_from_env:Symbol {id:"services/crates/commerce-admin/src/main.rs::Upstreams.from_env",kind:"method"})
MERGE (upstreams_all_to:Symbol {id:"services/crates/commerce-admin/src/main.rs::Upstreams.all_to",kind:"method"})
MERGE (env_or:Symbol {id:"services/crates/commerce-admin/src/main.rs::env_or",kind:"function"})
MERGE (mock_drop:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.MockUpstream.drop",kind:"method"})
MERGE (app_router:Symbol {id:"services/crates/commerce-admin/src/main.rs::router",kind:"function"})
MERGE (from_env:Symbol {id:"services/crates/commerce-admin/src/main.rs::AdminState.from_env",kind:"method"})
MERGE (optional_credential:Symbol {id:"services/crates/commerce-admin/src/main.rs::optional_credential",kind:"function"})
MERGE (create_session:Symbol {id:"services/crates/commerce-admin/src/main.rs::create_session",kind:"function"})
MERGE (secret_matches:Symbol {id:"services/crates/commerce-admin/src/main.rs::secret_matches",kind:"function"})
MERGE (current_session:Symbol {id:"services/crates/commerce-admin/src/main.rs::current_session",kind:"function"})
MERGE (list_orders:Symbol {id:"services/crates/commerce-admin/src/main.rs::list_orders",kind:"function"})
MERGE (order_detail:Symbol {id:"services/crates/commerce-admin/src/main.rs::order_detail",kind:"function"})
MERGE (reconcile_order:Symbol {id:"services/crates/commerce-admin/src/main.rs::reconcile_order",kind:"function"})
MERGE (list_products:Symbol {id:"services/crates/commerce-admin/src/main.rs::list_products",kind:"function"})
MERGE (product_detail:Symbol {id:"services/crates/commerce-admin/src/main.rs::product_detail",kind:"function"})
MERGE (inventory_levels:Symbol {id:"services/crates/commerce-admin/src/main.rs::inventory_levels",kind:"function"})
MERGE (list_payments:Symbol {id:"services/crates/commerce-admin/src/main.rs::list_payments",kind:"function"})
MERGE (reserve_inventory:Symbol {id:"services/crates/commerce-admin/src/main.rs::reserve_inventory",kind:"function"})
MERGE (release_reservation:Symbol {id:"services/crates/commerce-admin/src/main.rs::release_reservation",kind:"function"})
MERGE (encode_query:Symbol {id:"services/crates/commerce-admin/src/main.rs::encode_query",kind:"function"})
MERGE (encode_segment:Symbol {id:"services/crates/commerce-admin/src/main.rs::encode_segment",kind:"function"})
MERGE (request_upstream:Symbol {id:"services/crates/commerce-admin/src/main.rs::request_upstream",kind:"function"})
MERGE (upstream_problem:Symbol {id:"services/crates/commerce-admin/src/main.rs::upstream_problem",kind:"function"})
MERGE (upstream_unavailable:Symbol {id:"services/crates/commerce-admin/src/main.rs::upstream_unavailable",kind:"function"})
MERGE (upstream_too_large:Symbol {id:"services/crates/commerce-admin/src/main.rs::upstream_too_large",kind:"function"})
MERGE (bearer_token:Symbol {id:"services/crates/commerce-admin/src/main.rs::bearer_token",kind:"function"})
MERGE (authorize:Symbol {id:"services/crates/commerce-admin/src/main.rs::authorize",kind:"function"})
MERGE (correlation:Symbol {id:"services/crates/commerce-admin/src/main.rs::correlation",kind:"function"})
MERGE (now_ms:Symbol {id:"services/crates/commerce-admin/src/main.rs::now_ms",kind:"function"})
MERGE (main_fn:Symbol {id:"services/crates/commerce-admin/src/main.rs::main",kind:"function"})
MERGE (shutdown_signal:Symbol {id:"services/crates/commerce-admin/src/main.rs::shutdown_signal",kind:"function"})
MERGE (init_tracing:Symbol {id:"services/crates/commerce-admin/src/main.rs::init_tracing",kind:"function"})
MERGE (test_state:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.test_state",kind:"function"})
MERGE (mock_upstream:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.mock_upstream",kind:"function"})
MERGE (serve_mock:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.serve_mock",kind:"function"})
MERGE (login:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.login",kind:"function"})
MERGE (auth_request:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.auth_request",kind:"function"})
MERGE (read_body:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.read_body",kind:"function"})
MERGE (login_test:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.credentials_issue_opaque_sessions_and_reject_bad_passwords",kind:"test"})
MERGE (role_test:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.role_matrix_enforces_each_route_minimum",kind:"test"})
MERGE (upstream_test:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.upstream_problem_preserves_status_and_details",kind:"test"})
MERGE (reconcile_test:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.reconcile_conflict_is_propagated_without_retry",kind:"test"})
MERGE (connection_test:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.connection_failure_is_a_problem_response",kind:"test"})
MERGE (query_test:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.orders_filter_accepts_every_checkout_state",kind:"test"})
MERGE (money_test:Symbol {id:"services/crates/commerce-admin/src/main.rs::tests.upstream_money_bytes_are_forwarded_without_float_conversion",kind:"test"})
MERGE (self)-[:DEFINES]->(role)
MERGE (self)-[:DEFINES]->(session)
MERGE (self)-[:DEFINES]->(credential)
MERGE (self)-[:DEFINES]->(auth_store)
MERGE (self)-[:DEFINES]->(state)
MERGE (self)-[:DEFINES]->(upstreams)
MERGE (self)-[:DEFINES]->(authz_state)
MERGE (self)-[:DEFINES]->(correlation_id)
MERGE (self)-[:DEFINES]->(login_request)
MERGE (self)-[:DEFINES]->(session_response)
MERGE (self)-[:DEFINES]->(session_info)
MERGE (self)-[:DEFINES]->(orders_query)
MERGE (self)-[:DEFINES]->(products_query)
MERGE (self)-[:DEFINES]->(payments_query)
MERGE (self)-[:DEFINES]->(reconcile_request)
MERGE (self)-[:DEFINES]->(reservation_request)
MERGE (self)-[:DEFINES]->(mock_state)
MERGE (self)-[:DEFINES]->(observed_request)
MERGE (self)-[:DEFINES]->(mock_upstream_type)
MERGE (self)-[:DEFINES]->(session_response_test)
MERGE (self)-[:DEFINES]->(role_permits)
MERGE (self)-[:DEFINES]->(role_rank)
MERGE (self)-[:DEFINES]->(auth_store_new)
MERGE (self)-[:DEFINES]->(upstreams_from_env)
MERGE (self)-[:DEFINES]->(upstreams_all_to)
MERGE (self)-[:DEFINES]->(env_or)
MERGE (self)-[:DEFINES]->(mock_drop)
MERGE (self)-[:DEFINES]->(app_router)
MERGE (self)-[:DEFINES]->(from_env)
MERGE (self)-[:DEFINES]->(optional_credential)
MERGE (self)-[:DEFINES]->(create_session)
MERGE (self)-[:DEFINES]->(secret_matches)
MERGE (self)-[:DEFINES]->(current_session)
MERGE (self)-[:DEFINES]->(list_orders)
MERGE (self)-[:DEFINES]->(order_detail)
MERGE (self)-[:DEFINES]->(reconcile_order)
MERGE (self)-[:DEFINES]->(list_products)
MERGE (self)-[:DEFINES]->(product_detail)
MERGE (self)-[:DEFINES]->(inventory_levels)
MERGE (self)-[:DEFINES]->(list_payments)
MERGE (self)-[:DEFINES]->(reserve_inventory)
MERGE (self)-[:DEFINES]->(release_reservation)
MERGE (self)-[:DEFINES]->(encode_query)
MERGE (self)-[:DEFINES]->(encode_segment)
MERGE (self)-[:DEFINES]->(request_upstream)
MERGE (self)-[:DEFINES]->(upstream_problem)
MERGE (self)-[:DEFINES]->(upstream_unavailable)
MERGE (self)-[:DEFINES]->(upstream_too_large)
MERGE (self)-[:DEFINES]->(bearer_token)
MERGE (self)-[:DEFINES]->(authorize)
MERGE (self)-[:DEFINES]->(correlation)
MERGE (self)-[:DEFINES]->(now_ms)
MERGE (self)-[:DEFINES]->(main_fn)
MERGE (self)-[:DEFINES]->(shutdown_signal)
MERGE (self)-[:DEFINES]->(init_tracing)
MERGE (self)-[:DEFINES]->(test_state)
MERGE (self)-[:DEFINES]->(mock_upstream)
MERGE (self)-[:DEFINES]->(serve_mock)
MERGE (self)-[:DEFINES]->(login)
MERGE (self)-[:DEFINES]->(auth_request)
MERGE (self)-[:DEFINES]->(read_body)
MERGE (self)-[:DEFINES]->(login_test)
MERGE (self)-[:DEFINES]->(role_test)
MERGE (self)-[:DEFINES]->(upstream_test)
MERGE (self)-[:DEFINES]->(reconcile_test)
MERGE (self)-[:DEFINES]->(connection_test)
MERGE (self)-[:DEFINES]->(query_test)
MERGE (self)-[:DEFINES]->(money_test)
MERGE (app_router)-[:EXPOSES {method:"POST",path:"/admin/v1/sessions"}]->(create_session)
MERGE (app_router)-[:EXPOSES {method:"GET",path:"/admin/v1/session",role:"admin"}]->(current_session)
MERGE (app_router)-[:EXPOSES {method:"GET",path:"/admin/v1/orders",role:"viewer"}]->(list_orders)
MERGE (app_router)-[:EXPOSES {method:"GET",path:"/admin/v1/orders/{order_id}",role:"viewer"}]->(order_detail)
MERGE (app_router)-[:EXPOSES {method:"POST",path:"/admin/v1/orders/{order_id}/reconcile",role:"operator"}]->(reconcile_order)
MERGE (app_router)-[:EXPOSES {method:"GET",path:"/admin/v1/products",role:"viewer"}]->(list_products)
MERGE (app_router)-[:EXPOSES {method:"GET",path:"/admin/v1/products/{product_id}",role:"viewer"}]->(product_detail)
MERGE (app_router)-[:EXPOSES {method:"GET",path:"/admin/v1/inventory",role:"viewer"}]->(inventory_levels)
MERGE (app_router)-[:EXPOSES {method:"GET",path:"/admin/v1/payments",role:"viewer"}]->(list_payments)
MERGE (app_router)-[:EXPOSES {method:"POST",path:"/admin/v1/inventory/reservations",role:"operator"}]->(reserve_inventory)
MERGE (app_router)-[:EXPOSES {method:"POST",path:"/admin/v1/inventory/reservations/{reservation_id}/release",role:"operator"}]->(release_reservation)
MERGE (from_env)-[:CALLS]->(optional_credential)
MERGE (from_env)-[:CALLS]->(env_or)
MERGE (from_env)-[:CALLS]->(upstreams_from_env)
MERGE (upstreams_from_env)-[:CALLS]->(env_or)
MERGE (auth_store_new)-[:CREATES]->(auth_store)
MERGE (role_permits)-[:CALLS]->(role_rank)
MERGE (create_session)-[:CALLS]->(secret_matches)
MERGE (create_session)-[:CALLS]->(now_ms)
MERGE (create_session)-[:WRITES]->(auth_store)
MERGE (current_session)-[:READS]->(session)
MERGE (list_orders)-[:CALLS]->(encode_query)
MERGE (list_orders)-[:CALLS]->(request_upstream)
MERGE (order_detail)-[:CALLS]->(encode_segment)
MERGE (order_detail)-[:CALLS]->(request_upstream)
MERGE (reconcile_order)-[:CALLS]->(request_upstream)
MERGE (list_products)-[:CALLS]->(encode_query)
MERGE (list_products)-[:CALLS]->(request_upstream)
MERGE (product_detail)-[:CALLS]->(encode_segment)
MERGE (product_detail)-[:CALLS]->(request_upstream)
MERGE (inventory_levels)-[:CALLS]->(request_upstream)
MERGE (list_payments)-[:CALLS]->(encode_query)
MERGE (list_payments)-[:CALLS]->(request_upstream)
MERGE (reserve_inventory)-[:CALLS]->(request_upstream)
MERGE (release_reservation)-[:CALLS]->(encode_segment)
MERGE (release_reservation)-[:CALLS]->(request_upstream)
MERGE (authorize)-[:CALLS]->(bearer_token)
MERGE (authorize)-[:READS]->(auth_store)
MERGE (correlation)-[:CALLS]->(now_ms)
MERGE (request_upstream)-[:CALLS]->(upstream_problem)
MERGE (request_upstream)-[:CALLS]->(upstream_unavailable)
MERGE (request_upstream)-[:CALLS]->(upstream_too_large)
MERGE (main_fn)-[:CALLS]->(from_env)
MERGE (main_fn)-[:CALLS]->(app_router)
MERGE (main_fn)-[:CALLS]->(init_tracing)
MERGE (main_fn)-[:CALLS]->(shutdown_signal)
MERGE (test_state)-[:CALLS]->(auth_store_new)
MERGE (test_state)-[:CALLS]->(upstreams_all_to)
MERGE (mock_upstream)-[:CALLS]->(serve_mock)
MERGE (mock_upstream_type)-[:CONTAINS]->(mock_drop)
MERGE (login)-[:CALLS]->(read_body)
MERGE (login_test)-[:CALLS]->(login)
MERGE (role_test)-[:CALLS]->(login)
MERGE (role_test)-[:CALLS]->(auth_request)
MERGE (upstream_test)-[:CALLS]->(login)
MERGE (upstream_test)-[:CALLS]->(auth_request)
MERGE (upstream_test)-[:CALLS]->(read_body)
MERGE (money_test)-[:CALLS]->(login)
MERGE (money_test)-[:CALLS]->(auth_request)
MERGE (money_test)-[:CALLS]->(read_body)
MERGE (reconcile_test)-[:CALLS]->(login)
MERGE (reconcile_test)-[:CALLS]->(auth_request)
MERGE (reconcile_test)-[:CALLS]->(read_body)
MERGE (connection_test)-[:CALLS]->(test_state)
MERGE (connection_test)-[:CALLS]->(login)
MERGE (connection_test)-[:CALLS]->(auth_request)
MERGE (query_test)-[:CALLS]->(login)
MERGE (query_test)-[:CALLS]->(auth_request)
MERGE (login_test)-[:TESTS]->(create_session)
MERGE (role_test)-[:TESTS]->(authorize)
MERGE (upstream_test)-[:TESTS]->(request_upstream)
MERGE (reconcile_test)-[:TESTS]->(reconcile_order)
MERGE (connection_test)-[:TESTS]->(upstream_unavailable)
MERGE (query_test)-[:TESTS]->(list_orders)
MERGE (money_test)-[:TESTS]->(request_upstream);
@endcypher
*/
//! Admin facade for the commerce services.
//!
//! Upstream responses are forwarded without decoding them so integer minor-unit
//! money fields remain unchanged. Admin sessions are local, short-lived, and
//! never forwarded as customer identity to the upstream services.

use axum::{
    body::{Body, Bytes},
    extract::{Extension, Path, Query, Request, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use commerce_core::JsonBody;
use commerce_core::{
    error::{self, AppError, ProblemDetails},
    CheckoutState,
};
use reqwest::{redirect::Policy, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    env,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

const SESSION_TTL: Duration = Duration::from_secs(8 * 60 * 60);
const MAX_UPSTREAM_BODY_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Role {
    Viewer,
    Operator,
    Admin,
}

impl Role {
    fn permits(self, required: Self) -> bool {
        self.rank() >= required.rank()
    }

    fn rank(self) -> u8 {
        match self {
            Role::Viewer => 0,
            Role::Operator => 1,
            Role::Admin => 2,
        }
    }
}

#[derive(Clone, Copy)]
struct Session {
    role: Role,
    expires_at: Instant,
    expires_at_ms: i64,
}

struct Credential {
    user: String,
    password: String,
    role: Role,
}

#[derive(Clone)]
struct AuthStore {
    credentials: Arc<Vec<Credential>>,
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl AuthStore {
    fn new(credentials: Vec<Credential>) -> Self {
        Self {
            credentials: Arc::new(credentials),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[derive(Clone)]
pub struct AdminState {
    auth: AuthStore,
    client: Client,
    upstreams: Upstreams,
    /// Session id presented to the order service on the admin's behalf.
    ///
    /// The order service scopes `/v1/orders` and `/v1/orders/{id}` by
    /// `x-session-id` and answers `identity.missing_session` without it, so the
    /// admin facade must supply one. It is configurable because the order
    /// service treats an unknown session as an empty buyer, and a deployment
    /// that has seeded an all-orders session needs its own id here.
    admin_session_id: String,
    /// The service-to-service credential, when the deployment has a real one.
    ///
    /// The operator's view of the orders crosses the buyer boundary, so the
    /// order service only serves it to a caller that presents this. It lives
    /// in the deployment's environment and never reaches a browser.
    service_token: Option<String>,
}

#[derive(Clone)]
struct Upstreams {
    order: String,
    catalog: String,
    inventory: String,
    payment_adapter: String,
    content: String,
    _identity: String,
}

impl Upstreams {
    fn from_env() -> Self {
        Self {
            order: env_or("ORDER_URL", "http://commerce-order:8085"),
            catalog: env_or("CATALOG_URL", "http://commerce-catalog:8082"),
            inventory: env_or("INVENTORY_URL", "http://commerce-inventory:8084"),
            payment_adapter: env_or(
                "PAYMENT_ADAPTER_URL",
                "http://commerce-payment-adapter:8086",
            ),
            content: env_or("CONTENT_URL", "http://commerce-content:8096"),
            _identity: env_or("IDENTITY_URL", "http://commerce-identity:8081"),
        }
    }

    #[cfg(test)]
    fn all_to(base_url: &str) -> Self {
        Self {
            order: base_url.to_string(),
            catalog: base_url.to_string(),
            inventory: base_url.to_string(),
            payment_adapter: base_url.to_string(),
            content: base_url.to_string(),
            _identity: base_url.to_string(),
        }
    }
}

impl AdminState {
    pub fn from_env() -> Result<Self, String> {
        let admin_user = env_or("ADMIN_USER", "admin");
        let admin_password = env_or("ADMIN_PASSWORD", "dev-only-admin");
        if admin_user.is_empty() || admin_password.is_empty() {
            return Err("ADMIN_USER and ADMIN_PASSWORD must be non-empty".to_string());
        }

        let mut credentials = vec![Credential {
            user: admin_user,
            password: admin_password,
            role: Role::Admin,
        }];
        if let Some(credential) =
            optional_credential("OPERATOR_USER", "OPERATOR_PASSWORD", Role::Operator)?
        {
            credentials.push(credential);
        }
        if let Some(credential) =
            optional_credential("VIEWER_USER", "VIEWER_PASSWORD", Role::Viewer)?
        {
            credentials.push(credential);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .no_proxy()
            .redirect(Policy::none())
            .build()
            .map_err(|_| "could not create upstream HTTP client".to_string())?;

        Ok(Self {
            auth: AuthStore::new(credentials),
            client,
            upstreams: Upstreams::from_env(),
            admin_session_id: env_or("ADMIN_SESSION_ID", "admin-console"),
            service_token: commerce_core::secrets::service_token(),
        })
    }
}

fn env_or(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

fn optional_credential(
    user_key: &str,
    password_key: &str,
    role: Role,
) -> Result<Option<Credential>, String> {
    match (env::var(user_key).ok(), env::var(password_key).ok()) {
        (None, None) => Ok(None),
        (Some(user), Some(password)) if !user.is_empty() && !password.is_empty() => {
            Ok(Some(Credential {
                user,
                password,
                role,
            }))
        }
        _ => Err(format!(
            "{user_key} and {password_key} must both be non-empty"
        )),
    }
}

#[derive(Clone)]
struct AuthorizationState {
    auth: AuthStore,
    required: Role,
}

#[derive(Clone)]
struct CorrelationId(String);

/// The login body.
///
/// The contract (`contracts/openapi.yaml` -> `AdminLoginRequest`) requires
/// `username`, and that is what the Flutter console
/// (packages/api_client/lib/src/admin_api_client.dart) sends. This struct used
/// to require `user` instead, so the console could never log in: the server
/// rejected the request with 422 before it ever compared the password.
///
/// `user` is still accepted, so an older client that sends the old name keeps
/// working — the credential store is keyed on the same string either way.
#[derive(Debug, Deserialize)]
struct LoginRequest {
    #[serde(alias = "user")]
    username: String,
    password: String,
}

#[derive(Serialize)]
struct SessionResponse {
    token: String,
    role: Role,
    expires_in_ms: i64,
}

#[derive(Serialize)]
struct SessionInfo {
    role: Role,
    expires_at_ms: i64,
    expires_in_ms: i64,
}

#[derive(Debug, Deserialize)]
struct OrdersQuery {
    state: Option<CheckoutState>,
    limit: Option<u32>,
    cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProductsQuery {
    limit: Option<u32>,
    cursor: Option<String>,
}

/// The page parameters of `GET /admin/v1/inventory`.
///
/// The facade forwards them to the inventory service's listing route, so a
/// console that pages the catalogue pages the stock the same way.
#[derive(Debug, Deserialize)]
struct InventoryQuery {
    limit: Option<u32>,
    cursor: Option<String>,
}

/// The page parameters of `GET /admin/v1/payments`.
///
/// The contract gives this path an `orderId` filter and a cursor, and the
/// facade has to pass both on: it used to forward only `limit`, so the
/// contract's filters could not reach the adapter at all.
#[derive(Debug, Deserialize)]
struct PaymentsQuery {
    limit: Option<u32>,
    #[serde(rename = "orderId", alias = "order_id")]
    order_id: Option<String>,
    cursor: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReconcileRequest {
    authorized: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReservationRequest {
    sku: String,
    order_id: String,
    quantity: i64,
}

/// `GET /admin/v1/content` — the editable content document.
///
/// A viewer may read it; only an operator may write it. The write path is
/// what the operator console uses to change collections, series, trends,
/// topics, pages, FAQ, banners and size guides without a deploy.
async fn read_content(
    State(state): State<AdminState>,
    Extension(correlation): Extension<CorrelationId>,
) -> Result<Response, AppError> {
    let url = format!("{}/v1/content", state.upstreams.content);
    let response = state
        .client
        .get(&url)
        .header("x-correlation-id", correlation.0.as_str())
        .send()
        .await
        .map_err(|_| upstream_unavailable(&correlation.0))?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|_| upstream_unavailable(&correlation.0))?;
    Ok((
        status,
        [("content-type", "application/json")],
        bytes.to_vec(),
    )
        .into_response())
}

/// `PUT /admin/v1/content` — replace the content document.
///
/// The body carries the revision the editor was looking at. The content
/// service refuses a write against a stale revision, so two operators
/// cannot silently overwrite each other's work.
async fn write_content(
    State(state): State<AdminState>,
    Extension(correlation): Extension<CorrelationId>,
    body: Bytes,
) -> Result<Response, AppError> {
    let url = format!("{}/v1/content", state.upstreams.content);
    let response = state
        .client
        .put(&url)
        .header("x-correlation-id", correlation.0.as_str())
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|_| upstream_unavailable(&correlation.0))?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|_| upstream_unavailable(&correlation.0))?;
    Ok((
        status,
        [("content-type", "application/json")],
        bytes.to_vec(),
    )
        .into_response())
}

pub fn router(state: AdminState) -> Router {
    let auth = state.auth.clone();
    Router::new()
        // Unauthenticated liveness probe. Kept outside the /admin/v1 group so a
        // kubelet httpGet probe never has to present a bearer token, and kept
        // free of any upstream call so it reports process health rather than
        // dependency health — a failing dependency must not restart the pod.
        .route("/healthz", get(healthz))
        .route("/admin/v1/sessions", post(create_session))
        .route(
            "/admin/v1/session",
            get(current_session).route_layer(middleware::from_fn_with_state(
                AuthorizationState {
                    auth: auth.clone(),
                    required: Role::Admin,
                },
                authorize,
            )),
        )
        .route(
            "/admin/v1/orders",
            get(list_orders).route_layer(middleware::from_fn_with_state(
                AuthorizationState {
                    auth: auth.clone(),
                    required: Role::Viewer,
                },
                authorize,
            )),
        )
        .route(
            "/admin/v1/orders/{order_id}",
            get(order_detail).route_layer(middleware::from_fn_with_state(
                AuthorizationState {
                    auth: auth.clone(),
                    required: Role::Viewer,
                },
                authorize,
            )),
        )
        .route(
            "/admin/v1/orders/{order_id}/reconcile",
            post(reconcile_order).route_layer(middleware::from_fn_with_state(
                AuthorizationState {
                    auth: auth.clone(),
                    required: Role::Operator,
                },
                authorize,
            )),
        )
        .route(
            "/admin/v1/products",
            get(list_products).route_layer(middleware::from_fn_with_state(
                AuthorizationState {
                    auth: auth.clone(),
                    required: Role::Viewer,
                },
                authorize,
            )),
        )
        .route(
            "/admin/v1/products/{product_id}",
            get(product_detail).route_layer(middleware::from_fn_with_state(
                AuthorizationState {
                    auth: auth.clone(),
                    required: Role::Viewer,
                },
                authorize,
            )),
        )
        .route(
            "/admin/v1/inventory",
            get(inventory_levels).route_layer(middleware::from_fn_with_state(
                AuthorizationState {
                    auth: auth.clone(),
                    required: Role::Viewer,
                },
                authorize,
            )),
        )
        .route(
            "/admin/v1/payments",
            get(list_payments).route_layer(middleware::from_fn_with_state(
                AuthorizationState {
                    auth: auth.clone(),
                    required: Role::Viewer,
                },
                authorize,
            )),
        )
        .route(
            "/admin/v1/content",
            get(read_content).route_layer(middleware::from_fn_with_state(
                AuthorizationState {
                    auth: auth.clone(),
                    required: Role::Viewer,
                },
                authorize,
            )),
        )
        .route(
            "/admin/v1/content",
            put(write_content).route_layer(middleware::from_fn_with_state(
                AuthorizationState {
                    auth: auth.clone(),
                    required: Role::Operator,
                },
                authorize,
            )),
        )
        .route(
            "/admin/v1/inventory/reservations",
            post(reserve_inventory).route_layer(middleware::from_fn_with_state(
                AuthorizationState {
                    auth: auth.clone(),
                    required: Role::Operator,
                },
                authorize,
            )),
        )
        .route(
            "/admin/v1/inventory/reservations/{reservation_id}/release",
            post(release_reservation).route_layer(middleware::from_fn_with_state(
                AuthorizationState {
                    auth,
                    required: Role::Operator,
                },
                authorize,
            )),
        )
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        // The operator console is a separate Flutter Web origin (NodePort 30088),
        // so every admin call is cross-origin. Without a CORS layer the browser
        // rejects the preflight with "Method Not Allowed" and never sends the
        // real request, which presents as a login that silently does nothing.
        // Permissive mirrors the storefront BFF; a production deployment should
        // narrow allow_origin to the console's real origin.
        .layer(CorsLayer::permissive())
        .layer(middleware::from_fn(correlation))
}

/// Liveness only. Deliberately does not touch any upstream service so a
/// dependency outage shows up as failing admin requests, not as a restart loop.
async fn healthz() -> StatusCode {
    StatusCode::OK
}

async fn create_session(
    State(state): State<AdminState>,
    Extension(correlation): Extension<CorrelationId>,
    JsonBody(request): JsonBody<LoginRequest>,
) -> Result<Json<SessionResponse>, AppError> {
    let role = state
        .auth
        .credentials
        .iter()
        .find(|credential| {
            credential.user == request.username
                && secret_matches(&credential.password, &request.password)
        })
        .map(|credential| credential.role)
        .ok_or_else(|| {
            error::status::unauthorized(
                "admin.invalid_credentials",
                "user or password is incorrect",
                &correlation.0,
            )
        })?;

    let now = Instant::now();
    let now_ms = now_ms();
    let expires_at_ms = now_ms.saturating_add(SESSION_TTL.as_millis() as i64);
    let session = Session {
        role,
        expires_at: now + SESSION_TTL,
        expires_at_ms,
    };
    let token = uuid::Uuid::new_v4().simple().to_string();
    let mut sessions = state.auth.sessions.write().await;
    sessions.retain(|_, existing| existing.expires_at > now);
    sessions.insert(token.clone(), session);

    Ok(Json(SessionResponse {
        token,
        role,
        expires_in_ms: SESSION_TTL.as_millis() as i64,
    }))
}

fn secret_matches(expected: &str, supplied: &str) -> bool {
    let expected = expected.as_bytes();
    let supplied = supplied.as_bytes();
    let mut difference = u8::from(expected.len() != supplied.len());
    for index in 0..expected.len().max(supplied.len()) {
        difference |= expected.get(index).copied().unwrap_or_default()
            ^ supplied.get(index).copied().unwrap_or_default();
    }
    difference == 0
}

async fn current_session(Extension(session): Extension<Session>) -> Json<SessionInfo> {
    Json(SessionInfo {
        role: session.role,
        expires_at_ms: session.expires_at_ms,
        expires_in_ms: session
            .expires_at
            .saturating_duration_since(Instant::now())
            .as_millis() as i64,
    })
}

async fn list_orders(
    State(state): State<AdminState>,
    Query(query): Query<OrdersQuery>,
    Extension(correlation): Extension<CorrelationId>,
) -> Result<Response, AppError> {
    let mut pairs = Vec::new();
    if let Some(value) = query.state {
        pairs.push(("state".to_string(), value.as_str().to_string()));
    }
    if let Some(value) = query.limit {
        pairs.push(("limit".to_string(), value.to_string()));
    }
    if let Some(value) = query.cursor {
        pairs.push(("cursor".to_string(), value));
    }
    request_upstream_as_operator(
        &state,
        &state.upstreams.order,
        "/v1/operator/orders",
        Method::GET,
        encode_query(pairs),
        None,
        &correlation,
    )
    .await
}

async fn order_detail(
    State(state): State<AdminState>,
    Path(order_id): Path<String>,
    Extension(correlation): Extension<CorrelationId>,
) -> Result<Response, AppError> {
    request_upstream_as_operator(
        &state,
        &state.upstreams.order,
        &format!("/v1/operator/orders/{}", encode_segment(&order_id)),
        Method::GET,
        None,
        None,
        &correlation,
    )
    .await
}

async fn reconcile_order(
    State(state): State<AdminState>,
    Path(order_id): Path<String>,
    Extension(correlation): Extension<CorrelationId>,
    JsonBody(body): JsonBody<ReconcileRequest>,
) -> Result<Response, AppError> {
    request_upstream_as_operator(
        &state,
        &state.upstreams.order,
        &format!(
            "/v1/operator/orders/{}/reconcile",
            encode_segment(&order_id)
        ),
        Method::POST,
        None,
        Some(serde_json::json!({ "authorized": body.authorized })),
        &correlation,
    )
    .await
}

async fn list_products(
    State(state): State<AdminState>,
    Query(query): Query<ProductsQuery>,
    Extension(correlation): Extension<CorrelationId>,
) -> Result<Response, AppError> {
    let mut pairs = Vec::new();
    if let Some(value) = query.limit {
        pairs.push(("limit".to_string(), value.to_string()));
    }
    if let Some(value) = query.cursor {
        pairs.push(("cursor".to_string(), value));
    }
    request_upstream(
        &state,
        &state.upstreams.catalog,
        "/v1/products",
        Method::GET,
        encode_query(pairs),
        None,
        &correlation,
    )
    .await
}

async fn product_detail(
    State(state): State<AdminState>,
    Path(product_id): Path<String>,
    Extension(correlation): Extension<CorrelationId>,
) -> Result<Response, AppError> {
    request_upstream(
        &state,
        &state.upstreams.catalog,
        &format!("/v1/products/{}", encode_segment(&product_id)),
        Method::GET,
        None,
        None,
        &correlation,
    )
    .await
}

/// `GET /admin/v1/inventory` — the stock levels the inventory service holds.
///
/// The facade proxies this to the inventory service's listing route and
/// forwards the answer unchanged, so the console sees the live ledger rather
/// than a copy of it. The rows are keyed by SKU: SKU is the inventory
/// service's key, and product identity belongs to the catalogue.
///
/// An upstream that cannot be reached is still an error — `admin.upstream_error`
/// for an upstream that answers with a failure the facade cannot interpret,
/// `admin.upstream_unavailable` for one that cannot be reached at all. A
/// listing that cannot be served must not read as an empty stock list.
async fn inventory_levels(
    State(state): State<AdminState>,
    Query(query): Query<InventoryQuery>,
    Extension(correlation): Extension<CorrelationId>,
) -> Result<Response, AppError> {
    let mut pairs = Vec::new();
    if let Some(value) = query.limit {
        pairs.push(("limit".to_string(), value.to_string()));
    }
    if let Some(value) = query.cursor {
        pairs.push(("cursor".to_string(), value));
    }
    request_upstream(
        &state,
        &state.upstreams.inventory,
        "/v1/inventory",
        Method::GET,
        encode_query(pairs),
        None,
        &correlation,
    )
    .await
}

async fn list_payments(
    State(state): State<AdminState>,
    Query(query): Query<PaymentsQuery>,
    Extension(correlation): Extension<CorrelationId>,
) -> Result<Response, AppError> {
    let mut pairs = Vec::new();
    if let Some(value) = query.limit {
        pairs.push(("limit".to_string(), value.to_string()));
    }
    if let Some(value) = query.order_id {
        pairs.push(("orderId".to_string(), value));
    }
    if let Some(value) = query.cursor {
        pairs.push(("cursor".to_string(), value));
    }
    request_upstream(
        &state,
        &state.upstreams.payment_adapter,
        "/v1/payments",
        Method::GET,
        encode_query(pairs),
        None,
        &correlation,
    )
    .await
}

async fn reserve_inventory(
    State(state): State<AdminState>,
    Extension(correlation): Extension<CorrelationId>,
    JsonBody(body): JsonBody<ReservationRequest>,
) -> Result<Response, AppError> {
    request_upstream(
        &state,
        &state.upstreams.inventory,
        "/v1/inventory/reservations",
        Method::POST,
        None,
        Some(serde_json::json!({
            "sku": body.sku,
            "order_id": body.order_id,
            "quantity": body.quantity,
        })),
        &correlation,
    )
    .await
}

async fn release_reservation(
    State(state): State<AdminState>,
    Path(reservation_id): Path<String>,
    Extension(correlation): Extension<CorrelationId>,
) -> Result<Response, AppError> {
    request_upstream(
        &state,
        &state.upstreams.inventory,
        &format!(
            "/v1/inventory/reservations/{}/release",
            encode_segment(&reservation_id)
        ),
        Method::POST,
        None,
        None,
        &correlation,
    )
    .await
}

fn encode_query(pairs: Vec<(String, String)>) -> Option<String> {
    let encoded = pairs
        .into_iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                urlencoding::encode(&key),
                urlencoding::encode(&value)
            )
        })
        .collect::<Vec<_>>();
    (!encoded.is_empty()).then(|| encoded.join("&"))
}

fn encode_segment(segment: &str) -> String {
    urlencoding::encode(segment).into_owned()
}

/// One call to an upstream service.
///
/// The request is a struct rather than a list of parameters because the two
/// callers differ in one respect — whether they ask for the operator's view —
/// and a flag in the middle of a parameter list is easy to read wrong.
struct UpstreamCall<'a> {
    base_url: &'a str,
    path: &'a str,
    method: Method,
    query: Option<String>,
    body: Option<Value>,
    operator: bool,
}

async fn request_upstream(
    state: &AdminState,
    base_url: &str,
    path: &str,
    method: Method,
    query: Option<String>,
    body: Option<Value>,
    correlation: &CorrelationId,
) -> Result<Response, AppError> {
    request_upstream_with(
        state,
        correlation,
        UpstreamCall {
            base_url,
            path,
            method,
            query,
            body,
            operator: false,
        },
    )
    .await
}

/// The same call, as the operator.
///
/// It presents the service-to-service credential and asks for the operator's
/// view of the orders — the one that crosses the buyer boundary, and the one
/// the contract describes for `GET /admin/v1/orders`.
async fn request_upstream_as_operator(
    state: &AdminState,
    base_url: &str,
    path: &str,
    method: Method,
    query: Option<String>,
    body: Option<Value>,
    correlation: &CorrelationId,
) -> Result<Response, AppError> {
    request_upstream_with(
        state,
        correlation,
        UpstreamCall {
            base_url,
            path,
            method,
            query,
            body,
            operator: true,
        },
    )
    .await
}

async fn request_upstream_with(
    state: &AdminState,
    correlation: &CorrelationId,
    call: UpstreamCall<'_>,
) -> Result<Response, AppError> {
    let url = match call.query {
        Some(query) => format!(
            "{}{}?{}",
            call.base_url.trim_end_matches('/'),
            call.path,
            query
        ),
        None => format!("{}{}", call.base_url.trim_end_matches('/'), call.path),
    };
    let request = state
        .client
        .request(call.method, url)
        .header("x-correlation-id", correlation.0.as_str())
        // The order service scopes reads by buyer and rejects a request with no
        // session (`identity.missing_session`, 401). The admin facade acts on
        // behalf of the configured admin session rather than a customer, so it
        // must present one or every order read fails.
        .header("x-session-id", state.admin_session_id.as_str());
    // The operator's view crosses the buyer boundary, so the order service
    // only serves it to a caller that holds the service-to-service
    // credential. Without one the answer is "not available", never "every
    // order".
    let request = if call.operator {
        match state.service_token.as_deref() {
            Some(token) => request.header("x-service-token", token),
            None => {
                return Err(error::status::service_unavailable(
                    "admin.no_service_credential",
                    "this deployment has no service-to-service credential, so the operator's \
                     view of the orders is not available",
                    &correlation.0,
                ));
            }
        }
    } else {
        request
    };
    let request = match call.body {
        Some(body) => request.json(&body),
        None => request,
    };
    let mut upstream = request
        .send()
        .await
        .map_err(|_| upstream_unavailable(&correlation.0))?;
    let status =
        StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    if upstream
        .content_length()
        .is_some_and(|length| length > MAX_UPSTREAM_BODY_BYTES as u64)
    {
        return Err(upstream_too_large(&correlation.0));
    }

    let content_type = upstream.headers().get(header::CONTENT_TYPE).cloned();
    let mut bytes = Vec::new();
    while let Some(chunk) = upstream
        .chunk()
        .await
        .map_err(|_| upstream_unavailable(&correlation.0))?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_UPSTREAM_BODY_BYTES {
            return Err(upstream_too_large(&correlation.0));
        }
        bytes.extend_from_slice(&chunk);
    }
    if !status.is_success() {
        return Err(upstream_problem(status, &bytes, &correlation.0));
    }

    let mut response = Response::builder().status(status);
    if let Some(content_type) = content_type {
        response = response.header(header::CONTENT_TYPE, content_type);
    }
    response
        .body(Body::from(Bytes::from(bytes)))
        .map_err(|_| AppError::Internal("could not build upstream response".to_string()))
}

fn upstream_problem(status: StatusCode, body: &[u8], trace_id: &str) -> AppError {
    if let Ok(mut problem) = serde_json::from_slice::<ProblemDetails>(body) {
        problem.status = status.as_u16();
        return AppError::problem(problem);
    }
    AppError::problem(ProblemDetails::new(
        "https://api.gemarc.example/problems/upstream",
        status.canonical_reason().unwrap_or("Upstream Error"),
        status,
        "admin.upstream_error",
        format!("upstream service returned HTTP {}", status.as_u16()),
        trace_id,
        status.is_server_error(),
    ))
}

fn upstream_unavailable(trace_id: &str) -> AppError {
    error::status::service_unavailable(
        "admin.upstream_unavailable",
        "upstream service is unavailable",
        trace_id,
    )
}

fn upstream_too_large(trace_id: &str) -> AppError {
    AppError::problem(ProblemDetails::new(
        "https://api.gemarc.example/problems/upstream",
        "Bad Gateway",
        StatusCode::BAD_GATEWAY,
        "admin.upstream_response_too_large",
        "upstream response exceeded the configured size limit",
        trace_id,
        false,
    ))
}

async fn authorize(
    State(state): State<AuthorizationState>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(token) = bearer_token(request.headers()) else {
        return error::status::unauthorized(
            "admin.auth.required",
            "a valid bearer token is required",
            "",
        )
        .into_response();
    };

    let session = {
        let mut sessions = state.auth.sessions.write().await;
        match sessions.get(token).copied() {
            Some(session) if session.expires_at > Instant::now() => Some(session),
            Some(_) => {
                sessions.remove(token);
                None
            }
            None => None,
        }
    };
    let Some(session) = session else {
        return error::status::unauthorized(
            "admin.auth.invalid_token",
            "bearer token is invalid or expired",
            "",
        )
        .into_response();
    };
    if !session.role.permits(state.required) {
        return error::status::forbidden(
            "admin.auth.insufficient_role",
            "the authenticated role is not permitted for this route",
            "",
        )
        .into_response();
    }

    request.extensions_mut().insert(session);
    next.run(request).await
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let mut parts = value.split_whitespace();
    let scheme = parts.next()?;
    let token = parts.next()?;
    if !scheme.eq_ignore_ascii_case("bearer") || token.is_empty() || parts.next().is_some() {
        return None;
    }
    Some(token)
}

async fn correlation(mut request: Request, next: Next) -> Response {
    let correlation_id = request
        .headers()
        .get("x-correlation-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let Ok(header_value) = HeaderValue::from_str(&correlation_id) else {
        return error::status::bad_request(
            "admin.invalid_correlation_id",
            "x-correlation-id is invalid",
            "",
        )
        .into_response();
    };
    request
        .extensions_mut()
        .insert(CorrelationId(correlation_id));
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert("x-correlation-id", header_value);
    response
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

/// Wait for the process to be asked to stop.
///
/// The binary serves with a graceful shutdown, so a deployment that is being
/// replaced lets the request in flight finish instead of cutting it off.
pub async fn shutdown_signal() {
    if tokio::signal::ctrl_c().await.is_ok() {
        tracing::info!("shutdown signal received");
    }
}

/// The logs are the only way to see what the service did, so they are always
/// on.
pub fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::any;
    use http_body_util::BodyExt;
    use std::sync::Mutex;
    use tower::ServiceExt;

    /// The URIs the mock upstream was called with.
    #[derive(Clone, Default)]
    struct Observed {
        uris: Arc<Mutex<Vec<String>>>,
    }

    /// A stand-in for an upstream service.
    ///
    /// It records every URI it is called with, so a test can assert that the
    /// facade called the route it is supposed to call and not something else.
    async fn mock_upstream(status: StatusCode, body: String) -> (String, Observed) {
        let observed = Observed::default();
        let sink = observed.clone();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new().fallback(any(move |request: Request| {
            let sink = sink.clone();
            let body = body.clone();
            async move {
                sink.uris.lock().unwrap().push(request.uri().to_string());
                (status, [("content-type", "application/json")], body)
            }
        }));
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        (format!("http://{address}"), observed)
    }

    /// The admin state, with every upstream pointing at the mock.
    fn test_state(base_url: &str) -> AdminState {
        AdminState {
            auth: AuthStore::new(vec![Credential {
                user: "admin".to_string(),
                password: "admin-pass".to_string(),
                role: Role::Admin,
            }]),
            client: Client::builder()
                .timeout(Duration::from_secs(5))
                .no_proxy()
                .redirect(Policy::none())
                .build()
                .expect("client"),
            upstreams: Upstreams::all_to(base_url),
            admin_session_id: "admin-console".to_string(),
            service_token: Some("test-service-token".to_string()),
        }
    }

    /// Log in and return the bearer token.
    async fn login(app: &Router) -> String {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/admin/v1/sessions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"username":"admin","password":"admin-pass"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .expect("the login route must answer");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).expect("json body");
        json["token"].as_str().expect("a token").to_string()
    }

    /// An authenticated `GET` against the admin router.
    async fn auth_get(app: &Router, token: &str, uri: &str) -> Response {
        app.clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("the route must answer")
    }

    async fn body_json(response: Response) -> Value {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&body).expect("json body")
    }

    /// The admin facade must serve the inventory listing by proxying it to
    /// the inventory service's listing route.
    ///
    /// The route used to answer 404, because the inventory service had no
    /// read route to proxy to: the console had no stock view at all.
    #[tokio::test]
    async fn the_inventory_listing_is_proxied_to_the_inventory_service() {
        let (base_url, observed) = mock_upstream(
            StatusCode::OK,
            r#"{"items":[{"sku":"sku-bra-01","onHand":42,"reserved":0,"available":42}],"next_cursor":null}"#
                .to_string(),
        )
        .await;
        let app = router(test_state(&base_url));
        let token = login(&app).await;

        let response = auth_get(&app, &token, "/admin/v1/inventory").await;

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["items"][0]["sku"], "sku-bra-01");
        assert_eq!(json["items"][0]["onHand"], 42);
        assert_eq!(json["items"][0]["available"], 42);
        // The facade must call the inventory service's listing route.
        assert_eq!(
            observed.uris.lock().unwrap().clone(),
            vec!["/v1/inventory".to_string()],
        );
    }

    /// The page parameters the contract declares must reach the upstream, or
    /// a console that asks for one page gets the whole store.
    #[tokio::test]
    async fn the_inventory_listing_forwards_the_page_parameters() {
        let (base_url, observed) = mock_upstream(
            StatusCode::OK,
            r#"{"items":[],"next_cursor":null}"#.to_string(),
        )
        .await;
        let app = router(test_state(&base_url));
        let token = login(&app).await;

        let response = auth_get(&app, &token, "/admin/v1/inventory?limit=1&cursor=2").await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            observed.uris.lock().unwrap().clone(),
            vec!["/v1/inventory?limit=1&cursor=2".to_string()],
        );
    }

    /// An upstream that fails is still an error.
    ///
    /// The facade used to relay the inventory service's 404 as
    /// `admin.upstream_error`; now that the service has the route, this is
    /// the path a genuine upstream failure takes, and it must not be turned
    /// into an empty stock list.
    #[tokio::test]
    async fn a_failing_upstream_is_still_an_error() {
        let (base_url, _observed) = mock_upstream(StatusCode::NOT_FOUND, String::new()).await;
        let app = router(test_state(&base_url));
        let token = login(&app).await;

        let response = auth_get(&app, &token, "/admin/v1/inventory").await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let json = body_json(response).await;
        assert_eq!(json["code"], "admin.upstream_error");
    }

    /// An upstream that cannot be reached at all is a 503, not an empty list.
    #[tokio::test]
    async fn an_unreachable_upstream_is_a_problem_response() {
        // Port 1 on loopback refuses connections.
        let app = router(test_state("http://127.0.0.1:1"));
        let token = login(&app).await;

        let response = auth_get(&app, &token, "/admin/v1/inventory").await;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let json = body_json(response).await;
        assert_eq!(json["code"], "admin.upstream_unavailable");
    }
}
