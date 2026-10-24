//! The admin facade Function.
//!
//! It serves `/admin/v1`: the operator console's API. The facade
//! authenticates the operator itself and proxies the reads and writes to the
//! services that own the data.

#[path = "../serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    let router = commerce_edge::msa::service::admin().map_err(
        |message| -> vercel_runtime::Error {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                message,
            ))
        },
    )?;
    serve::function(router)
}
