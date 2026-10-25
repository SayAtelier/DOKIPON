//! The identity Function.
//!
//! It serves `/v1/sessions`: the guest session and the LINE id token
//! exchange.

#[path = "../serve.rs"]
mod serve;

pub async fn run() -> Result<(), vercel_runtime::Error> {
    // The identity service verifies LINE id tokens with a signing key. A
    // Function that kept the development key would accept an id token signed
    // by anyone, so the Function refuses to start without a real one.
    commerce_edge::secrets::require_for(commerce_edge::secrets::Service::Identity)
        .map_err(serve::config_error)?;
    // The sessions live in the database when the deployment has one. The id
    // comes from a sequence there, so two instances of this Function cannot
    // mint the same session id — two shoppers sharing one session would
    // share one cart — and every instance sees the same session.
    //
    // A deployment that has a database and cannot reach it does not start:
    // a store that quietly kept its state in memory while the database was
    // unreachable would look healthy and lose the record anyway.
    let db = commerce_edge::db::connect_from_env()
        .await
        .map_err(serve::config_error)?;
    if db.is_some() {
        tracing::info!("the identity function persists its sessions in the database");
    }
    serve::function(commerce_edge::msa::service::identity(db))
}
