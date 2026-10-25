//! The payment repository, against a real database.
//!
//! The test is skipped when there is no `DATABASE_URL`: the local run has
//! no database on purpose, and a test that needs one must not pretend to
//! have passed when there was nothing to test.
//!
//! In CI there **is** one — the workflow starts a `postgres:16-alpine` and
//! points `DATABASE_URL` at it — so this is where the schema, the id
//! sequence, the replay rule and the cross-instance visibility are actually
//! checked.

use commerce_core::db;
use commerce_core::{Currency, Money};
use commerce_payment_adapter::{
    AuthorizationOutcome, DbPaymentRepository, PaymentAdapter, PaymentRepository, WebhookEvent,
};

/// The provider's webhook secret. The test and the repository share it.
const SECRET: &str = "uat-webhook-secret";

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
}

/// An order id nobody else is using.
fn an_order() -> String {
    format!(
        "uat-order-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

fn jpy(minor: i64) -> Money {
    Money::new(minor, Currency::Jpy).expect("an amount")
}

#[tokio::test]
async fn an_attempt_survives_the_round_trip() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let repo = DbPaymentRepository::new(pool, SECRET);

    let order = an_order();
    let attempt = repo
        .authorize(&order, "pm-test", jpy(8_900))
        .await
        .expect("authorize");

    assert_eq!(attempt.order_id, order);
    assert_eq!(attempt.amount_minor, 8_900);
    assert_eq!(attempt.currency, Currency::Jpy);
    assert_eq!(attempt.status, AuthorizationOutcome::Authorized);
    assert!(
        attempt.attempt_id.starts_with("pa-"),
        "the id keeps its shape: {}",
        attempt.attempt_id
    );

    let page = repo.list(Some(&order), None, 20).await.expect("list");
    assert_eq!(page.items.len(), 1, "the attempt is in the listing");
    assert_eq!(page.items[0].payment_attempt_id, attempt.attempt_id);
    assert_eq!(page.items[0].amount.amount_minor, 8_900);
}

#[tokio::test]
async fn two_repositories_share_one_attempt() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    // Two repositories, standing for two instances of the payment Function.
    let first = DbPaymentRepository::new(pool.clone(), SECRET);
    let second = DbPaymentRepository::new(pool, SECRET);

    let order = an_order();
    first
        .authorize(&order, "pm-test", jpy(1_800))
        .await
        .expect("authorize on the first instance");

    let page = second
        .list(Some(&order), None, 20)
        .await
        .expect("list on the second instance");
    assert_eq!(page.items.len(), 1, "the other instance sees the attempt");
}

#[tokio::test]
async fn a_webhook_is_accepted_once() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let repo = DbPaymentRepository::new(pool, SECRET);

    let order = an_order();
    let attempt = repo
        .authorize(&order, "pm-test", jpy(3_900))
        .await
        .expect("authorize");

    let now = commerce_core::idempotency::now_ms();
    let mut event = WebhookEvent {
        provider_event_id: format!("uat-event-{order}"),
        event_type: "payment.declined".to_string(),
        provider_ref: attempt.provider_ref.clone(),
        occurred_at_ms: now,
        signature: String::new(),
    };
    event.signature = PaymentAdapter::new(SECRET).sign(&event);

    let accepted = repo
        .verify_and_accept(event.clone(), now)
        .await
        .expect("the webhook is accepted");
    assert_eq!(accepted.status, AuthorizationOutcome::Declined);

    // The same event again is a replay, and a replay is refused.
    let error = repo
        .verify_and_accept(event, now)
        .await
        .expect_err("a replayed webhook must be refused");
    assert!(
        error.to_string().contains("payment.webhook_replay"),
        "the conflict names itself: {error}"
    );
}

#[tokio::test]
async fn an_attempt_can_be_reconciled() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let repo = DbPaymentRepository::new(pool, SECRET);

    let order = an_order();
    let attempt = repo
        .authorize(&order, "pm-test", jpy(4_800))
        .await
        .expect("authorize");

    let reconciled = repo
        .reconcile(&attempt.provider_ref, AuthorizationOutcome::Declined)
        .await
        .expect("reconcile");
    assert_eq!(reconciled.status, AuthorizationOutcome::Declined);
    assert!(!reconciled.reconciliation_required);

    // The other instance sees the reconciled verdict.
    let page = repo.list(Some(&order), None, 20).await.expect("list");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].status, AuthorizationOutcome::Declined);
}
