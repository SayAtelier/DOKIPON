//! The order Function.
//!
//! It serves `/v1/checkouts` and `/v1/orders`.

#[path = "../serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    // The orders live in the database when the deployment has one: an order
    // created on one instance of this Function has to be visible to the next,
    // or the shopper's own order list is empty on the instance that answers.
    //
    // A deployment that has a database and cannot reach it does not start: a
    // store that quietly kept its state in memory while the database was
    // unreachable would look healthy and lose the order anyway.
    //
    // The connection is made inside the runtime that serves the Function.
    // Connecting is asynchronous, and a runtime of its own inside another
    // runtime does not work — `block_on` cannot block a thread that is
    // already driving an executor, and tokio panics when it is asked to.
    serve::function_from(|| async {
        let db = commerce_edge::db::connect_from_env()
            .await
            .map_err(serve::config_error)?;
        if db.is_some() {
            tracing::info!("the order function persists its orders in the database");
        }
        Ok(commerce_edge::msa::service::order(db))
    })
}
