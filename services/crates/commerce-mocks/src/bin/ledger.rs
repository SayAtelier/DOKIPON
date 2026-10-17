//! mock-ledger — an order ledger you can snapshot and restore.
//!
//! Serves CASE-OPS-001: is the trail recoverable, and is a partial restore reported?
//!
//! [证据等级: Observed] for this mock's own behaviour.
//! [Hypothetical] for anything about the real order store.

use commerce_mocks::control_router;

#[tokio::main]
async fn main() {
    let ledger = commerce_mocks::ledger::LedgerMock::new();
    ledger.seed_demo().await;
    let app = commerce_mocks::ledger::router(ledger);
    commerce_mocks::serve_forever(app, 9006, "mock-ledger").await;
}
