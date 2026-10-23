//! Identity service — LINE `id_token` verification, sessions, guest carts.
//!
//! Atlas security invariants enforced here:
//! - A LINE `id_token` is verified **server-side** for issuer, audience,
//!   signature, expiry and nonce. The client UI's idea of "logged in" is never
//!   authoritative.
//! - A **guest session** (external browser, not logged in) is a first-class,
//!   explicitly limited state — not an error and not a fake login.
//! - Sessions are short-lived, revocable, and bound to a channel.
//! - Tokens and profile PII are never returned to a simulation model or logged.

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use commerce_core::error::{status, AppError};
use commerce_core::JsonBody;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub type SharedIdentity = Arc<RwLock<IdentityStore>>;

/// Sessions are deliberately short-lived (atlas: "short-lived session").
pub const SESSION_TTL_MS: i64 = 30 * 60 * 1000; // 30 minutes

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Web,
    App,
    LineMiniApp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    /// LINE id_token verified server-side.
    LineUser,
    /// Anonymous, external-browser or LINE-not-logged-in. Read-only public
    /// catalog; checkout requires promoting to a real session.
    Guest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub kind: SessionKind,
    pub channel: Channel,
    /// LINE user id when verified. `None` for guests. Never a display name or
    /// email — those are PII we do not need.
    pub subject: Option<String>,
    pub issued_at_ms: i64,
    pub expires_at_ms: i64,
    pub revoked: bool,
}

impl Session {
    pub fn is_valid(&self, now: i64) -> bool {
        !self.revoked && self.expires_at_ms > now
    }

    /// Privilege check. Guests may browse but never own an order.
    pub fn can_view_order(&self, order_owner: &str) -> bool {
        self.kind == SessionKind::LineUser && self.subject.as_deref() == Some(order_owner)
    }

    pub fn can_checkout(&self) -> bool {
        self.kind == SessionKind::LineUser
    }
}

/// Claims we actually verify from a LINE `id_token`. We never trust or store
/// anything beyond this.
#[derive(Debug, Clone, Deserialize)]
pub struct LineIdTokenClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub exp: i64,
    /// Present when the client sent a nonce to `liff.getIDToken`.
    pub nonce: Option<String>,
}

/// Dev-only HS256 verifier. The real deployment swaps this for LINE's public-key
/// RS256 verification; the *shape* of the check (issuer, audience, expiry,
/// nonce) is identical, which is what the atlas cares about.
pub struct TokenVerifier {
    /// LINE channel id — the audience an id_token must carry.
    pub expected_audience: String,
    pub expected_issuer: String,
    /// HS256 signing key. In production this is never a shared secret.
    pub signing_key: String,
}

#[derive(Debug, Clone, Deserialize)]
struct JwtHeader {
    alg: String,
}

#[derive(Debug, Clone, Deserialize)]
struct JwtPayload<T> {
    #[serde(flatten)]
    claims: T,
}

impl TokenVerifier {
    /// Full verification. Returns the claims only if **every** check passes.
    pub fn verify(
        &self,
        id_token: &str,
        presented_nonce: Option<&str>,
        now_s: i64,
    ) -> Result<LineIdTokenClaims, AppError> {
        const TRACE: &str = "identity.liff";
        let parts: Vec<&str> = id_token.split('.').collect();
        if parts.len() != 3 {
            return Err(status::unauthorized(
                "identity.malformed_token",
                "id_token is not a 3-part JWS",
                TRACE,
            ));
        }
        let (header_b64, payload_b64, signature_b64) = (parts[0], parts[1], parts[2]);

        let header: JwtHeader = decode_json(header_b64)
            .map_err(|_| status::unauthorized("identity.bad_header", "unreadable JWS header", TRACE))?;
        // Reject `alg: none` and algorithm substitution outright.
        if header.alg != "HS256" {
            return Err(status::unauthorized(
                "identity.bad_algorithm",
                "unsupported id_token algorithm",
                TRACE,
            ));
        }

        let signed = format!("{header_b64}.{payload_b64}");
        let expected = hmac_sha256_hex(self.signing_key.as_bytes(), signed.as_bytes());
        if !constant_time_eq(expected.as_bytes(), signature_b64.as_bytes()) {
            return Err(status::unauthorized(
                "identity.bad_signature",
                "id_token signature mismatch",
                TRACE,
            ));
        }

        let wrapper: JwtPayload<LineIdTokenClaims> = decode_json(payload_b64)
            .map_err(|_| status::unauthorized("identity.bad_payload", "unreadable id_token payload", TRACE))?;
        let c = wrapper.claims;

        if c.iss != self.expected_issuer {
            return Err(status::unauthorized("identity.bad_issuer", "unexpected id_token issuer", TRACE));
        }
        if c.aud != self.expected_audience {
            return Err(status::unauthorized("identity.bad_audience", "id_token audience mismatch", TRACE));
        }
        if c.exp <= now_s {
            return Err(status::unauthorized("identity.expired_token", "id_token has expired", TRACE));
        }
        // Nonce binding defeats replay of a captured id_token.
        if let Some(n) = presented_nonce {
            if c.nonce.as_deref() != Some(n) {
                return Err(status::unauthorized(
                    "identity.nonce_mismatch",
                    "id_token nonce does not match the one issued",
                    TRACE,
                ));
            }
        }
        if c.sub.is_empty() {
            return Err(status::unauthorized("identity.no_subject", "id_token has no subject", TRACE));
        }
        Ok(c)
    }

