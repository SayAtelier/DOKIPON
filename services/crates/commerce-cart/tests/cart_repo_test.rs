//! The cart repository, against a real database.
//!
//! The test is skipped when there is no `DATABASE_URL`: the local run has
//! no database on purpose, and a test that needs one must not pretend to
//! have passed when there was nothing to test.
//!
//! In CI there **is** one — the workflow starts a `postgres:16-alpine` and
//! points `DATABASE_URL` at it — so this is where the schema, the version
//! check and the cross-instance visibility are actually checked.

use commerce_cart::{CartRepository, DbCartRepository};
use commerce_core::db;
use commerce_core::{Currency, Money};

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
}

/// A cart id nobody else is using.
fn a_cart_id() -> String {
    format!(
        "uat-cart-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

fn jpy(minor: i64) -> Money {
    Money::new(minor, Currency::Jpy).expect("a price")
}

#[tokio::test]
async fn a_cart_survives_the_round_trip() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let repo = DbCartRepository::new(pool);

    let id = a_cart_id();
    let price = jpy(4_800);
    let updated = repo
        .apply_mutation(&id, None, "sku-tee-01", 2, 1, price)
        .await
        .expect("the first mutation creates the cart");
    assert_eq!(updated.version, 2, "the version is bumped by the mutation");
    assert_eq!(updated.items.len(), 1);
    assert_eq!(updated.items[0].sku, "sku-tee-01");
    assert_eq!(updated.items[0].quantity, 2);
    assert_eq!(updated.items[0].unit_amount_snapshot, 4_800);

    let read = repo.load(&id).await.expect("load").expect("there");
    assert_eq!(read.cart_id, id);
    assert_eq!(read.version, 2);
    assert_eq!(read.items.len(), 1);
    assert_eq!(read.items[0].currency, Currency::Jpy);
    assert_eq!(read.items[0].quantity, 2);
}

#[tokio::test]
async fn a_stale_version_is_a_conflict() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let repo = DbCartRepository::new(pool);

    let id = a_cart_id();
    repo.apply_mutation(&id, None, "sku-tee-01", 1, 1, jpy(4_800))
        .await
        .expect("the first mutation creates the cart");

    // The caller believes the cart is at version 1, and it is at version 2.
    let error = repo
        .apply_mutation(&id, None, "sku-bra-01", 1, 1, jpy(8_900))
        .await
        .expect_err("a stale version must be refused");
    assert!(
        error.to_string().contains("cart.version_conflict"),
        "the conflict names itself: {error}"
    );
}

#[tokio::test]
async fn two_repositories_share_one_cart() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    // Two repositories, standing for two instances of the cart Function. The
    // whole reason the carts are in the database: a cart created on one
    // instance has to be visible to the next.
    let first = DbCartRepository::new(pool.clone());
    let second = DbCartRepository::new(pool);

    let id = a_cart_id();
    first
        .apply_mutation(&id, None, "sku-brief-01", 1, 1, jpy(3_900))
        .await
        .expect("the first instance creates the cart");

    let read = second.load(&id).await.expect("load").expect("there");
    assert_eq!(read.items.len(), 1, "the other instance sees the line");
    assert_eq!(read.items[0].sku, "sku-brief-01");
    assert_eq!(read.version, 2);
}

#[tokio::test]
async fn an_idempotency_key_replays_its_answer() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let repo = DbCartRepository::new(pool);

    let id = a_cart_id();
    let key = format!("uat-key-{id}");
    let answer = serde_json::json!({"cartId": id, "_fp": "fingerprint-1"});
    repo.remember(&id, &key, &answer).await.expect("remember");

    let replay = repo
        .replay(&id, &key)
        .await
        .expect("replay")
        .expect("there");
    assert_eq!(
        replay.get("_fp").and_then(|v| v.as_str()),
        Some("fingerprint-1")
    );

    // A different cart is a different actor, so the same key is not its own.
    assert!(
        repo.replay(&a_cart_id(), &key)
            .await
            .expect("replay")
            .is_none(),
        "the key is scoped to its cart"
    );
}
