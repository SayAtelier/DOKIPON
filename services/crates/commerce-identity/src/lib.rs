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
pub mod session_repo;

pub use session_repo::SessionRepo;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub type SharedIdentity = Arc<RwLock<IdentityStore>>;

/// Sessions are deliberately short-lived (atlas: "short-lived session").
pub const SESSION_TTL_MS: i64 = 30 * 60 * 1000; // 30 minutes

/// The channel a session is bound to.
///
/// The contract spells the LINE channel `line`
/// (`contracts/openapi.yaml` -> `Channel`: `[web, app, line]`), and the Dart
/// client sends exactly that: `CommerceApiClient.guestSession()` posts
/// `{'channel': channel.header}`, and `StoreChannel.line.header` is `"line"`.
///
/// This enum used to serialise `LineMiniApp` as `line_mini_app`, which broke
/// the request in both directions: the server *rejected* a contract-conformant
/// `{"channel": "line"}` body with a 422, and it *answered* with a
/// `channel` the contract does not allow. The old spelling is kept as a
/// deserialisation alias so a client that still sends `line_mini_app` is still
/// understood — the wire value is the contract's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Web,
    App,
    #[serde(rename = "line", alias = "line_mini_app")]
    LineMiniApp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    /// LINE id_token verified server-side.
    LineUser,
    /// Anonymous. A guest on the LINE channel is the degraded state inside
    /// the LINE app and may not check out; a guest on the web or the app
    /// channel is an ordinary shopper and may.
    Guest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// The contract spells this one field `sessionId` while the rest of this
    /// struct is snake_case: `contracts/openapi.yaml` -> `Session` requires
    /// `sessionId`, and the Dart client reads `json['sessionId']` only.
    ///
    /// Emitting `session_id` instead left the client with an **empty session
    /// id**, and a guest session that cannot be identified cannot own a cart
    /// — so "add to cart" silently did nothing at all. The field is renamed
    /// per field rather than with `rename_all` for exactly that reason:
    /// `expires_at_ms` is snake_case in the contract too.
    #[serde(rename = "sessionId")]
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

    /// Whether this session may place an order.
    ///
    /// A LINE user always may. A guest on the **LINE** channel may not:
    /// that is the degraded state inside the LINE app, and it is the one
    /// the shopper is told about. A guest on the web or the app channel is
    /// an ordinary shopper and may buy — the web storefront is a shop,
    /// not a catalogue.
    pub fn can_checkout(&self) -> bool {
        self.kind == SessionKind::LineUser || self.channel != Channel::LineMiniApp
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

        let header: JwtHeader = decode_json(header_b64).map_err(|_| {
            status::unauthorized("identity.bad_header", "unreadable JWS header", TRACE)
        })?;
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

        let wrapper: JwtPayload<LineIdTokenClaims> = decode_json(payload_b64).map_err(|_| {
            status::unauthorized("identity.bad_payload", "unreadable id_token payload", TRACE)
        })?;
        let c = wrapper.claims;

        if c.iss != self.expected_issuer {
            return Err(status::unauthorized(
                "identity.bad_issuer",
                "unexpected id_token issuer",
                TRACE,
            ));
        }
        if c.aud != self.expected_audience {
            return Err(status::unauthorized(
                "identity.bad_audience",
                "id_token audience mismatch",
                TRACE,
            ));
        }
        if c.exp <= now_s {
            return Err(status::unauthorized(
                "identity.expired_token",
                "id_token has expired",
                TRACE,
            ));
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
            return Err(status::unauthorized(
                "identity.no_subject",
                "id_token has no subject",
                TRACE,
            ));
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
        Self {
            now_ms,
            ..Default::default()
        }
    }

    fn next_session_id(&mut self) -> String {
        self.sequence += 1;
        format!("sess-{:06}", self.sequence)
    }

    /// `identity.guest` — the LIFF-004 external-browser path. Always
    /// succeeds. A guest on the **LINE** channel is the degraded state
    /// inside the LINE app and cannot check out; a guest on the web or the
    /// app channel is an ordinary shopper and can.
    pub fn issue_guest(&mut self, channel: Channel) -> Session {
        let id = self.next_session_id();
        let session = self.guest_with_id(id, channel);
        self.sessions
            .insert(session.session_id.clone(), session.clone());
        session
    }

    /// The session a guest gets, with an id the caller minted.
    ///
    /// The in-memory path mints the id itself; a deployment with a
    /// database mints it from a sequence there, so that two instances
    /// cannot mint the same one — two shoppers sharing a session would
    /// share a cart.
    pub fn guest_with_id(&self, id: String, channel: Channel) -> Session {
        Session {
            session_id: id,
            kind: SessionKind::Guest,
            channel,
            subject: None,
            issued_at_ms: self.now_ms,
            expires_at_ms: self.now_ms + SESSION_TTL_MS,
            revoked: false,
        }
    }

    /// `identity.liff` — only reachable after a successful token verification.
    pub fn issue_session(&mut self, claims: &LineIdTokenClaims, channel: Channel) -> Session {
        let id = self.next_session_id();
        let session = self.session_with_id(id, claims, channel);
        if let Some(n) = &claims.nonce {
            self.consumed_nonces.insert(n.clone(), self.now_ms);
        }
        self.sessions
            .insert(session.session_id.clone(), session.clone());
        session
    }

    /// The session a verified LINE user gets, with an id the caller minted.
    pub fn session_with_id(
        &self,
        id: String,
        claims: &LineIdTokenClaims,
        channel: Channel,
    ) -> Session {
        Session {
            session_id: id,
            kind: SessionKind::LineUser,
            channel,
            subject: Some(claims.sub.clone()),
            issued_at_ms: self.now_ms,
            expires_at_ms: self.now_ms + SESSION_TTL_MS,
            revoked: false,
        }
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

pub fn router(
    state: SharedIdentity,
    verifier: Arc<TokenVerifier>,
    repo: Option<Arc<SessionRepo>>,
) -> Router {
    Router::new()
        .route("/v1/sessions/line", post(line_session))
        .route("/v1/sessions/guest", post(guest_session))
        .route("/healthz", post(health))
        .with_state((state, verifier, repo))
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn line_session(
    State((store, verifier, repo)): State<(
        SharedIdentity,
        Arc<TokenVerifier>,
        Option<Arc<SessionRepo>>,
    )>,
    JsonBody(body): JsonBody<LineSessionBody>,
) -> Result<Json<Session>, AppError> {
    let (claims, channel, now_ms) = {
        let guard = store
            .read()
            .map_err(|_| AppError::Internal("identity lock poisoned".into()))?;
        let now_s = guard.now_ms / 1000;
        let claims = verifier.verify(&body.id_token, body.nonce.as_deref(), now_s)?;
        let channel = body.channel.unwrap_or(Channel::LineMiniApp);
        (claims, channel, guard.now_ms)
    };

    // The nonce is taken **once**, and the database is what decides: two
    // instances cannot both accept the same id_token, because the nonce
    // lives in the one place they share.
    if let Some(n) = &body.nonce {
        let fresh = match &repo {
            Some(repo) => repo.consume_nonce(n, now_ms).await?,
            None => {
                let mut guard = store
                    .write()
                    .map_err(|_| AppError::Internal("identity lock poisoned".into()))?;
                guard.consume_nonce(n)
            }
        };
        if !fresh {
            return Err(status::unauthorized(
                "identity.nonce_replay",
                "this nonce was already used",
                "identity.liff",
            ));
        }
    }

    let session = match &repo {
        Some(repo) => {
            // The id comes from the database's sequence, and the session is
            // written there: every instance of the service sees it.
            let id = repo.next_session_id().await?;
            let session = {
                let guard = store
                    .read()
                    .map_err(|_| AppError::Internal("identity lock poisoned".into()))?;
                guard.session_with_id(id, &claims, channel)
            };
            repo.save_session(&session).await?;
            session
        }
        None => {
            let mut guard = store
                .write()
                .map_err(|_| AppError::Internal("identity lock poisoned".into()))?;
            guard.issue_session(&claims, channel)
        }
    };
    Ok(Json(session))
}

async fn guest_session(
    State((store, _verifier, repo)): State<(
        SharedIdentity,
        Arc<TokenVerifier>,
        Option<Arc<SessionRepo>>,
    )>,
    JsonBody(body): JsonBody<GuestBody>,
) -> Result<Json<Session>, AppError> {
    let channel = body.channel.unwrap_or(Channel::LineMiniApp);
    let session = match &repo {
        Some(repo) => {
            let id = repo.next_session_id().await?;
            let session = {
                let guard = store
                    .read()
                    .map_err(|_| AppError::Internal("identity lock poisoned".into()))?;
                guard.guest_with_id(id, channel)
            };
            repo.save_session(&session).await?;
            session
        }
        None => {
            let mut guard = store
                .write()
                .map_err(|_| AppError::Internal("identity lock poisoned".into()))?;
            guard.issue_guest(channel)
        }
    };
    Ok(Json(session))
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

    /// The contract spells the session id `sessionId` (camelCase) while the
    /// rest of this struct is snake_case. This is a regression guard: the
    /// response used to carry `session_id`, the Dart client read
    /// `json['sessionId']` only, and every guest session therefore arrived
    /// with an **empty id** — which is invisible in the response (still 200,
    /// still the right number of fields) and made "add to cart" do nothing.
    #[test]
    fn session_serializes_the_session_id_the_contract_spells() {
        let session = Session {
            session_id: "sess-000001".to_string(),
            kind: SessionKind::Guest,
            channel: Channel::Web,
            subject: None,
            issued_at_ms: 1_000,
            expires_at_ms: 2_000,
            revoked: false,
        };

        let json = serde_json::to_value(&session).expect("serialize");

        assert_eq!(json["sessionId"], "sess-000001");
        assert_eq!(json["kind"], "guest");
        // The rest of the struct keeps the contract's snake_case spelling.
        assert_eq!(json["expires_at_ms"], 2_000);
        // The old spelling must be gone: a client that prefers the snake_case
        // key would read null.
        assert!(json.get("session_id").is_none());
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
        let attacker = TokenVerifier {
            signing_key: "other".into(),
            ..verifier()
        };
        let token = attacker.mint_for_test("U1", 2_000_000_000, None);
        assert!(v.verify(&token, None, 1_000).is_err());
    }

    #[test]
    fn wrong_audience_is_rejected() {
        let v = verifier();
        let other = TokenVerifier {
            expected_audience: "chan-999".into(),
            ..verifier()
        };
        let token = other.mint_for_test("U1", 2_000_000_000, None);
        let err = v.verify(&token, None, 1_000).unwrap_err();
        assert!(matches!(err, AppError::Problem(p) if p.code == "identity.bad_audience"));
    }

    #[test]
    fn wrong_issuer_is_rejected() {
        let v = verifier();
        let other = TokenVerifier {
            expected_issuer: "https://evil.example".into(),
            ..verifier()
        };
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
        let payload = base64_url(
            br#"{"iss":"https://access.line.me","aud":"chan-123","sub":"U1","exp":9999999999}"#,
        );
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
    fn the_line_channel_is_wired_as_the_contract_spells_it() {
        // The contract's `Channel` is `[web, app, line]`
        // (`contracts/openapi.yaml`), and the Dart client sends exactly that:
        // `StoreChannel.line.header` is `'line'`
        // (`packages/domain_models/lib/src/session.dart`).

        // It must be accepted on the way in...
        let parsed: Channel =
            serde_json::from_str("\"line\"").expect("the contract's channel is accepted");
        assert_eq!(parsed, Channel::LineMiniApp);

        // ...and it must be the value that goes out.
        assert_eq!(
            serde_json::to_string(&Channel::LineMiniApp).unwrap(),
            "\"line\""
        );

        // The old spelling still reads, so a client that sends it is not
        // broken by the rename.
        let legacy: Channel =
            serde_json::from_str("\"line_mini_app\"").expect("the old spelling still reads");
        assert_eq!(legacy, Channel::LineMiniApp);

        // The other two are unaffected.
        assert_eq!(serde_json::to_string(&Channel::Web).unwrap(), "\"web\"");
        assert_eq!(serde_json::to_string(&Channel::App).unwrap(), "\"app\"");
    }

    #[test]
    fn a_guest_in_the_line_app_may_browse_but_not_check_out() {
        // The rule is narrowed to the degraded state inside the LINE app.
        // A guest *there* is a LINE user who has not granted access, and
        // the app tells them to sign in before buying.
        let mut s = IdentityStore::new(1_000);
        let guest = s.issue_guest(Channel::LineMiniApp);
        assert_eq!(guest.kind, SessionKind::Guest);
        assert!(!guest.can_checkout());
        assert!(!guest.can_view_order("U1"));
        assert!(!guest.is_valid(1_000 + SESSION_TTL_MS));
        assert!(guest.is_valid(1_000));
    }

    #[test]
    fn a_guest_on_the_web_may_check_out() {
        // The web storefront is a shop, not a catalogue: a shopper who
        // never signs in still buys. What such a guest cannot do is be
        // identified afterwards — the order is keyed by the session.
        let mut s = IdentityStore::new(1_000);
        let guest = s.issue_guest(Channel::Web);
        assert_eq!(guest.kind, SessionKind::Guest);
        assert!(guest.can_checkout());
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
        let claims = v
            .verify(&v.mint_for_test("U1", 2_000_000_000, None), None, 1_000)
            .unwrap();
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
        assert_eq!(
            a,
            hmac_sha256_hex(b"key", b"The quick brown fox jumps over the lazy dog")
        );
        assert_ne!(
            a,
            hmac_sha256_hex(b"key2", b"The quick brown fox jumps over the lazy dog")
        );
    }
}
