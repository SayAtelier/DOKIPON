//! The inventory repository, against a real database.
//!
//! The test is skipped when there is no `DATABASE_URL`: the local run has
//! no database on purpose, and a test that needs one must not pretend to
//! have passed when there was nothing to test.
//!
//! In CI there **is** one — the workflow starts a `postgres:16-alpine` and
//! points `DATABASE_URL` at it — so this is where the schema, the id
//! sequence, the stock rule and the cross-instance visibility are actually
//! checked.
//!
//! Every test seeds **its own SKU**. The stock row is shared state: two
//! tests that reserved the same SKU would move each other's numbers, and
//! the assertions would be about the other test.

use commerce_core::db;
use commerce_inventory::{DbInventoryRepository, InventoryRepository};

/// How much stock the test's own SKU has.
const ON_HAND: i64 = 10;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
}

/// A SKU and an order id that nobody else is using.
fn a_sku_and_order() -> (String, String) {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let stamp = format!("{stamp}-{}", std::process::id());
    (format!("uat-sku-{stamp}"), format!("uat-order-{stamp}"))
}

/// A repository with the test's own SKU in it.
async fn a_repository() -> (DbInventoryRepository, String) {
    let url = database_url().expect("the caller checked");
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let repo = DbInventoryRepository::new(pool);
    let (sku, _) = a_sku_and_order();
    repo.seed(&[(sku.as_str(), ON_HAND)]).await.expect("seed");
    (repo, sku)
}

/// The level of one SKU, read out of the ledger.
async fn level_of(repo: &DbInventoryRepository, sku: &str) -> commerce_inventory::StockLevelRow {
    let page = repo.list(0, 100, None).await.expect("list");
    page.items
        .into_iter()
        .find(|row| row.sku == sku)
        .expect("the sku is in the ledger")
}

#[tokio::test]
async fn a_reservation_survives_the_round_trip() {
    if database_url().is_none() {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    }
    let (repo, sku) = a_repository().await;
    let (_, order) = a_sku_and_order();

    let reservation = repo.reserve(&sku, &order, 1).await.expect("reserve");
    assert_eq!(reservation.sku, sku);
    assert_eq!(reservation.order_id, order);
    assert_eq!(reservation.quantity, 1);
    assert!(
        reservation.reservation_id.starts_with("res-"),
        "the id keeps its shape: {}",
        reservation.reservation_id
    );

    // The level the reservation took is held against the stock.
    let row = level_of(&repo, &sku).await;
    assert_eq!(row.reserved, 1);
    assert_eq!(row.on_hand, ON_HAND);
    assert_eq!(row.available, ON_HAND - 1);
}

#[tokio::test]
async fn the_stock_is_shared_between_two_repositories() {
    if database_url().is_none() {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    }
    let url = database_url().expect("the caller checked");
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    // Two repositories, standing for two instances of the inventory
    // Function: the units must not be promised twice.
    let first = DbInventoryRepository::new(pool.clone());
    let second = DbInventoryRepository::new(pool);
    let (sku, order) = a_sku_and_order();
    first.seed(&[(sku.as_str(), ON_HAND)]).await.expect("seed");

    first
        .reserve(&sku, &order, 1)
        .await
        .expect("reserve on the first instance");

    let row = level_of(&second, &sku).await;
    assert_eq!(row.reserved, 1, "the other instance sees the reservation");
    assert_eq!(row.available, ON_HAND - 1);
}

#[tokio::test]
async fn a_reservation_cannot_exceed_the_stock() {
    if database_url().is_none() {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    }
    let (repo, sku) = a_repository().await;
    let (_, order) = a_sku_and_order();

    let error = repo
        .reserve(&sku, &order, ON_HAND + 1)
        .await
        .expect_err("more than the stock on hand must be refused");
    assert!(
        error.to_string().contains("inventory.insufficient"),
        "the conflict names itself: {error}"
    );

    let row = level_of(&repo, &sku).await;
    assert_eq!(row.reserved, 0, "the refused reservation took nothing");
}

#[tokio::test]
async fn reserving_twice_for_one_order_returns_the_same_reservation() {
    if database_url().is_none() {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    }
    let (repo, sku) = a_repository().await;
    let (_, order) = a_sku_and_order();

    let first = repo.reserve(&sku, &order, 1).await.expect("reserve");
    let second = repo.reserve(&sku, &order, 1).await.expect("reserve again");

    assert_eq!(
        first.reservation_id, second.reservation_id,
        "the same order and SKU get the same reservation"
    );

    let row = level_of(&repo, &sku).await;
    assert_eq!(row.reserved, 1, "the units are not reserved twice");
    assert_eq!(row.available, ON_HAND - 1);
}

#[tokio::test]
async fn a_committed_reservation_takes_the_units_out_of_the_stock() {
    if database_url().is_none() {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    }
    let (repo, sku) = a_repository().await;
    let (_, order) = a_sku_and_order();

    let reservation = repo.reserve(&sku, &order, 2).await.expect("reserve");
    let committed = repo
        .commit(&reservation.reservation_id)
        .await
        .expect("commit");
    assert_eq!(committed.quantity, 2);

    let row = level_of(&repo, &sku).await;
    assert_eq!(row.reserved, 0, "the units are no longer held");
    assert_eq!(row.on_hand, ON_HAND - 2, "the units left the warehouse");
    assert_eq!(row.available, ON_HAND - 2);
}
