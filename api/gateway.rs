//! Vercel function entry point: the gateway.
//!
//! Vercel requires the entry under `api/` at the deployment root, and the
//! deployment root has to be the repository root because the function
//! compiles the Rust workspace in `services/crates/`. The implementation
//! lives in `services/deploy/vercel/src/`.

#[path = "../services/deploy/vercel/src/gateway.rs"]
mod gateway;

fn main() {
    if let Err(error) = gateway::run() {
        eprintln!("dokipon gateway function: {error}");
        std::process::exit(1);
    }
}
