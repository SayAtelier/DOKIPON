//! mock-postgres — a constraint-enforcing store, not a database.
//!
//! Serves CASE-IT-001 (DDL constraints) and CASE-CHAOS-003 (connection loss).
//!
//! [证据等级: Observed] for this mock's own behaviour.
//! [Hypothetical] for anything about the real PostgreSQL.

use commerce_mocks::control_router;

#[tokio::main]
async fn main() {
    let db = commerce_mocks::postgres::DbMock::new();
    let app = control_router(db.faults()).merge(commerce_mocks::postgres::router(db));
    commerce_mocks::serve_forever(app, 9003, "mock-postgres").await;
}
