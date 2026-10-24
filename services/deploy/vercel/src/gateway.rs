//! The gateway Function.
//!
//! The gateway is the public origin of the function topology, the way the
//! Envoy gateway is the public origin of the cluster topology. It answers the
//! paths it owns itself and forwards the rest to the owning service's
//! Function, over HTTP.
//!
//! It owns no state: the services keep their state in their own Functions,
//! exactly as they keep it in their own containers in the cluster topology.

#[path = "serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    let gateway = commerce_edge::msa::gateway::Gateway::from_env();
    serve::function(commerce_edge::msa::gateway::router(gateway))
}
