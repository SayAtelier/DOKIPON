//! Vercel function entry point: the payment adapter service.
//!
//! Vercel requires the entry under `api/` at the deployment root, and the
//! deployment root has to be the repository root because the function
//! compiles the Rust workspace in `services/crates/`. The implementation
//! lives in `services/deploy/vercel/src/`.

#[path = "../../services/deploy/vercel/src/svc/payment.rs"]
mod payment;

fn main() {
    if let Err(error) = payment::run() {
        eprintln!("dokipon payment function: {error}");
        std::process::exit(1);
    }
}
