//! The admin facade Function.
//!
//! It serves `/admin/v1`: the operator console's API. The facade
//! authenticates the operator itself and proxies the reads and writes to the
//! services that own the data.

#[path = "../serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    // The operator console's facade authenticates the operator with a
    // password. A Function that kept the development password would let
    // anyone into the console, so the Function refuses to start without a
    // real one.
    commerce_edge::secrets::require_for(commerce_edge::secrets::Service::Admin)
        .map_err(serve::config_error)?;
    let router = commerce_edge::msa::service::admin().map_err(serve::config_error)?;
    serve::function(router)
}
