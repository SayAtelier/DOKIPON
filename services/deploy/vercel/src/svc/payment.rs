//! The payment adapter Function.
//!
//! It serves `/v1/payments`: the authorization, the reconciliation and the
//! provider's webhook.

#[path = "../serve.rs"]
mod serve;

pub fn run() -> Result<(), vercel_runtime::Error> {
    // The identity service verifies LINE id tokens with a signing key. A
    // Function that kept the development key would accept an id token signed
    // by anyone, so the Function refuses to start without a real one.
    commerce_edge::secrets::require_for(commerce_edge::secrets::Service::Payment)
        .map_err(serve::config_error)?;

    // The attempts live in the database when the deployment has one: a
    // provider event that arrives at one instance has to be refused at the
    // next — otherwise the same event is accepted twice — and the operator's
    // payments list has to be the same list everywhere.
    //
    // A deployment that has a database and cannot reach it does not start: a
    // store that quietly kept its state in memory while the database was
    // unreachable would look healthy and lose the attempt anyway.
    //
    // The connection is made inside the runtime that serves the Function.
    // Connecting is asynchronous, and a runtime of its own inside another
    // runtime does not work — `block_on` cannot block a thread that is
    // already driving an executor, and tokio panics when it is asked to.
    serve::function_from(|| async {
        let db = commerce_edge::db::connect_from_env()
            .await
            .map_err(serve::config_error)?;
        if db.is_some() {
            tracing::info!("the payment function persists its attempts in the database");
        }
        Ok(commerce_edge::msa::service::payment(db))
    })
}
