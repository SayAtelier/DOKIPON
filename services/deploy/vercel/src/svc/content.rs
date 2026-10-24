//! The content Function.
//!
//! It serves `/v1/content`: the editable content document the storefront
//! renders and the operator console edits.

#[path = "../serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    serve::function(commerce_edge::msa::service::content())
}
