//! Vercel function entry point: the inventory service.
//!
//! Vercel requires the entry under `api/` at the deployment root, and the
//! deployment root has to be the repository root because the function
//! compiles the Rust workspace in `services/crates/`. The implementation
//! lives in `services/deploy/vercel/src/`.

#[path = "../../services/deploy/vercel/src/svc/inventory.rs"]
mod inventory;

fn main() {
    if let Err(error) = inventory::run() {
        eprintln!("dokipon inventory function: {error}");
        std::process::exit(1);
    }
}
