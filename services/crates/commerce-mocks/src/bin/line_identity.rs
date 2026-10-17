//! mock-line-identity — issues LINE-shaped id_tokens, including invalid ones.
//!
//! Serves CASE-E2E-002/003: what does the server do with each token flavour?
//!
//! [证据等级: Observed] for this mock's own behaviour.
//! [Hypothetical] for anything about the real LINE platform.

use commerce_mocks::control_router;

#[tokio::main]
async fn main() {
    let key = std::env::var("LINE_ID_TOKEN_KEY").unwrap_or_else(|_| "dev-only-key".into());
    let channel = std::env::var("LINE_CHANNEL_ID").unwrap_or_else(|_| "1234567890".into());
    let mock = commerce_mocks::line_identity::LineIdentityMock::new(key, channel);
    let app = commerce_mocks::line_identity::router(mock);
    commerce_mocks::serve_forever(app, 9004, "mock-line-identity").await;
}
