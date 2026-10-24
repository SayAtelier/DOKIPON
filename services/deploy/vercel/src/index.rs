//! The single-function fallback: the whole contract in one Function.
//!
//! This is the topology the deployment used before the services were split
//! into Functions of their own, and it is what the deployment falls back to
//! if the multi-function topology cannot be served. It mounts every service
//! router in one process and dispatches between them in memory, so it still
//! answers every path in the contract.
//!
//! The contract-coverage tests drive this assembly in-process.

#[path = "serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    // This Function serves every service in one process, so it needs every
    // service's credentials. A Function that kept a development credential
    // would authenticate nobody it should not, so it refuses to start.
    commerce_edge::secrets::require_for(commerce_edge::secrets::Service::All)
        .map_err(serve::config_error)?;

    // The admin facade reads its credentials and its upstream URLs from the
    // environment. On Vercel those are the project's environment variables.
    let admin = commerce_admin::AdminState::from_env().map_err(serve::config_error)?;

    let (edge, _services) = commerce_edge::build(admin);
    serve::function(edge)
}
