//! The order repository, against a real database.
//!
//! The test is skipped when there is no `DATABASE_URL`: the local run has
//! no database on purpose, and a test that needs one must not pretend to
//! have passed when there was nothing to test.
//!
//! In CI there **is** one — the workflow starts a `postgres:16-alpine` and
//! points `DATABASE_URL` at it — so this is where the schema, the id
//! sequence, the idempotency gate and the cross-instance visibility are
//! actually checked.

use commerce_core::db;
use commerce_core::{CheckoutState, Currency};
use commerce_order::{DbOrderRepository, OrderLine, OrderRepository, PaymentOutcome, SubmitStart};

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
}

/// A buyer nobody else is using, so the tests do not see each other's orders.
fn a_buyer() -> String {
    format!(
        "uat-buyer-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

fn a_line() -> OrderLine {
    OrderLine {
        sku: "sku-bra-01".to_string(),
        quantity: 1,
        unit_amount_minor: 8_900,
        currency: Currency::Jpy,
    }
}

/// The whole checkout, the way the handler does it.
///
/// The saga is begun, the stock is reserved — here by the test itself, since
/// there is no inventory service in this test — and then the saga finishes.
async fn submit(
    repo: &DbOrderRepository,
    buyer: &str,
    lines: Vec<OrderLine>,
    key: &str,
    outcome: PaymentOutcome,
) -> Result<(String, CheckoutState, PaymentOutcome), commerce_core::error::AppError> {
    match repo.begin_submit(buyer, lines, key).await? {
        SubmitStart::Replay {
            checkout_id,
            order_id: _,
            state,
        } => Ok((checkout_id, state, outcome)),
        SubmitStart::Begin(begin) => {
            let reservation_id = "res-000001".to_string();
            repo.finish_submit(*begin, reservation_id, outcome).await
        }
    }
}
#[tokio::test]
async fn an_order_survives_the_round_trip() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let repo = DbOrderRepository::new(pool);

    let buyer = a_buyer();
    let (checkout_id, state, _) = submit(
        &repo,
        &buyer,
        vec![a_line()],
        "key-1",
        PaymentOutcome::Authorized,
    )
    .await
    .expect("submit");
    assert_eq!(
        state,
        CheckoutState::Confirmed,
        "an authorized order confirms"
    );

    let order = repo.by_checkout(&checkout_id).await.expect("by checkout");
    assert_eq!(order.buyer_id, buyer);
    assert_eq!(order.state, CheckoutState::Confirmed);
    assert_eq!(order.lines.len(), 1);
    assert_eq!(order.lines[0].sku, "sku-bra-01");
    assert_eq!(order.lines[0].unit_amount_minor, 8_900);
    assert_eq!(order.lines[0].currency, Currency::Jpy);
    assert_eq!(order.total.amount_minor, 8_900);
    assert!(!order.trail.is_empty(), "the order carries its trail");
    assert!(
        !order.outbox.is_empty(),
        "the order carries the events it produced"
    );
    assert!(
        order.order_id.starts_with("ord-"),
        "the id keeps its shape: {}",
        order.order_id
    );
}

#[tokio::test]
async fn two_repositories_share_one_order() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    // Two repositories, standing for two instances of the order Function. The
    // whole reason the orders are in the database: an order created on one
    // instance has to be visible to the next, or the shopper's own order
    // list is empty on the instance that answers.
    let first = DbOrderRepository::new(pool.clone());
    let second = DbOrderRepository::new(pool);

    let buyer = a_buyer();
    let (checkout_id, _, _) = submit(
        &first,
        &buyer,
        vec![a_line()],
        "key-1",
        PaymentOutcome::Authorized,
    )
    .await
    .expect("submit");

    let order = second.by_checkout(&checkout_id).await.expect("by checkout");
    assert_eq!(order.buyer_id, buyer);
    assert_eq!(order.lines.len(), 1);

    let listed = second.list_for(&buyer).await.expect("list");
    assert_eq!(listed.len(), 1, "the other instance lists the order too");
}

#[tokio::test]
async fn the_same_key_replays_the_same_answer() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let repo = DbOrderRepository::new(pool);

    let buyer = a_buyer();
    let (first_checkout, first_state, _) = submit(
        &repo,
        &buyer,
        vec![a_line()],
        "key-replay",
        PaymentOutcome::Authorized,
    )
    .await
    .expect("submit");
    let (second_checkout, second_state, _) = submit(
        &repo,
        &buyer,
        vec![a_line()],
        "key-replay",
        PaymentOutcome::Authorized,
    )
    .await
    .expect("the second submit replays");

    assert_eq!(first_checkout, second_checkout, "the same checkout");
    assert_eq!(first_state, second_state, "the same state");
    assert_eq!(
        repo.list_for(&buyer).await.expect("list").len(),
        1,
        "a replay does not create a second order"
    );
}

#[tokio::test]
async fn an_unknown_payment_is_reconcilable() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let repo = DbOrderRepository::new(pool);

    let buyer = a_buyer();
    let (_, state, _) = submit(
        &repo,
        &buyer,
        vec![a_line()],
        "key-unknown",
        PaymentOutcome::Unknown,
    )
    .await
    .expect("submit");
    assert_eq!(
        state,
        CheckoutState::PaymentUnknown,
        "an unresolved payment is neither a success nor a failure"
    );

    let order = repo
        .list_for(&buyer)
        .await
        .expect("list")
        .pop()
        .expect("an order");
    assert!(order.awaiting_reconciliation);

    let final_state = repo
        .reconcile(&order.order_id, true)
        .await
        .expect("reconcile");
    assert_eq!(final_state, CheckoutState::Confirmed);

    let read = repo.get(&order.order_id).await.expect("get");
    assert_eq!(read.state, CheckoutState::Confirmed);
    assert!(!read.awaiting_reconciliation);
    assert!(
        read.trail.len() > order.trail.len(),
        "the reconciliation is on the trail"
    );
}

#[tokio::test]
async fn the_orders_waiting_for_reconciliation_are_listed() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let repo = DbOrderRepository::new(pool);

    let buyer = a_buyer();
    submit(
        &repo,
        &buyer,
        vec![a_line()],
        "key-unknown-2",
        PaymentOutcome::Unknown,
    )
    .await
    .expect("submit");

    let waiting = repo.awaiting_reconciliation().await.expect("list");
    assert!(
        waiting.iter().any(|order| order.buyer_id == buyer),
        "the order that is waiting for a verdict is in the list"
    );
}
