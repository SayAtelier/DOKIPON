//! Vercel function entry point: the identity service.
//!
//! Vercel requires the entry under `api/` at the deployment root, and the
//! deployment root has to be the repository root because the function
//! compiles the Rust workspace in `services/crates/`. The implementation
//! lives in `services/deploy/vercel/src/`.
//!
//! The entry point is synchronous on purpose. The runtime belongs to
//! `services/deploy/vercel/src/serve.rs`, and an entry point that made a
//! runtime of its own would start one inside the other — which tokio refuses
//! to do.

#[path = "../../services/deploy/vercel/src/svc/identity.rs"]
mod identity;

fn main() {
    if let Err(error) = identity::run() {
        eprintln!("dokipon identity function: {error}");
        std::process::exit(1);
    }
}
