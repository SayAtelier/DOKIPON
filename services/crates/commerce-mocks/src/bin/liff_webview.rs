//! mock-liff-webview — serves the app under a chosen LINE host profile.
//!
//! Serves CASE-E2E-002/003/004: the per-WebView degradation matrix.
//!
//! [证据等级: Observed] for this mock's own behaviour.
//! [Hypothetical] for anything about the real LINE WebView.

use commerce_mocks::control_router;

#[tokio::main]
async fn main() {
    let app = commerce_mocks::liff_webview::router();
    commerce_mocks::serve_forever(app, 9005, "mock-liff-webview").await;
}
