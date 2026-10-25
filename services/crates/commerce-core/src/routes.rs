//! The public path table: which service owns which public path.
//!
//! ## Why this table lives in the core
//!
//! There are two ways to put the commerce services behind one origin, and both
//! are in this repository:
//!
//! - the **edge** (`commerce-edge`) mounts every service router in one process
//!   and dispatches a request to the service that owns its path;
//! - the **BFF** (`commerce-bff`) is the public origin when the services run as
//!   separate containers (the k3s deployment), and forwards a request it does
//!   not own to the service that does.
//!
//! Both need the same answer to the same question — "who owns `/v1/carts/x`?"
//! — and the answers must be the same answer. Two hand-written tables drift:
//! one of them starts forwarding a path to the wrong service, or stops
//! forwarding it at all, and the symptom is a 404 that only appears in the
//! deployed topology. So the table is written once, here, and both read it.
//!
//! The table is derived from `contracts/openapi.yaml`, which is the contract
//! of record. A path that is not in the contract and not in this table has no
//! owner, and the answer is "404", not "ask whichever service is last".

/// A service that can own a public path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceId {
    /// The edge itself: the storefront boot config and the content document.
    Bff,
    Catalog,
    Cart,
    Order,
    Content,
    Identity,
    Inventory,
    Payment,
    Admin,
}

/// The public path prefixes and the service that owns them.
///
/// A path belongs to the entry when it *is* the prefix or starts with
/// `prefix + "/"`. `/v1/orders` therefore owns `/v1/orders` and
/// `/v1/orders/ord-000001`, but not `/v1/ordersx`.
pub const PATH_TABLE: &[(&str, ServiceId)] = &[
    ("/v1/storefront", ServiceId::Bff),
    ("/v1/products", ServiceId::Catalog),
    ("/v1/carts", ServiceId::Cart),
    ("/v1/checkouts", ServiceId::Order),
    ("/v1/orders", ServiceId::Order),
    ("/v1/content", ServiceId::Content),
    ("/v1/sessions", ServiceId::Identity),
    ("/v1/inventory", ServiceId::Inventory),
    ("/v1/payments", ServiceId::Payment),
    ("/admin/v1", ServiceId::Admin),
];

/// The service that owns `path`, or `None` when no service does.
///
/// A path that matches nothing is a 404 at the edge, not a pass-through to
/// the last service in the table: a typo in a public path must not be
/// answered by whichever service happens to be last.
pub fn owner_of(path: &str) -> Option<ServiceId> {
    PATH_TABLE
        .iter()
        .find(|(prefix, _)| path == *prefix || path.starts_with(&format!("{prefix}/")))
        .map(|(_, id)| *id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_contract_path_has_an_owner() {
        // The twelve storefront paths of `contracts/openapi.yaml`, plus the
        // admin facade's prefix.
        let contract_paths = [
            "/v1/storefront",
            "/v1/products",
            "/v1/products/p-bra-01",
            "/v1/carts/cart-000001",
            "/v1/carts/cart-000001/items/sku-bra-01",
            "/v1/checkouts",
            "/v1/checkouts/chk-000001",
            "/v1/orders",
            "/v1/orders/ord-000001",
            "/v1/orders/ord-000001/reconcile",
            "/v1/sessions/line",
            "/v1/sessions/guest",
        ];
        for path in contract_paths {
            assert!(
                owner_of(path).is_some(),
                "{path} is a contract path and must have an owner"
            );
        }
    }

    #[test]
    fn a_path_is_owned_by_the_longest_matching_prefix() {
        assert_eq!(owner_of("/v1/products"), Some(ServiceId::Catalog));
        assert_eq!(owner_of("/v1/products/p-bra-01"), Some(ServiceId::Catalog));
        assert_eq!(owner_of("/v1/carts/c-1/items/sku-1"), Some(ServiceId::Cart));
        assert_eq!(
            owner_of("/v1/orders/ord-1/reconcile"),
            Some(ServiceId::Order)
        );
        assert_eq!(owner_of("/v1/sessions/line"), Some(ServiceId::Identity));
        assert_eq!(owner_of("/admin/v1/orders"), Some(ServiceId::Admin));
    }

    #[test]
    fn a_prefix_does_not_own_a_longer_word() {
        // `/v1/ordersx` is not `/v1/orders`.
        assert_eq!(owner_of("/v1/ordersx"), None);
        assert_eq!(owner_of("/v1/productsx"), None);
    }

    #[test]
    fn an_unknown_path_has_no_owner() {
        assert_eq!(owner_of("/v1/nope"), None);
        assert_eq!(owner_of("/"), None);
        assert_eq!(owner_of("/healthz"), None);
    }
}
