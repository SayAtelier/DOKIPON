//! The session repository, against a real database.
//!
//! The test is skipped when there is no `DATABASE_URL`: the local run has
//! no database on purpose, and a test that needs one must not pretend to
//! have passed when there was nothing to test.
//!
//! In CI there **is** one — the workflow starts a `postgres:16-alpine` and
//! points `DATABASE_URL` at it — so this is where the schema, the id
//! sequence and the nonce's single-use rule are actually checked.

use commerce_core::db;
use commerce_identity::{Channel, Session, SessionKind, SessionRepo};

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
}

fn a_session(id: &str) -> Session {
    Session {
        session_id: id.to_string(),
        kind: SessionKind::LineUser,
        channel: Channel::Web,
        subject: Some("U_uat".to_string()),
        issued_at_ms: 1_000,
        expires_at_ms: 1_000 + 3_600_000,
        revoked: false,
    }
}

#[tokio::test]
async fn a_session_survives_the_round_trip() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let repo = SessionRepo::new(pool);

    let id = repo.next_session_id().await.expect("mint an id");
    assert!(id.starts_with("sess-"), "the id keeps its shape: {id}");

    let session = a_session(&id);
    repo.save_session(&session).await.expect("save");

    let read = repo.load_session(&id).await.expect("load").expect("there");
    assert_eq!(read.session_id, id);
    assert_eq!(read.kind, SessionKind::LineUser);
    assert_eq!(read.channel, Channel::Web);
    assert_eq!(read.subject.as_deref(), Some("U_uat"));
    assert_eq!(read.expires_at_ms, session.expires_at_ms);
}

#[tokio::test]
async fn two_ids_are_never_the_same() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let repo = SessionRepo::new(pool);

    // The whole point of the sequence: two instances of the service must not
    // mint the same session id, or two shoppers share one cart.
    let first = repo.next_session_id().await.expect("mint");
    let second = repo.next_session_id().await.expect("mint");
    assert_ne!(first, second);
}

#[tokio::test]
async fn a_nonce_can_only_be_taken_once() {
    let Some(url) = database_url() else {
        eprintln!("no DATABASE_URL: the database path is not exercised here");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let repo = SessionRepo::new(pool);

    // A nonce nobody else has: the rule is that it can be taken once, and
    // a test that reused another test's nonce would prove nothing.
    let nonce = format!(
        "uat-nonce-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    assert!(repo.consume_nonce(&nonce, 1_000).await.expect("take"));
    assert!(
        !repo.consume_nonce(&nonce, 1_000).await.expect("take again"),
        "a nonce that was already taken must not be taken twice"
    );
}
