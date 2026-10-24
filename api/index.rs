//! Vercel function entry point: the single-function fallback.
//!
//! Vercel requires the entry under `api/` at the deployment root, and the
//! deployment root has to be the repository root because the function
//! compiles the Rust workspace in `services/crates/`. The implementation
//! lives in `services/deploy/vercel/src/`.

#[path = "../services/deploy/vercel/src/index.rs"]
mod index;

fn main() {
    if let Err(error) = index::run() {
        eprintln!("dokipon edge function: {error}");
        std::process::exit(1);
    }
}
