//! mock-broker — an at-least-once broker you can make misbehave.
//!
//! Serves CASE-IT-002 (redelivery) and CASE-CHAOS-002 (broker outage).
//!
//! [证据等级: Observed] for this mock's own behaviour.
//! [Hypothetical] for anything about the real a message broker.

use commerce_mocks::control_router;

#[tokio::main]
async fn main() {
    let mock = commerce_mocks::broker::BrokerMock::new();
    let app = control_router(mock.faults()).merge(commerce_mocks::broker::router(mock));
    commerce_mocks::serve_forever(app, 9002, "mock-broker").await;
}
