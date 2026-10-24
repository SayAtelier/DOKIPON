//! The payment adapter Function.
//!
//! It serves `/v1/payments`: authorize, reconcile and the PSP webhook.

#[path = "../serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    serve::function(commerce_edge::msa::service::payment())
}