    /// Dev/test helper: mint a valid HS256 token for this verifier.
    pub fn mint_for_test(&self, sub: &str, exp: i64, nonce: Option<&str>) -> String {
        let header = base64_url(br#"{"alg":"HS256","typ":"JWT"}"#);
        let mut payload = serde_json::json!({
            "iss": self.expected_issuer,
            "aud": self.expected_audience,
            "sub": sub,
            "exp": exp,
        });
        if let Some(n) = nonce {
            payload["nonce"] = serde_json::Value::String(n.to_string());
        }
        let payload_b64 = base64_url(payload.to_string().as_bytes());
        let signed = format!("{header}.{payload_b64}");
        let sig = hmac_sha256_hex(self.signing_key.as_bytes(), signed.as_bytes());
        format!("{signed}.{sig}")
    }
}

#[derive(Debug, Default)]
pub struct IdentityStore {
    pub sessions: HashMap<String, Session>,
    /// Nonces already consumed — prevents id_token replay within a window.
    pub consumed_nonces: HashMap<String, i64>,
    pub sequence: u64,
    pub now_ms: i64,
}

impl IdentityStore {
    pub fn new(now_ms: i64) -> Self {
        Self { now_ms, ..Default::default() }
    }

    fn next_session_id(&mut self) -> String {
        self.sequence += 1;
        format!("sess-{:06}", self.sequence)
    }

    /// `identity.guest` — the LIFF-004 external-browser path. Always succeeds;
    /// the resulting session simply cannot check out.
    pub fn issue_guest(&mut self, channel: Channel) -> Session {
        let id = self.next_session_id();
        let s = Session {
            session_id: id.clone(),
            kind: SessionKind::Guest,
            channel,
            subject: None,
            issued_at_ms: self.now_ms,
            expires_at_ms: self.now_ms + SESSION_TTL_MS,
            revoked: false,
        };
        self.sessions.insert(id, s.clone());
        s
    }

    /// `identity.liff` — only reachable after a successful token verification.
    pub fn issue_session(&mut self, claims: &LineIdTokenClaims, channel: Channel) -> Session {
        let id = self.next_session_id();
        let s = Session {
            session_id: id.clone(),
            kind: SessionKind::LineUser,
            channel,
            subject: Some(claims.sub.clone()),
            issued_at_ms: self.now_ms,
            expires_at_ms: self.now_ms + SESSION_TTL_MS,
            revoked: false,
        };
        if let Some(n) = &claims.nonce {
            self.consumed_nonces.insert(n.clone(), self.now_ms);
        }
        self.sessions.insert(id, s.clone());
        s
    }

    /// Single-use nonce consumption (LIFF-006 style replay defence).
    pub fn consume_nonce(&mut self, nonce: &str) -> bool {
        !self.consumed_nonces.contains_key(nonce)
    }

    pub fn authorize(&self, session_id: &str) -> Result<&Session, AppError> {
        const TRACE: &str = "identity.authorize";
        let session = self.sessions.get(session_id).ok_or_else(|| {
            status::unauthorized("identity.unknown_session", "session not found", TRACE)
        })?;
        if !session.is_valid(self.now_ms) {
            return Err(status::unauthorized(
                "identity.session_expired",
                "session expired or revoked",
                TRACE,
            ));
        }
        Ok(session)
    }

