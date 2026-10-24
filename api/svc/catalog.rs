//! Vercel function entry point: the catalogue service.
//!
//! Vercel requires the entry under `api/` at the deployment root, and the
//! deployment root has to be the repository root because the function
//! compiles the Rust workspace in `services/crates/`. The implementation
//! lives in `services/deploy/vercel/src/`.

#[path = "../../services/deploy/vercel/src/svc/catalog.rs"]
mod catalog;

fn main() {
    if let Err(error) = catalog::run() {
        eprintln!("dokipon catalog function: {error}");
        std::process::exit(1);
    }
}
