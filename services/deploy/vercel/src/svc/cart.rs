//! The cart Function.
//!
//! It serves `/v1/carts`. It prices every line from the catalogue Function
//! over HTTP, never from the client's claim.

#[path = "../serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    serve::function(commerce_edge::msa::service::cart())
}
