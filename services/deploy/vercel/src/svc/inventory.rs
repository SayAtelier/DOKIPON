//! The inventory Function.
//!
//! It serves `/v1/inventory`: the stock levels and the reservation ledger.

#[path = "../serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    // The ledger lives in the database when the deployment has one: the stock
    // levels the operator console shows have to be the same on every
    // instance, and a reservation taken on one instance has to be visible to
    // the next — otherwise the units can be promised twice.
    //
    // A deployment that has a database and cannot reach it does not start: a
    // store that quietly kept its state in memory while the database was
    // unreachable would look healthy and lose the reservation anyway.
    //
    // The connection is made inside the runtime that serves the Function.
    // Connecting is asynchronous, and a runtime of its own inside another
    // runtime does not work — `block_on` cannot block a thread that is
    // already driving an executor, and tokio panics when it is asked to.
    serve::function_from(|| async {
        let db = commerce_edge::db::connect_from_env()
            .await
            .map_err(serve::config_error)?;
        if let Some(pool) = &db {
            tracing::info!("the inventory function persists its ledger in the database");
            // The demo stock levels go in first, so the SKUs the catalogue
            // knows about exist in the ledger. A SKU that is already there
            // keeps its level: the reserved units of a live reservation must
            // not be overwritten by a boot.
            let repo = commerce_inventory::DbInventoryRepository::new(pool.clone());
            repo.seed(commerce_inventory::DEMO_STOCK_SEED)
                .await
                .map_err(|error| serve::config_error(error.to_string()))?;
        }
        Ok(commerce_edge::msa::service::inventory(db))
    })
}
