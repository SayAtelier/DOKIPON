//! The sessions, in a database every instance of the service shares.
//!
//! ## Why the sessions and not the orders
//!
//! A Vercel Function may have several instances, and a store that lives in
//! one process's memory is therefore per-instance. Two consequences, both
//! real:
//!
//!   - **the id collides.** The in-memory counter starts at one in every
//!     process, so two instances both mint `sess-000001` — and two
//!     different shoppers then share one session, and with it one cart.
//!   - **the session is unknown to the next instance.** A session issued by
//!     one instance cannot be validated by another.
//!
//! The session id therefore comes from a **sequence in the database** when
//! there is one, and the session itself is written there, so every instance
//! sees the same one.
//!
//! ## What is the truth
//!
//! When a database is configured, **the database is the truth** and the
//! in-memory map is not consulted for a session the process did not issue
//! itself. A store that silently answered from its own memory while the
//! database said otherwise is the failure this whole design exists to
//! prevent.

use crate::{Channel, Session, SessionKind};
use commerce_core::error::AppError;
use sqlx::postgres::PgPool;
use sqlx::Row;

/// A field that would not read.
fn read_error(error: sqlx::Error) -> AppError {
    AppError::Internal(format!("could not read the session: {error}"))
}

/// The session repository.
#[derive(Clone)]
pub struct SessionRepo {
    pool: PgPool,
}

impl SessionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// The next session id.
    ///
    /// From the database's sequence, so that two instances cannot mint the
    /// same one. The shape is the one the in-memory store uses
    /// (`sess-000001`), so nothing downstream has to learn a new id.
    pub async fn next_session_id(&self) -> Result<String, AppError> {
        let value: i64 = sqlx::query_scalar("SELECT nextval('identity_session_seq')")
            .fetch_one(&self.pool)
            .await
            .map_err(|error| AppError::Internal(format!("could not mint a session id: {error}")))?;
        Ok(format!("sess-{value:06}"))
    }

    /// Write a session, and the nonce it consumed, in one go.
    pub async fn save_session(&self, session: &Session) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO identity_session \
             (session_id, kind, channel, subject, issued_at_ms, expires_at_ms, revoked) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (session_id) DO UPDATE SET \
               kind = EXCLUDED.kind, channel = EXCLUDED.channel, subject = EXCLUDED.subject, \
               expires_at_ms = EXCLUDED.expires_at_ms, revoked = EXCLUDED.revoked",
        )
        .bind(session.session_id.as_str())
        .bind(match session.kind {
            SessionKind::Guest => "guest",
            SessionKind::LineUser => "line_user",
        })
        .bind(match session.channel {
            Channel::Web => "web",
            Channel::App => "app",
            Channel::LineMiniApp => "line",
        })
        .bind(session.subject.as_deref())
        .bind(session.issued_at_ms)
        .bind(session.expires_at_ms)
        .bind(session.revoked)
        .execute(&self.pool)
        .await
        .map_err(|error| AppError::Internal(format!("could not write the session: {error}")))?;
        Ok(())
    }

    /// Read a session back.
    pub async fn load_session(&self, session_id: &str) -> Result<Option<Session>, AppError> {
        let row = sqlx::query(
            "SELECT session_id, kind, channel, subject, issued_at_ms, expires_at_ms, revoked \
             FROM identity_session WHERE session_id = $1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| AppError::Internal(format!("could not read the session: {error}")))?;

        let Some(row) = row else {
            return Ok(None);
        };
        let id: String = row.try_get("session_id").map_err(read_error)?;
        let kind: String = row.try_get("kind").map_err(read_error)?;
        let channel: String = row.try_get("channel").map_err(read_error)?;
        let subject: Option<String> = row.try_get("subject").map_err(read_error)?;
        let issued_at_ms: i64 = row.try_get("issued_at_ms").map_err(read_error)?;
        let expires_at_ms: i64 = row.try_get("expires_at_ms").map_err(read_error)?;
        let revoked: bool = row.try_get("revoked").map_err(read_error)?;

        Ok(Some(Session {
            session_id: id,
            kind: match kind.as_str() {
                "line_user" => SessionKind::LineUser,
                _ => SessionKind::Guest,
            },
            channel: match channel.as_str() {
                "web" => Channel::Web,
                "app" => Channel::App,
                _ => Channel::LineMiniApp,
            },
            subject,
            issued_at_ms,
            expires_at_ms,
            revoked,
        }))
    }

    /// Take a nonce, once.
    ///
    /// The insert is the check: a nonce that is already there conflicts, and
    /// the caller is told it was a replay. Two instances cannot both take the
    /// same nonce, because the database decides, not the process.
    pub async fn consume_nonce(&self, nonce: &str, now_ms: i64) -> Result<bool, AppError> {
        let inserted = sqlx::query(
            "INSERT INTO consumed_nonce (nonce, used_at_ms) VALUES ($1, $2) \
             ON CONFLICT (nonce) DO NOTHING",
        )
        .bind(nonce)
        .bind(now_ms)
        .execute(&self.pool)
        .await
        .map_err(|error| AppError::Internal(format!("could not consume the nonce: {error}")))?;
        Ok(inserted.rows_affected() == 1)
    }
}