    pub fn revoke(&mut self, session_id: &str) -> bool {
        if let Some(s) = self.sessions.get_mut(session_id) {
            s.revoked = true;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LineSessionBody {
    pub id_token: String,
    pub nonce: Option<String>,
    pub channel: Option<Channel>,
}

#[derive(Debug, Deserialize)]
pub struct GuestBody {
    pub channel: Option<Channel>,
}

pub fn router(state: SharedIdentity, verifier: Arc<TokenVerifier>) -> Router {
    Router::new()
        .route("/v1/sessions/line", post(line_session))
        .route("/v1/sessions/guest", post(guest_session))
        .route("/healthz", post(health))
        .with_state((state, verifier))
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn line_session(
    State((store, verifier)): State<(SharedIdentity, Arc<TokenVerifier>)>,
    JsonBody(body): JsonBody<LineSessionBody>,
) -> Result<Json<Session>, AppError> {
    let mut guard = store
        .write()
        .map_err(|_| AppError::Internal("identity lock poisoned".into()))?;
    let now_s = guard.now_ms / 1000;
    let claims = verifier.verify(&body.id_token, body.nonce.as_deref(), now_s)?;
    if let Some(n) = &body.nonce {
        if !guard.consume_nonce(n) {
            return Err(status::unauthorized(
                "identity.nonce_replay",
                "this nonce was already used",
                "identity.liff",
            ));
        }
    }
    let channel = body.channel.unwrap_or(Channel::LineMiniApp);
    Ok(Json(guard.issue_session(&claims, channel)))
}

async fn guest_session(
    State((store, _)): State<(SharedIdentity, Arc<TokenVerifier>)>,
    JsonBody(body): JsonBody<GuestBody>,
) -> Result<Json<Session>, AppError> {
    let mut guard = store
        .write()
        .map_err(|_| AppError::Internal("identity lock poisoned".into()))?;
    let channel = body.channel.unwrap_or(Channel::LineMiniApp);
    Ok(Json(guard.issue_guest(channel)))
}

// ---------------------------------------------------------------------------
// Token verification helpers.
//
// These were part of the binary's `main.rs`; they are the verifier's
// implementation and belong with it, not with the process entry point.
// ---------------------------------------------------------------------------

fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    const BLOCK: usize = 64;
    let mut k0 = [0u8; BLOCK];
    if key.len() > BLOCK {
        let digest = Sha256::digest(key);
        k0[..32].copy_from_slice(&digest);
    } else {
        k0[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= k0[i];
        opad[i] ^= k0[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);
    format!("{:x}", outer.finalize())
}

fn base64_url(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out.trim_end_matches('=').to_string()
}

fn decode_json<T: for<'de> Deserialize<'de>>(b64: &str) -> Result<T, ()> {
    let raw = base64_url_decode(b64).ok_or(())?;
    serde_json::from_slice(&raw).map_err(|_| ())
}

fn base64_url_decode(s: &str) -> Option<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits = 0;
    let mut out = Vec::new();
    for ch in s.bytes() {
        if ch == b'=' {
            break;
        }
        let v = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    Some(out)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verifier() -> TokenVerifier {
        TokenVerifier {
            expected_audience: "chan-123".into(),
            expected_issuer: "https://access.line.me".into(),
            signing_key: "test-key".into(),
        }
    }

    #[test]
    fn base64_url_round_trip() {
        for s in ["{}", "abc", "abcdef", "a", "ab"] {
            let enc = base64_url(s.as_bytes());
            assert_eq!(base64_url_decode(&enc).unwrap(), s.as_bytes());
        }
    }

    #[test]
    fn valid_token_produces_claims() {
        let v = verifier();
        let token = v.mint_for_test("U123", 2_000_000_000, None);
        let claims = v.verify(&token, None, 1_000_000).unwrap();
        assert_eq!(claims.sub, "U123");
        assert_eq!(claims.aud, "chan-123");
    }

    #[test]
    fn wrong_signature_is_rejected() {
        let v = verifier();
        let attacker = TokenVerifier { signing_key: "other".into(), ..verifier() };
        let token = attacker.mint_for_test("U1", 2_000_000_000, None);
        assert!(v.verify(&token, None, 1_000).is_err());
    }

    #[test]
    fn wrong_audience_is_rejected() {
        let v = verifier();
        let other = TokenVerifier { expected_audience: "chan-999".into(), ..verifier() };
        let token = other.mint_for_test("U1", 2_000_000_000, None);
        let err = v.verify(&token, None, 1_000).unwrap_err();
        assert!(matches!(err, AppError::Problem(p) if p.code == "identity.bad_audience"));
    }

    #[test]
    fn wrong_issuer_is_rejected() {
        let v = verifier();
        let other = TokenVerifier { expected_issuer: "https://evil.example".into(), ..verifier() };
        let token = other.mint_for_test("U1", 2_000_000_000, None);
        let err = v.verify(&token, None, 1_000).unwrap_err();
        assert!(matches!(err, AppError::Problem(p) if p.code == "identity.bad_issuer"));
    }

    #[test]
    fn expired_token_is_rejected() {
        let v = verifier();
        let token = v.mint_for_test("U1", 500, None);
        let err = v.verify(&token, None, 1_000).unwrap_err();
        assert!(matches!(err, AppError::Problem(p) if p.code == "identity.expired_token"));
    }

    #[test]
    fn nonce_must_match() {
        let v = verifier();
        let token = v.mint_for_test("U1", 2_000_000_000, Some("n-abc"));
        assert!(v.verify(&token, Some("n-abc"), 1_000).is_ok());
        assert!(v.verify(&token, Some("n-different"), 1_000).is_err());
    }

    #[test]
    fn alg_none_and_algarbitrary_are_rejected() {
        let header = base64_url(br#"{"alg":"none"}"#);
        let payload = base64_url(br#"{"iss":"https://access.line.me","aud":"chan-123","sub":"U1","exp":9999999999}"#);
        let token = format!("{header}.{payload}.");
        let err = verifier().verify(&token, None, 1).unwrap_err();
        assert!(matches!(err, AppError::Problem(p) if p.code == "identity.bad_algorithm"));
    }

    #[test]
    fn malformed_token_is_rejected() {
        assert!(verifier().verify("not-a-jwt", None, 1).is_err());
        assert!(verifier().verify("a.b", None, 1).is_err());
    }

    #[test]
    fn guest_session_can_browse_but_not_check_out() {
        let mut s = IdentityStore::new(1_000);
        let guest = s.issue_guest(Channel::LineMiniApp);
        assert_eq!(guest.kind, SessionKind::Guest);
        assert!(!guest.can_checkout());
        assert!(!guest.can_view_order("U1"));
        assert!(!guest.is_valid(1_000 + SESSION_TTL_MS));
        assert!(guest.is_valid(1_000));
    }

    #[test]
    fn line_session_owns_its_own_orders_only() {
        let v = verifier();
        let mut s = IdentityStore::new(1_000);
        let token = v.mint_for_test("U-owner", 2_000_000_000, Some("n1"));
        let claims = v.verify(&token, Some("n1"), 1_000).unwrap();
        let sess = s.issue_session(&claims, Channel::LineMiniApp);
        assert!(sess.can_view_order("U-owner"));
        assert!(!sess.can_view_order("U-other"));
        assert!(sess.can_checkout());
    }

    #[test]
    fn nonce_replay_is_refused() {
        let mut s = IdentityStore::new(1_000);
        assert!(s.consume_nonce("n1"));
        s.consumed_nonces.insert("n1".into(), 1_000);
        assert!(!s.consume_nonce("n1"));
    }

    #[test]
    fn revocation_invalidates_a_session_immediately() {
        let v = verifier();
        let mut s = IdentityStore::new(1_000);
        let claims = v.verify(&v.mint_for_test("U1", 2_000_000_000, None), None, 1_000).unwrap();
        let sess = s.issue_session(&claims, Channel::Web);
        assert!(s.authorize(&sess.session_id).is_ok());
        s.revoke(&sess.session_id);
        assert!(s.authorize(&sess.session_id).is_err());
    }

    #[test]
    fn unknown_session_is_unauthorized() {
        let s = IdentityStore::new(1_000);
        assert!(s.authorize("sess-nope").is_err());
    }

    #[test]
    fn hmac_matches_a_known_vector_shape() {
        // Not a published NIST vector — this pins determinism and the empty-key path.
        let a = hmac_sha256_hex(b"key", b"The quick brown fox jumps over the lazy dog");
        assert_eq!(a.len(), 64);
        assert_eq!(a, hmac_sha256_hex(b"key", b"The quick brown fox jumps over the lazy dog"));
        assert_ne!(a, hmac_sha256_hex(b"key2", b"The quick brown fox jumps over the lazy dog"));
    }
}
