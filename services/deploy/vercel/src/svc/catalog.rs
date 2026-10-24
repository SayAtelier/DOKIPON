//! The catalogue Function.
//!
//! It serves `/v1/products`: the product list and the product detail. Its
//! prices are the authoritative ones — the cart and the order ask this
//! Function for a price when they need one.

#[path = "../serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    serve::function(commerce_edge::msa::service::catalog())
}
