//! The identity Function.
//!
//! It serves `/v1/sessions`: the guest session and the LINE id token
//! exchange.

#[path = "../serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    serve::function(commerce_edge::msa::service::identity())
}
