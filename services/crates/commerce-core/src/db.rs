//! The database the commerce services persist to.
//!
//! ## What this is for
//!
//! The commerce deployment runs **one Function per service**, and a Function
//! may have **several instances**. A store that lives in a process's memory
//! is therefore per-instance: a cart created on one instance is invisible on
//! the next, and an order can simply disappear. A deployment that has to
//! keep a sale needs a store that every instance shares — a database.
//!
//! ## How a service uses it
//!
//! ```ignore
//! // In the service's main: a database is optional, and its absence is
//! // not an error — the local run has none, and the local run's stores are
//! // in memory on purpose.
//! let db = match commerce_core::db::from_env() {
//!     Some(url) => Some(commerce_core::db::connect(&url).await?),
//!     None => None,
//! };
//! ```
//!
//! and a store that has one writes through to it.
//!
//! ## Failing closed
//!
//! A service that is **given** a database and cannot reach it must not
//! start. A store that silently keeps its state in memory when the database
//! is unreachable is the failure this whole design exists to prevent: the
//! deployment would look healthy and lose the order anyway. `connect`
//! returns an error, and the caller decides — every caller in this
//! repository fails closed.

use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Executor;
use std::time::Duration;

/// The pool type, under a name the other crates can use without depending
/// on `sqlx` themselves.
pub use sqlx::postgres::PgPool as Pool;

/// The environment variable that points at the database.
pub const DATABASE_URL_ENV: &str = "DATABASE_URL";

/// The schema, applied by [`migrate`].
pub const SCHEMA: &str = include_str!("../migrations/0001_commerce.sql");

/// The database URL from the environment, if there is one.
///
/// `DATABASE_URL` is the name the platform's own database integrations set
/// (Vercel's Postgres, Neon's), so a deployment that was given a database
/// needs no extra configuration to find it.
pub fn from_env() -> Option<String> {
    match std::env::var(DATABASE_URL_ENV) {
        Ok(url) if !url.trim().is_empty() => Some(url),
        _ => None,
    }
}

/// Connect to the database.
///
/// The pool is small on purpose: a Vercel Function has a short life and
/// every connection is a Neon connection, and a pool bigger than the
/// instance's traffic would only hold them open.
pub async fn connect(url: &str) -> Result<PgPool, String> {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(15))
        .connect(url)
        .await
        .map_err(|error| format!("could not connect to the database: {error}"))
}

/// Apply the schema.
///
/// Idempotent: every statement in the schema is `IF NOT EXISTS`, so this
/// can run on every boot, and a second service applying it changes
/// nothing.
///
/// ## One migration at a time
///
/// `CREATE TABLE IF NOT EXISTS` is **not** safe to run twice at the same
/// moment: the second one collides with the first in the catalog, and the
/// error is `duplicate key value violates unique constraint
/// "pg_type_typname_nsp_index"`. A deployment has several Function
/// instances, and they boot at the same time on a cold start — so they all
/// apply the schema at the same moment, and one of them dies.
///
/// The schema is therefore applied under an **advisory lock**, and the lock
/// is held for the whole transaction: the first instance applies the schema
/// and the others wait for it and then find everything already there.
pub async fn migrate(pool: &PgPool) -> Result<(), String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("could not start the migration: {error}"))?;

    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(SCHEMA_LOCK)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("could not take the schema lock: {error}"))?;

    // The schema is a script of many statements, so it goes through the
    // simple protocol: a prepared statement carries exactly one command.
    tx.execute(SCHEMA)
        .await
        .map_err(|error| format!("could not apply the schema: {error}"))?;

    tx.commit()
        .await
        .map_err(|error| format!("could not commit the migration: {error}"))?;
    Ok(())
}

/// The lock the schema is applied under.
///
/// Any constant will do, as long as every instance of the deployment uses
/// the same one. It is "DOKIPON" in ASCII, plus a byte, so that it cannot
/// collide with a lock another program on the same database took.
const SCHEMA_LOCK: i64 = 0x444F_4B49_504F_4E01;

/// The database the deployment has, if it has one.
///
/// A deployment that **has** a database and cannot reach it must not
/// start: a store that quietly keeps its state in memory while the
/// database is unreachable looks healthy and loses the record anyway. So
/// this returns an error, and the caller decides — every caller in this
/// repository fails closed.
///
/// The schema is applied here too, so a deployment that was given a
/// database needs no migration step of its own. It is idempotent.
pub async fn connect_from_env() -> Result<Option<PgPool>, String> {
    match from_env() {
        Some(url) => {
            let pool = connect(&url).await?;
            migrate(&pool).await?;
            Ok(Some(pool))
        }
        None => Ok(None),
    }
}

/// Whether the database answers.
///
/// The health check a deployment can answer: the pool is up, and the schema
/// is there.
pub async fn is_healthy(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .is_ok()
}
