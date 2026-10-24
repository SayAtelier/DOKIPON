//! The commerce services' stores, built the same way in every topology.
//!
//! ## Why the builders live here
//!
//! There are two ways to run the commerce services, and both of them need the
//! same stores:
//!
//! - the **in-process edge** (`commerce_edge::build`) mounts every service
//!   router in one process and dispatches between them in memory;
//! - the **function topology** (`commerce_edge::msa`) gives each service its
//!   own Vercel Function, and the services reach each other over HTTP.
//!
//! The seeds are not decoration. The inventory ledger is seeded with the
//! catalogue's *reserved* SKUs, and the checkout path looks stock up by SKU: a
//! SKU that exists in the catalogue but not in the ledger makes a purchasable
//! product fail at the last step, and a SKU that exists in the ledger but not
//! in the catalogue cannot be priced at all. When the two topologies built
//! their stores separately, that was exactly the kind of drift that only shows
//! up in the deployed topology.
//!
//! So the seeds are written once, here, and both topologies ask for them.

use std::sync::{Arc, RwLock};

/// The catalogue. Immutable after seeding, so no lock is needed to read it.
pub fn catalog() -> Arc<commerce_catalog::CatalogStore> {
    Arc::new(commerce_catalog::CatalogStore::seed_demo())
}

/// The cart store.
pub fn cart() -> Arc<RwLock<commerce_cart::CartStore>> {
    Arc::new(RwLock::new(commerce_cart::CartStore::new()))
}

/// The order store.
pub fn order() -> Arc<RwLock<commerce_order::OrderStore>> {
    Arc::new(RwLock::new(commerce_order::OrderStore::new(
        commerce_core::idempotency::now_ms(),
    )))
}

/// The inventory ledger, seeded with the catalogue's reserved SKUs.
///
/// These SKUs must match the catalogue's seed: the checkout path looks stock
/// up by sku, and a sku that exists in one service but not the other makes a
/// purchasable product fail at the last step.
///
/// Only the `Reserved` skus are here. `Unbounded` and `MadeToOrder` need no
/// stock (`StockPolicy::needs_inventory`), which is why `sku-cami-01` is
/// absent.
pub fn inventory() -> Arc<RwLock<commerce_inventory::InventoryStore>> {
    Arc::new(RwLock::new(commerce_inventory::InventoryStore::new(&[
        ("sku-bra-01", 42),
        ("sku-brief-01", 7),
        ("sku-set-01", 0),
        ("sku-night-01", 28),
        ("sku-adult-01", 3),
        ("sku-swim-01", 12),
        ("sku-sale-01", 5),
    ])))
}

/// The identity store: sessions and the nonce replay guard.
pub fn identity() -> Arc<RwLock<commerce_identity::IdentityStore>> {
    Arc::new(RwLock::new(commerce_identity::IdentityStore::new(
        commerce_core::idempotency::now_ms(),
    )))
}

/// The payment adapter.
///
/// The webhook secret is the PSP's, read from the environment. The default is
/// a development value: it is not a credential, and a deployment that cares
/// sets the variable. The deployment's entry point refuses to start while
/// the default is still in place — see `commerce_core::secrets` — so the
/// default only ever reaches a local run.
///
/// The adapter reads the clock on every request, so the webhook's freshness
/// window is measured against the current time.
pub fn payment() -> Arc<RwLock<commerce_payment_adapter::PaymentAdapter>> {
    let secret = std::env::var("PSP_WEBHOOK_SECRET").unwrap_or_else(|_| "dev-only-secret".into());
    Arc::new(RwLock::new(commerce_payment_adapter::PaymentAdapter::new(secret)))
}

/// The content store: the editable content document.
pub fn content() -> Arc<RwLock<commerce_content::ContentStore>> {
    Arc::new(RwLock::new(commerce_content::ContentStore::new(
        std::env::var("CONTENT_PATH").ok().map(std::path::PathBuf::from),
    )))
}

/// The verifier the identity service checks LINE id tokens against.
///
/// The signing key and the channel id are read from the environment. The
/// defaults are development values: an id token signed with the development
/// key would be accepted by anyone who reads this repository. The
/// deployment's entry point refuses to start while a default is still in
/// place — see `commerce_core::secrets`.
pub fn identity_verifier() -> Arc<commerce_identity::TokenVerifier> {
    Arc::new(commerce_identity::TokenVerifier {
        expected_audience: std::env::var("LINE_CHANNEL_ID")
            .unwrap_or_else(|_| "dev-channel".into()),
        expected_issuer: "https://access.line.me".to_string(),
        signing_key: std::env::var("LINE_ID_TOKEN_KEY")
            .unwrap_or_else(|_| "dev-only-key".into()),
    })
}
