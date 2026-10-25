//! Vercel function entry point: the identity service.
//!
//! Vercel requires the entry under `api/` at the deployment root, and the
//! deployment root has to be the repository root because the function
//! compiles the Rust workspace in `services/crates/`. The implementation
//! lives in `services/deploy/vercel/src/`.

#[path = "../../services/deploy/vercel/src/svc/identity.rs"]
mod identity;

#[tokio::main]
async fn main() {
    if let Err(error) = identity::run().await {
        eprintln!("dokipon identity function: {error}");
        std::process::exit(1);
    }
}
