//! mock-psp — a payment provider that can take the money and lose the answer.
//!
//! Serves CASE-CHAOS-001: what does checkout do when the PSP is unreachable?
//!
//! [证据等级: Observed] for this mock's own behaviour.
//! [Hypothetical] for anything about the real a payment provider.

use commerce_mocks::control_router;

#[tokio::main]
async fn main() {
    let mock = commerce_mocks::psp::PspMock::new();
    let app = control_router(mock.faults()).merge(commerce_mocks::psp::router(mock));
    commerce_mocks::serve_forever(app, 9001, "mock-psp").await;
}
