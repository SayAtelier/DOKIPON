//! The gateway's routing table agrees with the contract.
//!
//! ## What this test is for
//!
//! There are two topologies, and they have to agree on one thing: **which
//! service owns which public path**.
//!
//! - the **in-process edge** dispatches a request to the service that owns
//!   its path, in memory;
//! - the **gateway** forwards the same request to the same service, over
//!   HTTP, to that service's Function.
//!
//! Both read the same table (`commerce_core::routes`), so they cannot drift
//! apart by accident. This test is the guard that says so out loud: every
//! path in `contracts/openapi.yaml` — all twenty-three of them — has an
//! owner, the gateway routes it to that owner, and the owner is the service
//! the contract's path prefix names.
//!
//! It reads the contract itself rather than a hand-written list of paths, so
//! a path added to the YAML and not routed fails here.
//!
//! ## No network, no ports
//!
//! The routing table is data, so the test is data too: nothing binds a port
//! and nothing is dialled.

use std::path::PathBuf;

use commerce_edge::msa::routing::{
    owner_of, route_for, service_prefix, Route, ServiceId, GATEWAY_PATH,
};
use serde_yaml::Value;

/// The value substituted for every path parameter.
const PARAM_SAMPLE: &str = "sample-id";

/// The contract of record, read from the repository.
fn contract() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../contracts/openapi.yaml");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_yaml::from_str(&raw).expect("contracts/openapi.yaml must be valid YAML")
}

/// The paths of the contract, as they are written in the YAML.
fn contract_paths() -> Vec<String> {
    let doc = contract();
    let paths = doc["paths"]
        .as_mapping()
        .expect("the contract declares a paths map");
    paths
        .keys()
        .map(|key| {
            key.as_str()
                .expect("a contract path is a string")
                .to_string()
        })
        .collect()
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

/// Every path in the contract, and the service that owns it.
///
/// The ownership is spelled out here rather than derived, so the test says
/// what it means: the storefront's twelve paths belong to the six services
/// that serve them, and the eleven admin paths belong to the admin facade.
const CONTRACT_OWNERS: &[(&str, ServiceId)] = &[
    ("/v1/storefront", ServiceId::Bff),
    ("/v1/products", ServiceId::Catalog),
    ("/v1/products/{productId}", ServiceId::Catalog),
    ("/v1/carts/{cartId}", ServiceId::Cart),
    ("/v1/carts/{cartId}/items/{sku}", ServiceId::Cart),
    ("/v1/checkouts", ServiceId::Order),
    ("/v1/checkouts/{checkoutId}", ServiceId::Order),
    ("/v1/orders", ServiceId::Order),
    ("/v1/orders/{orderId}", ServiceId::Order),
    ("/v1/orders/{orderId}/reconcile", ServiceId::Order),
    ("/v1/sessions/line", ServiceId::Identity),
    ("/v1/sessions/guest", ServiceId::Identity),
    ("/admin/v1/sessions", ServiceId::Admin),
    ("/admin/v1/session", ServiceId::Admin),
    ("/admin/v1/orders", ServiceId::Admin),
    ("/admin/v1/orders/{orderId}", ServiceId::Admin),
    ("/admin/v1/orders/{orderId}/reconcile", ServiceId::Admin),
    ("/admin/v1/products", ServiceId::Admin),
    ("/admin/v1/products/{productId}", ServiceId::Admin),
    ("/admin/v1/inventory", ServiceId::Admin),
    ("/admin/v1/inventory/reservations", ServiceId::Admin),
    (
        "/admin/v1/inventory/reservations/{reservationId}/release",
        ServiceId::Admin,
    ),
    ("/admin/v1/payments", ServiceId::Admin),
];

/// The Function each service is served by.
const FUNCTION_PATHS: &[(ServiceId, &str)] = &[
    (ServiceId::Catalog, "/api/svc/catalog"),
    (ServiceId::Cart, "/api/svc/cart"),
    (ServiceId::Order, "/api/svc/order"),
    (ServiceId::Content, "/api/svc/content"),
    (ServiceId::Identity, "/api/svc/identity"),
    (ServiceId::Inventory, "/api/svc/inventory"),
    (ServiceId::Payment, "/api/svc/payment"),
    (ServiceId::Admin, "/api/svc/admin"),
];

#[test]
fn the_contract_declares_the_paths_this_test_knows() {
    let paths = contract_paths();
    assert_eq!(
        paths.len(),
        23,
        "the contract declares 23 paths, the owner list must match it: {paths:?}"
    );
    for path in &paths {
        assert!(
            CONTRACT_OWNERS.iter().any(|(known, _)| known == path),
            "{path} is in the contract but not in the owner list"
        );
    }
    for (path, _) in CONTRACT_OWNERS {
        assert!(
            paths.iter().any(|known| known == path),
            "{path} is in the owner list but not in the contract"
        );
    }
}

#[test]
fn every_contract_path_is_owned_by_the_service_the_contract_names() {
    for (path, expected) in CONTRACT_OWNERS {
        let concrete_path = concrete(path);
        assert_eq!(
            owner_of(&concrete_path),
            Some(*expected),
            "{path} is owned by {expected:?} in the shared path table"
        );

        // And the gateway's decision is that owner: the BFF's own paths are
        // the gateway's, every other path is forwarded to its owner.
        match route_for(&concrete_path) {
            Route::Local => assert_eq!(
                *expected,
                ServiceId::Bff,
                "{path} is answered by the gateway, so the BFF must own it"
            ),
            Route::Forward(id) => assert_eq!(
                id, *expected,
                "{path} is forwarded to {id:?}, but {expected:?} owns it"
            ),
            Route::Unknown => panic!("{path} is a contract path but the gateway routes it nowhere"),
        }
    }
}

#[test]
fn every_service_is_served_by_the_function_it_is_deployed_on() {
    for (id, path) in FUNCTION_PATHS {
        assert_eq!(service_prefix(*id), *path, "{id:?} is served by {path}");
        assert!(
            path.starts_with("/api/svc/"),
            "{path} is not under /api/svc/"
        );
    }
}

#[test]
fn the_gateway_never_calls_itself() {
    // The gateway's own route is /api/gateway. Every service is served by a
    // Function under /api/svc/, so no service's base URL can be the
    // gateway's own path — the gateway cannot forward a request to itself.
    for (id, path) in FUNCTION_PATHS {
        assert_ne!(
            *path, GATEWAY_PATH,
            "{id:?} is served by the gateway itself"
        );
        assert!(
            !path.starts_with(GATEWAY_PATH),
            "{id:?} is served under the gateway's own path"
        );
    }
}

#[test]
fn a_path_no_service_owns_is_routed_nowhere() {
    // The gateway must answer "no service handles this path" rather than
    // forward it to whichever service is last.
    assert_eq!(route_for("/v1/nope"), Route::Unknown);
    assert_eq!(route_for("/"), Route::Unknown);
    assert_eq!(route_for("/healthz"), Route::Unknown);
}
