//! LINE id-token issuer mock — serves `CASE-E2E-002`, `CASE-E2E-003`.
//!
//! This mock **issues** id_tokens with the same claim set and the same failure
//! modes LINE actually has, so the server's verification path can be exercised
//! end to end without a LINE account.
//!
//! It models the four rejection paths the design cares about:
//!
//! | Token | Server must answer |
//! |---|---|
//! | wrong `aud` (other channel) | 401 `identity.bad_audience` |
//! | wrong `iss` | 401 `identity.bad_issuer` |
//! | `exp` in the past | 401 `identity.expired_token` |
//! | nonce mismatch | 401 `identity.nonce_mismatch` |
//!
//! It cannot model platform review, policy, or a real LINE login. Those stay
//! `[Hypothetical]` no matter what this crate does.

use axum::{extract::Query, http::StatusCode, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

/// The claims LINE puts in an `id_token`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdTokenClaims {
    /// Always `https://access.line.me`.
    pub iss: String,
    /// The channel id the token was minted for.
    pub aud: String,
    /// LINE user id. Note: NOT a display name.
    pub sub: String,
    /// Expiry, seconds since epoch.
    pub exp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenFlavour {
    /// A correct token.
    Valid,
    /// Signed with a different key.
    WrongSignature,
    /// `aud` is another channel's id.
    WrongAudience,
    /// `iss` is an attacker.
    WrongIssuer,
    /// Already expired.
    Expired,
    /// Carries a nonce different from the one the client sent.
    NonceMismatch,
    /// `alg: none` — the algorithm-substitution attack.
    AlgNone,
    /// Structurally not a JWS.
    Malformed,
}

#[derive(Clone)]
pub struct LineIdentityMock {
    signing_key: String,
    expected_audience: String,
    expected_issuer: String,
    /// Nonces already handed out. A nonce may be used once — this is what
    /// proves the server's replay defence has something to bite on.
    issued_nonces: Arc<RwLock<HashSet<String>>>,
    now_s: Arc<RwLock<i64>>,
}

impl LineIdentityMock {
    pub fn new(signing_key: impl Into<String>, expected_audience: impl Into<String>) -> Self {
        Self {
            signing_key: signing_key.into(),
            expected_audience: expected_audience.into(),
            expected_issuer: "https://access.line.me".to_string(),
            issued_nonces: Arc::new(RwLock::new(HashSet::new())),
            now_s: Arc::new(RwLock::new(1_700_000_000)),
        }
    }

    pub fn set_now(&self, epoch_s: i64) {
        *self.now_s.write().expect("line lock") = epoch_s;
    }

    pub fn issued_nonce_count(&self) -> usize {
        self.issued_nonces.read().expect("line lock").len()
    }

    /// Mint a token of the requested flavour.
    pub fn mint(&self, flavour: TokenFlavour, nonce: Option<&str>) -> String {
        let now = *self.now_s.read().expect("line lock");
        let mut claims = IdTokenClaims {
            iss: self.expected_issuer.clone(),
            aud: self.expected_audience.clone(),
            sub: "U_mock_user".to_string(),
            exp: now + 3600,
            nonce: nonce.map(str::to_string),
            // LINE includes these; the server must ignore them rather than
            // trust them as authorization inputs.
            name: Some("Mock Taro".to_string()),
            picture: Some("https://example.invalid/avatar.png".to_string()),
        };

        let key = match flavour {
            TokenFlavour::WrongSignature => "a-different-key".to_string(),
            _ => self.signing_key.clone(),
        };
        match flavour {
            TokenFlavour::WrongAudience => claims.aud = "9999999999".into(),
            TokenFlavour::WrongIssuer => claims.iss = "https://evil.example".into(),
            TokenFlavour::Expired => claims.exp = now - 1,
            _ => {}
        }
        if flavour == TokenFlavour::AlgNone {
            // Header says `none` and the signature is empty.
            let h = b64(br#"{"alg":"none","typ":"JWT"}"#);
            let p = b64(serde_json::to_string(&claims).unwrap().as_bytes());
            return format!("{h}.{p}.");
        }
        if flavour == TokenFlavour::Malformed {
            return "not.a-jws".to_string();
        }

        if let Some(n) = nonce {
            self.issued_nonces
                .write()
                .expect("line lock")
                .insert(n.to_string());
        }

        let h = b64(br#"{"alg":"HS256","typ":"JWT"}"#);
        let p = b64(serde_json::to_string(&claims).unwrap().as_bytes());
        let signed = format!("{h}.{p}");
        let sig = hmac_hex(key.as_bytes(), signed.as_bytes());
        format!("{signed}.{sig}")
    }
}

#[derive(Debug, Deserialize)]
struct MintQuery {
    #[serde(default)]
    flavour: Option<TokenFlavour>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    sub: Option<String>,
}

pub fn router(mock: LineIdentityMock) -> Router {
    let m1 = mock.clone();
    let m2 = mock.clone();
    let m3 = mock.clone();
    Router::new()
        .route(
            "/mockline/mint",
            get(move |Query(q): Query<MintQuery>| {
                let mock = m1.clone();
                async move {
                    let flavour = q.flavour.unwrap_or(TokenFlavour::Valid);
                    let token = mock.mint(flavour, q.nonce.as_deref());
                    Json(serde_json::json!({
                        "idToken": token,
                        "flavour": flavour,
                        "nonce": q.nonce,
                        "sub": q.sub.unwrap_or_else(|| "U_mock_user".into()),
                        "note": "Profile claims are present but the server must ignore them as authorization inputs.",
                    }))
                }
            }),
        )
        .route(
            "/mockline/claims",
            get(move |Json(t): Json<serde_json::Value>| {
                let mock = m2.clone();
                async move {
                    let token = t["idToken"].as_str().unwrap_or_default().to_string();
                    Json(serde_json::json!({
                        "parts": token.split('.').count(),
                        "issuedNonces": mock.issued_nonce_count(),
                        "expectedIssuer": mock.expected_issuer,
                        "expectedAudience": mock.expected_audience,
                    }))
                }
            }),
        )
        .route(
            "/mockline/reset",
            axum::routing::post(move || {
                let mock = m3.clone();
                async move {
                    mock.issued_nonces.write().expect("line lock").clear();
                    Json(serde_json::json!({ "ok": true }))
                }
            }),
        )
        .route(
            "/healthz",
            get(|| async { StatusCode::OK }),
        )
}

// ------------------------------------------------------------------ codecs

fn b64(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

/// HS256, matching `commerce_identity`'s verifier so the mock and the SUT agree
/// on the wire format rather than each having its own idea of it.
fn hmac_hex(key: &[u8], message: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    const BLOCK: usize = 64;
    let mut k0 = [0u8; BLOCK];
    if key.len() > BLOCK {
        let d = Sha256::digest(key);
        k0[..32].copy_from_slice(&d);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mock() -> LineIdentityMock {
        LineIdentityMock::new("shared-secret", "1234567890")
    }

    fn decode_payload(token: &str) -> serde_json::Value {
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        use base64::Engine;
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[1])
            .unwrap();
        serde_json::from_slice(&raw).unwrap()
    }

    #[test]
    fn valid_token_carries_the_full_claim_set() {
        let token = mock().mint(TokenFlavour::Valid, Some("n1"));
        let c = decode_payload(&token);
        assert_eq!(c["iss"], "https://access.line.me");
        assert_eq!(c["aud"], "1234567890");
        assert_eq!(c["sub"], "U_mock_user");
        assert_eq!(c["nonce"], "n1");
        // LINE really does include these; the server must not use them.
        assert_eq!(c["name"], "Mock Taro");
    }

    #[test]
    fn token_has_three_jws_parts_and_a_signature() {
        let token = mock().mint(TokenFlavour::Valid, None);
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(!parts[2].is_empty(), "valid tokens are signed");
    }

    #[test]
    fn wrong_audience_is_distinguishable_from_a_valid_token() {
        let token = mock().mint(TokenFlavour::WrongAudience, None);
        assert_eq!(decode_payload(&token)["aud"], "9999999999");
    }

    #[test]
    fn wrong_issuer_is_distinguishable() {
        let token = mock().mint(TokenFlavour::WrongIssuer, None);
        assert_eq!(decode_payload(&token)["iss"], "https://evil.example");
    }

    #[test]
    fn expired_token_has_a_past_exp() {
        let m = mock();
        let now = *m.now_s.read().unwrap();
        let token = m.mint(TokenFlavour::Expired, None);
        assert!(decode_payload(&token)["exp"].as_i64().unwrap() < now);
    }

    #[test]
    fn alg_none_has_an_empty_signature() {
        let token = mock().mint(TokenFlavour::AlgNone, None);
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts[2].is_empty(), "alg:none carries no signature");
        use base64::Engine;
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[0])
            .unwrap();
        assert!(String::from_utf8(header).unwrap().contains("\"none\""));
    }

    #[test]
    fn malformed_is_not_a_jws() {
        assert_eq!(mock().mint(TokenFlavour::Malformed, None), "not.a-jws");
    }

    #[test]
    fn wrong_signature_differs_from_the_correct_one() {
        let m = mock();
        let good = m.mint(TokenFlavour::Valid, None);
        let bad = m.mint(TokenFlavour::WrongSignature, None);
        let gp: Vec<&str> = good.split('.').collect();
        let bp: Vec<&str> = bad.split('.').collect();
        assert_eq!(gp[..2], bp[..2], "header+payload identical");
        assert_ne!(gp[2], bp[2], "signature differs");
    }

    #[test]
    fn issued_nonces_are_tracked_so_replay_is_detectable() {
        let m = mock();
        assert_eq!(m.issued_nonce_count(), 0);
        m.mint(TokenFlavour::Valid, Some("n1"));
        m.mint(TokenFlavour::Valid, Some("n2"));
        assert_eq!(m.issued_nonce_count(), 2);
    }

    #[test]
    fn set_now_moves_the_clock_for_expiry_tests() {
        let m = mock();
        m.set_now(2_000_000_000);
        let c = decode_payload(&m.mint(TokenFlavour::Valid, None));
        assert!(c["exp"].as_i64().unwrap() > 2_000_000_000);
    }
}
