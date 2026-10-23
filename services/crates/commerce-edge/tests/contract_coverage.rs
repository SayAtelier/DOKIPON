//! Every path in the OpenAPI contract is served by the gateway.
//!
//! ## What this test is for
//!
//! `contracts/openapi.yaml` is the contract of record, and the Dart client has
//! exactly one `baseUri`
//! (`packages/api_client/lib/src/commerce_api_client.dart`). That makes the
//! gateway's job precise: **one origin must serve every path in the
//! contract**. Before the edge existed, the twelve paths were spread over five
//! binaries and no single process served them all — `/v1/sessions/line` lived
//! only in the identity service, so a client pointed at the BFF could never
//! exchange a LINE token.
//!
//! This test reads the contract itself rather than a hand-written list of
//! paths, so it cannot fall behind the contract: a path added to the YAML and
//! not served by the gateway fails here.
//!
//! ## How "present and responds" is decided
//!
//! A routing miss and a domain miss look alike from the outside — both are
//! `404`. They are not the same thing:
//!
//! - a **routing miss** is the edge's own `edge.no_such_route` problem: no
//!   service claims the path;
//! - a **domain miss** is a service answering properly — `cart.not_found`,
//!   `catalog.product_not_found` — which means the path is routed and the
//!   handler ran.
//!
//! So the assertion is: the path+method is not `405 Method Not Allowed`, and
//! the body is not the edge's no-such-route problem. A domain `404` passes;
//! an unrouted path does not.
//!
//! ## No network, no ports
//!
//! Every request goes through `tower::ServiceExt::oneshot`, so the router is
//! driven in-process. Nothing binds a port and nothing is dialled.

use std::path::PathBuf;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_yaml::Value;
use tower::ServiceExt;

/// The value substituted for every path parameter.
const PARAM_SAMPLE: &str = "sample-id";

/// The contract of record, read from the repository.
fn contract() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../contracts/openapi.yaml");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_yaml::from_str(&raw).expect("contracts/openapi.yaml must be valid YAML")
}

/// The gateway under test: the edge, which mounts every service router in one
/// process behind one router.
fn gateway() -> Router {
    let admin = commerce_admin::AdminState::from_env()
        .expect("the admin facade builds without a database");
    let (edge, _services) = commerce_edge::build(admin);
    edge
}

