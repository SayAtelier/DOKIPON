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
    routing::{get, post},
    Json, Router,
};
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
struct AdminState {
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
}

#[derive(Clone)]
struct Upstreams {
    order: String,
    catalog: String,
    inventory: String,
    payment_adapter: String,
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
            _identity: base_url.to_string(),
        }
    }
}

impl AdminState {
    fn from_env() -> Result<Self, String> {
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

#[derive(Debug, Deserialize)]
struct PaymentsQuery {
    limit: Option<u32>,
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

fn router(state: AdminState) -> Router {
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
    Json(request): Json<LoginRequest>,
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
    request_upstream(
        &state,
        &state.upstreams.order,
        "/v1/orders",
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
    request_upstream(
        &state,
        &state.upstreams.order,
        &format!("/v1/orders/{}", encode_segment(&order_id)),
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
    Json(body): Json<ReconcileRequest>,
) -> Result<Response, AppError> {
    request_upstream(
        &state,
        &state.upstreams.order,
        &format!("/v1/orders/{}/reconcile", encode_segment(&order_id)),
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

async fn inventory_levels(
    State(state): State<AdminState>,
    Extension(correlation): Extension<CorrelationId>,
) -> Result<Response, AppError> {
    request_upstream(
        &state,
        &state.upstreams.inventory,
        "/v1/inventory",
        Method::GET,
        None,
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
    let pairs = query
        .limit
        .map(|value| vec![("limit".to_string(), value.to_string())])
        .unwrap_or_default();
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
    Json(body): Json<ReservationRequest>,
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

async fn request_upstream(
    state: &AdminState,
    base_url: &str,
    path: &str,
    method: Method,
    query: Option<String>,
    body: Option<Value>,
    correlation: &CorrelationId,
) -> Result<Response, AppError> {
    let url = match query {
        Some(query) => format!("{}{}?{}", base_url.trim_end_matches('/'), path, query),
        None => format!("{}{}", base_url.trim_end_matches('/'), path),
    };
    let request = state
        .client
        .request(method, url)
        .header("x-correlation-id", correlation.0.as_str())
        // The order service scopes reads by buyer and rejects a request with no
        // session (`identity.missing_session`, 401). The admin facade acts on
        // behalf of the configured admin session rather than a customer, so it
        // must present one or every order read fails.
        .header("x-session-id", state.admin_session_id.as_str());
    let request = match body {
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
        return AppError::Problem(problem);
    }
    AppError::Problem(ProblemDetails::new(
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
    AppError::Problem(ProblemDetails::new(
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let state = AdminState::from_env()
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;
    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8087);
    let listener =
        tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port))).await?;
    tracing::info!("admin service listening on {}", listener.local_addr()?);
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    if tokio::signal::ctrl_c().await.is_ok() {
        tracing::info!("shutdown signal received");
    }
}

fn init_tracing() {
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
    use axum::{body::to_bytes, extract::State, routing::any};
    use http_body_util::BodyExt;
    use std::{net::SocketAddr, sync::Mutex};
    use tokio::task::JoinHandle;
    use tower::ServiceExt;

    #[derive(Clone)]
    struct MockState {
        status: StatusCode,
        response_body: Arc<Vec<u8>>,
        requests: Arc<Mutex<Vec<ObservedRequest>>>,
    }

    #[derive(Clone)]
    struct ObservedRequest {
        method: String,
        uri: String,
        body: Vec<u8>,
        has_authorization: bool,
    }

    struct MockUpstream {
        base_url: String,
        task: JoinHandle<()>,
        requests: Arc<Mutex<Vec<ObservedRequest>>>,
    }

    impl Drop for MockUpstream {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    fn test_state(base_url: &str) -> AdminState {
        let credentials = vec![
            Credential {
                user: "admin".to_string(),
                password: "admin-password".to_string(),
                role: Role::Admin,
            },
            Credential {
                user: "operator".to_string(),
                password: "operator-password".to_string(),
                role: Role::Operator,
            },
            Credential {
                user: "viewer".to_string(),
                password: "viewer-password".to_string(),
                role: Role::Viewer,
            },
        ];
        AdminState {
            auth: AuthStore::new(credentials),
            client: Client::builder()
                .timeout(Duration::from_secs(2))
                .no_proxy()
                .redirect(Policy::none())
                .build()
                .unwrap(),
            upstreams: Upstreams::all_to(base_url),
            admin_session_id: "admin-console".to_string(),
        }
    }

    async fn mock_upstream(status: StatusCode, response_body: Vec<u8>) -> MockUpstream {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address: SocketAddr = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let state = MockState {
            status,
            response_body: Arc::new(response_body),
            requests: requests.clone(),
        };
        let app = Router::new().fallback(any(serve_mock)).with_state(state);
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        MockUpstream {
            base_url: format!("http://{address}"),
            task,
            requests,
        }
    }

    async fn serve_mock(State(state): State<MockState>, request: Request) -> Response {
        let (parts, body) = request.into_parts();
        let body = to_bytes(body, MAX_UPSTREAM_BODY_BYTES)
            .await
            .unwrap_or_default()
            .to_vec();
        state.requests.lock().unwrap().push(ObservedRequest {
            method: parts.method.to_string(),
            uri: parts.uri.to_string(),
            body,
            has_authorization: parts.headers.contains_key(header::AUTHORIZATION),
        });
        Response::builder()
            .status(state.status)
            .header(header::CONTENT_TYPE, "application/problem+json")
            .body(Body::from(state.response_body.as_ref().clone()))
            .unwrap()
    }

    async fn login(app: &Router, user: &str, password: &str) -> String {
        // `username` is the contract's field name
        // (contracts/openapi.yaml -> AdminLoginRequest) and the name the
        // Flutter console sends. The tests used to post `user` here, which
        // agreed with the server and so never caught the mismatch.
        let body = serde_json::json!({ "username": user, "password": password }).to_string();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/admin/v1/sessions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = read_body(response).await;
        serde_json::from_slice::<SessionResponseForTest>(&body)
            .unwrap()
            .token
    }

    #[derive(Deserialize)]
    struct SessionResponseForTest {
        token: String,
    }

    /// The contract's login field name must work, and the old one must keep
    /// working.
    ///
    /// The server used to require `user` while the contract and the Flutter
    /// console both say `username`, so the console could never sign in — the
    /// server answered 422 before it ever looked at the password. The unit
    /// tests did not notice because they all posted `user`, agreeing with the
    /// bug.
    #[tokio::test]
    async fn login_accepts_the_contract_field_name() {
        let upstream = mock_upstream(StatusCode::OK, br#"{"items":[]}"#.to_vec()).await;
        let app = router(test_state(&upstream.base_url));

        for body in [
            r#"{"username":"admin","password":"admin-password"}"#,
            r#"{"user":"admin","password":"admin-password"}"#,
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(Method::POST)
                        .uri("/admin/v1/sessions")
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(body.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "login with body {body} must succeed"
            );
        }
    }

    /// A login body with neither field name is a client error, not a 401:
    /// the request never carried an identity to check.
    #[tokio::test]
    async fn login_without_any_username_field_is_rejected() {
        let upstream = mock_upstream(StatusCode::OK, br#"{"items":[]}"#.to_vec()).await;
        let app = router(test_state(&upstream.base_url));

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/admin/v1/sessions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"password":"admin-password"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    fn auth_request(token: &str, method: Method, uri: &str, body: Option<Value>) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {token}"));
        let body = match body {
            Some(body) => {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                Body::from(body.to_string())
            }
            None => Body::empty(),
        };
        builder.body(body).unwrap()
    }

    async fn read_body(response: Response) -> Vec<u8> {
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec()
    }

    #[tokio::test]
    async fn credentials_issue_opaque_sessions_and_reject_bad_passwords() {
        let upstream = mock_upstream(StatusCode::OK, br#"{"items":[]}"#.to_vec()).await;
        let app = router(test_state(&upstream.base_url));

        let bad = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/admin/v1/sessions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"user":"admin","password":"wrong"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            bad.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/problem+json"
        );

        let token = login(&app, "admin", "admin-password").await;
        assert_eq!(token.len(), 32);
        assert!(token.bytes().all(|byte| byte.is_ascii_hexdigit()));
        let current = app
            .oneshot(auth_request(&token, Method::GET, "/admin/v1/session", None))
            .await
            .unwrap();
        assert_eq!(current.status(), StatusCode::OK);
        let body: Value = serde_json::from_slice(&read_body(current).await).unwrap();
        assert_eq!(body["role"], "admin");
        assert!(body["expires_at_ms"].as_i64().unwrap() > now_ms());
    }

    #[tokio::test]
    async fn role_matrix_enforces_each_route_minimum() {
        let upstream = mock_upstream(StatusCode::OK, br#"{"items":[]}"#.to_vec()).await;
        let app = router(test_state(&upstream.base_url));
        let anonymous = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/admin/v1/products")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
        let roles = [
            ("viewer", "viewer-password", StatusCode::FORBIDDEN),
            ("operator", "operator-password", StatusCode::OK),
            ("admin", "admin-password", StatusCode::OK),
        ];

        for (user, password, write_status) in roles {
            let token = login(&app, user, password).await;
            let read = app
                .clone()
                .oneshot(auth_request(
                    &token,
                    Method::GET,
                    "/admin/v1/products",
                    None,
                ))
                .await
                .unwrap();
            assert_eq!(read.status(), StatusCode::OK, "{user} must read");

            let write = app
                .clone()
                .oneshot(auth_request(
                    &token,
                    Method::POST,
                    "/admin/v1/inventory/reservations",
                    Some(serde_json::json!({
                        "sku": "sku-1",
                        "order_id": "order-1",
                        "quantity": 1,
                    })),
                ))
                .await
                .unwrap();
            assert_eq!(write.status(), write_status, "{user} write permission");

            let session = app
                .clone()
                .oneshot(auth_request(&token, Method::GET, "/admin/v1/session", None))
                .await
                .unwrap();
            assert_eq!(
                session.status(),
                if user == "admin" {
                    StatusCode::OK
                } else {
                    StatusCode::FORBIDDEN
                },
                "{user} session inspection permission"
            );
        }
    }

    #[tokio::test]
    async fn upstream_problem_preserves_status_and_details() {
        let problem = ProblemDetails::new(
            "https://api.gemarc.example/problems/service-unavailable",
            "Service Unavailable",
            StatusCode::SERVICE_UNAVAILABLE,
            "catalog.unavailable",
            "catalog is restarting",
            "upstream-trace",
            true,
        );
        let upstream = mock_upstream(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::to_vec(&problem).unwrap(),
        )
        .await;
        let app = router(test_state(&upstream.base_url));
        let token = login(&app, "viewer", "viewer-password").await;
        let response = app
            .oneshot(auth_request(
                &token,
                Method::GET,
                "/admin/v1/products",
                None,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/problem+json"
        );
        let body: ProblemDetails = serde_json::from_slice(&read_body(response).await).unwrap();
        assert_eq!(body.status, StatusCode::SERVICE_UNAVAILABLE.as_u16());
        assert_eq!(body.code, "catalog.unavailable");
        assert_eq!(body.trace_id, "upstream-trace");
    }

    #[tokio::test]
    async fn upstream_money_bytes_are_forwarded_without_float_conversion() {
        let payload = br#"{"amountMinor":9007199254740993,"currency":"USD"}"#.to_vec();
        let upstream = mock_upstream(StatusCode::OK, payload.clone()).await;
        let app = router(test_state(&upstream.base_url));
        let token = login(&app, "viewer", "viewer-password").await;
        let response = app
            .oneshot(auth_request(
                &token,
                Method::GET,
                "/admin/v1/products",
                None,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(read_body(response).await, payload);
    }

    #[tokio::test]
    async fn reconcile_conflict_is_propagated_without_retry() {
        let problem = ProblemDetails::new(
            "https://api.gemarc.example/problems/conflict",
            "Conflict",
            StatusCode::CONFLICT,
            "checkout.not_awaiting_reconciliation",
            "order is not awaiting reconciliation",
            "order-trace",
            false,
        );
        let upstream =
            mock_upstream(StatusCode::CONFLICT, serde_json::to_vec(&problem).unwrap()).await;
        let app = router(test_state(&upstream.base_url));
        let token = login(&app, "operator", "operator-password").await;
        let response = app
            .oneshot(auth_request(
                &token,
                Method::POST,
                "/admin/v1/orders/order-1/reconcile",
                Some(serde_json::json!({ "authorized": true })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body: ProblemDetails = serde_json::from_slice(&read_body(response).await).unwrap();
        assert_eq!(body.code, "checkout.not_awaiting_reconciliation");

        let requests = upstream.requests.lock().unwrap();
        assert_eq!(
            requests.len(),
            1,
            "reconcile is forwarded once, without retry"
        );
        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].uri, "/v1/orders/order-1/reconcile");
        assert_eq!(
            serde_json::from_slice::<Value>(&requests[0].body).unwrap(),
            serde_json::json!({ "authorized": true })
        );
        assert!(
            !requests[0].has_authorization,
            "admin tokens stay inside the facade"
        );
    }

    #[tokio::test]
    async fn connection_failure_is_a_problem_response() {
        let app = router(test_state("http://127.0.0.1:1"));
        let token = login(&app, "viewer", "viewer-password").await;
        let response = app
            .oneshot(auth_request(
                &token,
                Method::GET,
                "/admin/v1/products",
                None,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/problem+json"
        );
    }

    #[tokio::test]
    async fn orders_filter_accepts_every_checkout_state() {
        let upstream = mock_upstream(StatusCode::OK, br#"{"items":[]}"#.to_vec()).await;
        let app = router(test_state(&upstream.base_url));
        let token = login(&app, "viewer", "viewer-password").await;
        let states = [
            "new",
            "reserving",
            "stock_reserved",
            "awaiting_payment",
            "payment_unknown",
            "confirming",
            "confirmed",
            "rejected",
            "cancelled",
        ];

        for state in states {
            let response = app
                .clone()
                .oneshot(auth_request(
                    &token,
                    Method::GET,
                    &format!("/admin/v1/orders?state={state}&limit=5&cursor=next"),
                    None,
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "state {state}");
        }

        let requests = upstream.requests.lock().unwrap();
        assert_eq!(requests.len(), states.len());
        for (request, state) in requests.iter().zip(states) {
            assert_eq!(
                request.uri,
                format!("/v1/orders?state={state}&limit=5&cursor=next")
            );
        }
    }
}
