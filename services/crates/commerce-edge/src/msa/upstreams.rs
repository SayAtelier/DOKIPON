//! Where each service's Function lives.
//!
//! ## The base URL is "where the service's routes are rooted"
//!
//! One variable per service, and the same names the cluster deployment and
//! the admin facade already use (`CATALOG_URL`, `ORDER_URL`, …). A base URL is
//! the address the service's own routes hang off, so a caller appends the
//! service's path to it:
//!
//! ```text
//! {CATALOG_URL}/v1/products
//! ```
//!
//! That is why the value carries the Function's mount path in the function
//! topology (`https://<host>/api/svc/catalog`) and does not in the local one
//! (`http://localhost:8082`, where the service's own binary serves `/`).
//!
//! ## The order of preference
//!
//! 1. **the service's own variable** — an explicit address always wins;
//! 2. **the deployment's own origin** — on Vercel every service is a route of
//!    the same deployment, so `VERCEL_URL` plus the service's mount path is
//!    where that service's Function is. This is what makes the function
//!    topology work without a per-service variable per deployment;
//! 3. **the local address** — the port the service's own binary listens on,
//!    for running the services outside Vercel.
//!
//! The cluster deployment's defaults are Docker service names
//! (`http://commerce-catalog:8082`). Those do not resolve on Vercel — the
//! DNS name does not exist there — which is why the defaults here are
//! addresses that do.

use super::routing::{service_prefix, ServiceId};

/// The base URL of every service's Function.
#[derive(Debug, Clone)]
pub struct Upstreams {
    catalog: String,
    cart: String,
    order: String,
    identity: String,
    inventory: String,
    payment: String,
    content: String,
    admin: String,
}

impl Upstreams {
    /// Read the base URLs from the environment.
    pub fn from_env() -> Self {
        Self {
            catalog: base_url("CATALOG_URL", ServiceId::Catalog, "http://localhost:8082"),
            cart: base_url("CART_URL", ServiceId::Cart, "http://localhost:8083"),
            order: base_url("ORDER_URL", ServiceId::Order, "http://localhost:8085"),
            identity: base_url("IDENTITY_URL", ServiceId::Identity, "http://localhost:8081"),
            inventory: base_url("INVENTORY_URL", ServiceId::Inventory, "http://localhost:8084"),
            payment: base_url(
                "PAYMENT_ADAPTER_URL",
                ServiceId::Payment,
                "http://localhost:8086",
            ),
            content: base_url("CONTENT_URL", ServiceId::Content, "http://localhost:8096"),
            admin: base_url("ADMIN_URL", ServiceId::Admin, "http://localhost:8090"),
        }
    }

    /// The base URL of the service's Function.
    ///
    /// `None` for the BFF: it has no Function of its own in this topology —
    /// the gateway *is* the BFF.
    pub fn base_url_for(&self, id: ServiceId) -> Option<&str> {
        let url = match id {
            ServiceId::Bff => return None,
            ServiceId::Catalog => &self.catalog,
            ServiceId::Cart => &self.cart,
            ServiceId::Order => &self.order,
            ServiceId::Identity => &self.identity,
            ServiceId::Inventory => &self.inventory,
            ServiceId::Payment => &self.payment,
            ServiceId::Content => &self.content,
            ServiceId::Admin => &self.admin,
        };
        Some(url.as_str())
    }

    /// Every service points at one address.
    ///
    /// A test needs a gateway whose services are all the same stand-in; the
    /// deployment needs eight different ones.
    #[cfg(test)]
    pub fn all_to(base_url: &str) -> Self {
        let base = base_url.trim_end_matches('/').to_string();
        Self {
            catalog: base.clone(),
            cart: base.clone(),
            order: base.clone(),
            identity: base.clone(),
            inventory: base.clone(),
            payment: base.clone(),
            content: base.clone(),
            admin: base,
        }
    }
}

/// The base URL of one service, from the environment.
///
/// The trailing slash is stripped so a URL can be built by concatenation
/// without producing a double slash.
fn base_url(variable: &str, id: ServiceId, local: &str) -> String {
    if let Ok(value) = std::env::var(variable) {
        let value = normalize(&value);
        if !value.is_empty() {
            return value;
        }
    }

    // On Vercel the deployment's own domain is in the environment, and every
    // service of the deployment is a route of it.
    if let Ok(host) = std::env::var("VERCEL_URL") {
        let host = host.trim();
        if !host.is_empty() {
            return format!("https://{host}{}", service_prefix(id));
        }
    }

    local.to_string()
}

/// A base URL, cleaned up: no surrounding space, no trailing slash.
///
/// The trailing slash is stripped so a URL can be built by concatenation
/// without producing a double slash.
fn normalize(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_service_has_a_base_url_except_the_bff() {
        let upstreams = Upstreams::from_env();
        assert!(upstreams.base_url_for(ServiceId::Bff).is_none());
        for id in [
            ServiceId::Catalog,
            ServiceId::Cart,
            ServiceId::Order,
            ServiceId::Identity,
            ServiceId::Inventory,
            ServiceId::Payment,
            ServiceId::Content,
            ServiceId::Admin,
        ] {
            let url = upstreams
                .base_url_for(id)
                .unwrap_or_else(|| panic!("{id:?} must have a base url"));
            assert!(!url.is_empty(), "{id:?} has an empty base url");
            assert!(!url.ends_with('/'), "{id:?} has a trailing slash: {url}");
        }
    }

    #[test]
    fn a_base_url_never_ends_in_a_slash() {
        // The trailing slash is stripped so `{base}{path}` cannot produce
        // `//v1/products`.
        assert_eq!(normalize("http://localhost:8082/"), "http://localhost:8082");
        assert_eq!(normalize("  http://localhost:8082/  "), "http://localhost:8082");
        assert_eq!(normalize("http://localhost:8082"), "http://localhost:8082");
    }
}
