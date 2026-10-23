//! Standalone binary for the commerce-content service.
//!
//! The service's logic lives in `lib.rs` so that the merged `commerce-edge`
//! deployment — one Vercel function serving every service — can mount the
//! same router alongside the others. This binary is the local and k3s
//! deployment: it binds a port and serves the router on its own.

use commerce_content::*;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    // Where the document is persisted. Unset means memory only, which is
    // fine for a container that is recreated on every deploy with the
    // document supplied as a ConfigMap.
    let path = std::env::var("CONTENT_PATH")
        .ok()
        .map(PathBuf::from);
    let store: SharedContent = Arc::new(RwLock::new(ContentStore::new(path.clone())));
    let app = router(store);
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("bind content");
    tracing::info!(
        "content service listening on {}, persisting to {}",
        listener.local_addr().unwrap(),
        path.map(|p| p.display().to_string())
            .unwrap_or_else(|| "(memory only)".to_string())
    );
    axum::serve(listener, app).await.expect("serve content");
}
