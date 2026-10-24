//! The payment adapter Function.
//!
//! It serves `/v1/payments`: authorize, reconcile and the PSP webhook.

#[path = "../serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    // The payment adapter verifies the PSP's webhook signature with a secret.
    // A Function that kept the development secret would accept a webhook
    // signed by anyone, so the Function refuses to start without a real one.
    commerce_edge::secrets::require_for(commerce_edge::secrets::Service::Payment)
        .map_err(serve::config_error)?;
    serve::function(commerce_edge::msa::service::payment())
}
