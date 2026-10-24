//! The inventory Function.
//!
//! It serves `/v1/inventory`: the stock levels it holds and the reservation
//! ledger the checkout reserves against.

#[path = "../serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    serve::function(commerce_edge::msa::service::inventory())
}
