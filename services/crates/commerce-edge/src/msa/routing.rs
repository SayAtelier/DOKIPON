//! Which service owns which public path, and where that service lives.
//!
//! ## One table, two topologies
//!
//! The public path table is written once, in
//! [`commerce_core::routes`], and both topologies read it:
//!
//! - the **in-process edge** dispatches a request to the service that owns
//!   its path, in memory;
//! - the **gateway** forwards the same request to the same service, over
//!   HTTP, to that service's Function.
//!
//! A path that is served in one topology and not in the other is a 404 that
//! only the deployed topology shows, so the table is not written twice.
//!
//! ## The function paths
//!
//! Each service's Function is mounted under a path of its own, and the
//! service's own routes are rooted at `/`. The Function path is therefore
//! *where the service is*, and the service's routes are *what it serves* —
//! the same split an Envoy route and an upstream container have.

pub use commerce_core::routes::{owner_of, ServiceId};

/// The path each service's Function is mounted under.
///
/// The gateway's own path is `/api/gateway`; it is not in this table because
/// the gateway is not a service — it owns the paths the services own.
pub const SERVICE_PREFIXES: &[(ServiceId, &str)] = &[
    (ServiceId::Bff, "/api/svc/bff"),
    (ServiceId::Catalog, "/api/svc/catalog"),
    (ServiceId::Cart, "/api/svc/cart"),
    (ServiceId::Order, "/api/svc/order"),
    (ServiceId::Content, "/api/svc/content"),
    (ServiceId::Identity, "/api/svc/identity"),
    (ServiceId::Inventory, "/api/svc/inventory"),
    (ServiceId::Payment, "/api/svc/payment"),
    (ServiceId::Admin, "/api/svc/admin"),
];

/// The gateway's own path.
///
/// The gateway must never call itself, so this path is not a service prefix
/// and no service's base URL is derived from it.
pub const GATEWAY_PATH: &str = "/api/gateway";

/// The path of the Function that serves `id`.
pub fn service_prefix(id: ServiceId) -> &'static str {
    SERVICE_PREFIXES
        .iter()
        .find(|(candidate, _)| *candidate == id)
        .map(|(_, prefix)| *prefix)
        .expect("every service has a function path")
}

/// What the gateway does with an incoming path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// The gateway answers it itself.
    ///
    /// The BFF's own paths are the gateway's: the gateway replaces the BFF
    /// Deployment of the cluster topology, so the paths the BFF served are
    /// served here.
    Local,
    /// The named service owns it, and the gateway forwards it to that
    /// service's Function.
    Forward(ServiceId),
    /// No service owns it.
    Unknown,
}

/// The gateway's decision for `path`.
///
/// The decision is the public path table's, plus one rule: the paths the BFF
/// owns are the gateway's own, because the gateway *is* the BFF in this
/// topology.
pub fn route_for(path: &str) -> Route {
    match owner_of(path) {
        Some(ServiceId::Bff) => Route::Local,
        Some(id) => Route::Forward(id),
        None => Route::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_service_has_a_function_path_and_the_paths_are_distinct() {
        let mut seen: Vec<&str> = Vec::new();
        for (id, prefix) in SERVICE_PREFIXES {
            assert!(!seen.contains(prefix), "{prefix} is used twice");
            seen.push(prefix);
            assert!(
                prefix.starts_with("/api/svc/"),
                "{prefix} is not under /api/svc/"
            );
            assert_eq!(service_prefix(*id), *prefix);
        }
    }

    #[test]
    fn the_gateway_never_calls_itself() {
        // Every service's Function is under /api/svc/, and the gateway's own
        // route is /api/gateway. A service base URL can therefore never be
        // the gateway's own path.
        for (_, prefix) in SERVICE_PREFIXES {
            assert!(!prefix.starts_with(GATEWAY_PATH));
            assert_ne!(*prefix, GATEWAY_PATH);
        }
    }

    #[test]
    fn the_bffs_own_paths_are_the_gateways_own() {
        assert_eq!(route_for("/v1/storefront"), Route::Local);
    }

    #[test]
    fn every_other_service_path_is_forwarded_to_its_owner() {
        assert_eq!(
            route_for("/v1/products"),
            Route::Forward(ServiceId::Catalog)
        );
        assert_eq!(route_for("/v1/carts/c-1"), Route::Forward(ServiceId::Cart));
        assert_eq!(route_for("/v1/orders"), Route::Forward(ServiceId::Order));
        assert_eq!(route_for("/v1/content"), Route::Forward(ServiceId::Content));
        assert_eq!(
            route_for("/v1/sessions/guest"),
            Route::Forward(ServiceId::Identity)
        );
        assert_eq!(
            route_for("/v1/inventory"),
            Route::Forward(ServiceId::Inventory)
        );
        assert_eq!(
            route_for("/v1/payments/authorize"),
            Route::Forward(ServiceId::Payment)
        );
        assert_eq!(
            route_for("/admin/v1/orders"),
            Route::Forward(ServiceId::Admin)
        );
    }

    #[test]
    fn a_path_no_service_owns_is_unknown() {
        assert_eq!(route_for("/v1/nope"), Route::Unknown);
        assert_eq!(route_for("/"), Route::Unknown);
    }
}