/// Replace every `{param}` in a contract path with a sample value.
fn concrete(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut rest = path;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let end = start
            + rest[start..]
                .find('}')
                .expect("a path parameter in the contract is closed");
        out.push_str(PARAM_SAMPLE);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

/// The HTTP methods a contract path item may declare.
fn methods(item: &Value) -> Vec<Method> {
    let mut methods = Vec::new();
    let Some(entries) = item.as_mapping() else {
        return methods;
    };
    for (name, _) in entries {
        let Some(name) = name.as_str() else { continue };
        let method = match name {
            "get" => Method::GET,
            "post" => Method::POST,
            "put" => Method::PUT,
            "patch" => Method::PATCH,
            "delete" => Method::DELETE,
            // `parameters`, `summary`, `tags`, `security`, ... are not methods.
            _ => continue,
        };
        methods.push(method);
    }
    methods
}

/// The body the edge answers with when it cannot route a path at all.
fn is_no_such_route(body: &str) -> bool {
    body.contains("edge.no_such_route")
}

#[tokio::test]
async fn every_contract_path_is_served_by_the_gateway() {
    let doc = contract();
    let paths = doc["paths"]
        .as_mapping()
        .expect("the contract declares a paths map");

    let mut checked = 0usize;
    for (path, item) in paths {
        let path = path.as_str().expect("a contract path is a string");
        let concrete_path = concrete(path);

        // The shared path table must know who owns the path. This is the
        // table both the edge and the BFF read, so a path the table does not
        // know would 404 in one deployment or the other.
        assert!(
            commerce_core::routes::owner_of(&concrete_path).is_some(),
            "{path} has no owning service in commerce_core::routes"
        );

        for method in methods(item) {
            let request = Request::builder()
                .method(method.clone())
                .uri(&concrete_path)
                .header("x-session-id", "sess-000001")
                .header("idempotency-key", "11111111-1111-1111-1111-111111111111")
                .header("x-channel", "web")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("the request builds");

            let response = gateway()
                .oneshot(request)
                .await
                .expect("the gateway answers every request");

            assert_ne!(
                response.status(),
                StatusCode::METHOD_NOT_ALLOWED,
                "{method} {path} is declared in the contract but the gateway does not route it"
            );

            let status = response.status();
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            let body = String::from_utf8_lossy(&bytes);
            assert!(
                !is_no_such_route(&body),
                "{method} {path} is declared in the contract but no service claims it (status {status}): {body}"
            );

            checked += 1;
        }
    }

    // The contract declares twelve storefront path+method pairs and eleven
    // admin ones. The count is asserted so the loop above can never pass by
    // finding nothing to check.
    assert!(
        checked >= 23,
        "expected the contract to declare at least 23 path+method pairs, checked {checked}"
    );
}

#[tokio::test]
async fn the_storefront_twelve_paths_are_all_reachable() {
    // The twelve paths of the storefront section, spelled out. This is the
    // list the Dart client can call, and the reason the gateway exists.
    let storefront = [
        ("GET", "/v1/storefront"),
        ("GET", "/v1/products"),
        ("GET", "/v1/products/p-bra-01"),
        ("GET", "/v1/carts/cart-000001"),
        ("PATCH", "/v1/carts/cart-000001/items/sku-bra-01"),
        ("POST", "/v1/checkouts"),
        ("GET", "/v1/checkouts/chk-000001"),
        ("GET", "/v1/orders"),
        ("GET", "/v1/orders/ord-000001"),
        ("POST", "/v1/orders/ord-000001/reconcile"),
        ("POST", "/v1/sessions/line"),
        ("POST", "/v1/sessions/guest"),
    ];

    for (method, path) in storefront {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header("x-session-id", "sess-000001")
            .header("idempotency-key", "11111111-1111-1111-1111-111111111111")
            .header("x-channel", "web")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .expect("the request builds");

        let response = gateway()
            .oneshot(request)
            .await
            .expect("the gateway answers every request");

        assert_ne!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "{method} {path} is a storefront contract path but the gateway does not route it"
        );

        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8_lossy(&bytes);
        assert!(
            !is_no_such_route(&body),
            "{method} {path} is a storefront contract path but no service claims it (status {status}): {body}"
        );
    }
}

#[tokio::test]
async fn the_bff_origin_knows_every_storefront_path() {
    // The BFF is the public origin in the containerized deployment, so it has
    // to know the path of every contract path it does not serve itself. This
    // checks the routing decision without a network: `base_url_for` is the
    // function the forwarding fallback consults.
    let upstreams = commerce_bff::Upstreams::from_env();
    for (_, path) in [
        ("GET", "/v1/products"),
        ("GET", "/v1/products/p-bra-01"),
        ("GET", "/v1/carts/cart-000001"),
        ("PATCH", "/v1/carts/cart-000001/items/sku-bra-01"),
        ("POST", "/v1/checkouts"),
        ("GET", "/v1/checkouts/chk-000001"),
        ("GET", "/v1/orders"),
        ("GET", "/v1/orders/ord-000001"),
        ("POST", "/v1/orders/ord-000001/reconcile"),
        ("POST", "/v1/sessions/line"),
        ("POST", "/v1/sessions/guest"),
    ] {
        assert!(
            upstreams.base_url_for(path).is_some(),
            "{path} is a contract path and the BFF must forward it to its owner"
        );
    }

    // The BFF's own paths are not forwarded to anything.
    assert!(upstreams.base_url_for("/v1/storefront").is_none());
    assert!(upstreams.base_url_for("/v1/content").is_none());
}
