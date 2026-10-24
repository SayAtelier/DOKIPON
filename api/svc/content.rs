//! Vercel function entry point: the content service.
//!
//! Vercel requires the entry under `api/` at the deployment root, and the
//! deployment root has to be the repository root because the function
//! compiles the Rust workspace in `services/crates/`. The implementation
//! lives in `services/deploy/vercel/src/`.

#[path = "../../services/deploy/vercel/src/svc/content.rs"]
mod content;

fn main() {
    if let Err(error) = content::run() {
        eprintln!("dokipon content function: {error}");
        std::process::exit(1);
    }
}
