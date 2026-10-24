//! Vercel function entry point: the admin facade service.
//!
//! Vercel requires the entry under `api/` at the deployment root, and the
//! deployment root has to be the repository root because the function
//! compiles the Rust workspace in `services/crates/`. The implementation
//! lives in `services/deploy/vercel/src/`.

#[path = "../../services/deploy/vercel/src/svc/admin.rs"]
mod admin;

fn main() {
    if let Err(error) = admin::run() {
        eprintln!("dokipon admin function: {error}");
        std::process::exit(1);
    }
}
