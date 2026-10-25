//! The DOKIPON edge as a standalone binary.
//!
//! Serves every commerce service behind one router on one port. This is the
//! local and container deployment; the Vercel deployment uses
//! `src/bin/vercel.rs` instead, which wraps the same router in the Vercel
//! runtime's bridge.

use commerce_edge::build;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // This process serves every service, so it needs every service's
    // credentials. A deployment that keeps a development credential would
    // authenticate nobody it should not, so it refuses to start.
    commerce_edge::secrets::require_for(commerce_edge::secrets::Service::All)
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;

    let admin = commerce_admin::AdminState::from_env()
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);

    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind edge");

    tracing::info!(
        "commerce edge listening on {}",
        listener.local_addr().unwrap()
    );

    let (edge, _services) = build(admin);
    axum::serve(listener, edge).await?;
    Ok(())
}
