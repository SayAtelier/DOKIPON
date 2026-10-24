//! The identity Function.
//!
//! It serves `/v1/sessions`: the guest session and the LINE id token
//! exchange.

#[path = "../serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    // The identity service verifies LINE id tokens with a signing key. A
    // Function that kept the development key would accept an id token signed
    // by anyone, so the Function refuses to start without a real one.
    commerce_edge::secrets::require_for(commerce_edge::secrets::Service::Identity)
        .map_err(serve::config_error)?;
    serve::function(commerce_edge::msa::service::identity())
}
