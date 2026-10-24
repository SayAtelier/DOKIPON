//! The mock ↔ product agreement test.
//!
//! `mock-line-identity` issues LINE-shaped `id_token`s; `commerce-identity`
//! verifies them. If the two disagree about the token's shape — the header,
//! the claim set, the signature encoding, the issuer, the audience — then
//! every test that uses the mock is testing a **fiction**: the mock would be
//! minting tokens the product would never accept, and a green test would
//! prove nothing about the product at all.
//!
//! So this test mints with the mock and verifies with the product's own
//! `TokenVerifier`, and it does it with the credentials the local run uses —
//! the ones in `tools/commerce-e2e/mock_data/local/local.json` — so the
//! fixture set cannot drift away from the mock.
//!
//! ## Why this test is here and not in the mock crate
//!
//! The mock crate is a **test fixture**: nothing in `services/` may depend on
//! it, and it must not depend on a product service either. An agreement test
//! needs both sides, so it cannot live in either crate alone. It lives in the
//! E2E project instead — `tools/commerce-e2e/agreement/` — which is outside
//! `services/` and outside the mock crate, and reaches both by path.
//!
//! ## What "agreement" means here
//!
//! | the mock issues | the product must answer |
//! |---|---|
//! | a well-formed token | accepts it, and the claims come back |
//! | a token signed with another key | `identity.bad_signature` |
//! | a token whose `exp` is in the past | `identity.expired_token` |
//! | a token for another channel | `identity.bad_audience` |
//! | a token from another issuer | `identity.bad_issuer` |
//! | a token whose nonce is not the one presented | `identity.nonce_mismatch` |
//!
//! The problem codes are the **product's own** — the ones
//! `commerce_identity` returns — not strings this test invented.

use std::path::PathBuf;

use commerce_core::error::AppError;
use commerce_identity::TokenVerifier;
use commerce_mocks::line_identity::{LineIdentityMock, TokenFlavour};

/// The issuer LINE uses. The product hard-codes it, so the mock does too.
const ISSUER: &str = "https://access.line.me";

/// The wall clock, in seconds since the epoch — the clock the product
/// verifies `exp` against (`commerce_identity` reads the real time).
///
/// The test uses the wall clock rather than a frozen one on purpose: the
/// mock's issuer mints against the wall clock too, so this is the same
/// comparison the service makes. A frozen clock on both sides would hide a
/// disagreement between the two — and there was one: the mock used to be
/// frozen in the past, so every token it issued was born expired and every
/// running service refused it with `identity.expired_token`.
fn now_s() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// The fixture set: the credentials both sides must use.
// ---------------------------------------------------------------------------

/// The local fixture set. A machine-readable manifest, so the credentials
/// that reach the mock and the credentials that reach the product are read
/// from one place.
fn fixture() -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../mock_data/local/local.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("the local fixture set must be readable at {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("the local fixture set must be valid JSON at {}: {e}", path.display()))
}

fn credential(fixture: &serde_json::Value, name: &str) -> String {
    fixture["credentials"][name]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("the fixture set must carry credentials.{name}.value"))
        .to_string()
}

/// The signing key and the channel id, as the local run sets them.
fn credentials() -> (String, String) {
    let fixture = fixture();
    (
        credential(&fixture, "lineIdTokenKey"),
        credential(&fixture, "lineChannelId"),
    )
}

/// The mock, on the fixture set's credentials. Its clock is left alone: the
/// issuer mints against the wall clock, which is the clock the product
/// verifies against.
fn mock() -> LineIdentityMock {
    let (key, channel) = credentials();
    LineIdentityMock::new(key, channel)
}

/// The product's verifier, on the same credentials.
fn verifier() -> TokenVerifier {
    let (key, channel) = credentials();
    TokenVerifier {
        expected_audience: channel,
        expected_issuer: ISSUER.to_string(),
        signing_key: key,
    }
}

/// The product's own problem code for a refusal.
fn problem_code(err: AppError) -> String {
    match err {
        AppError::Problem(p) => p.code,
        other => panic!("the product must refuse with problem details, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// The agreement.
// ---------------------------------------------------------------------------

/// The headline: a token the mock issues is a token the product accepts.
#[test]
fn a_token_the_mock_issues_is_accepted_by_the_identity_service() {
    let token = mock().mint(TokenFlavour::Valid, None);

    let claims = verifier()
        .verify(&token, None, now_s())
        .expect("the product must accept the mock's token");

    assert_eq!(claims.sub, "U_mock_user");
    let (_, channel) = credentials();
    assert_eq!(claims.aud, channel);
    assert_eq!(claims.iss, ISSUER);
}

/// The same agreement, but over the mock's **router**: the token the mock
/// serves over HTTP is the one the product accepts. A mock whose HTTP answer
/// carries a different shape from the one its API returns would pass the test
/// above and still break every real test.
#[tokio::test]
async fn the_token_the_mock_serves_over_http_is_accepted_by_the_identity_service() {
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = commerce_mocks::line_identity::router(mock());

    let request = axum::http::Request::builder()
        .method("GET")
        .uri("/mockline/mint")
        .body(Body::empty())
        .expect("request builds");
    let response = app.oneshot(request).await.expect("the router answers");
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body reads")
        .to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("body is json");
    let token = body["idToken"]
        .as_str()
        .expect("the mock serves an idToken")
        .to_string();

    let claims = verifier()
        .verify(&token, None, now_s())
        .expect("the product must accept the token the mock serves");

    assert_eq!(claims.sub, "U_mock_user");
}

/// A token signed with another key is refused — with the product's code.
#[test]
fn a_tampered_token_is_refused() {
    let token = mock().mint(TokenFlavour::WrongSignature, None);

    let err = verifier()
        .verify(&token, None, now_s())
        .expect_err("the product must refuse a token it did not sign");

    assert_eq!(problem_code(err), "identity.bad_signature");
}

/// An expired token is refused — with the product's code.
#[test]
fn an_expired_token_is_refused() {
    let token = mock().mint(TokenFlavour::Expired, None);

    let err = verifier()
        .verify(&token, None, now_s())
        .expect_err("the product must refuse an expired token");

    assert_eq!(problem_code(err), "identity.expired_token");
}

/// A token minted for another channel is refused — with the product's code.
#[test]
fn a_wrong_audience_token_is_refused() {
    let token = mock().mint(TokenFlavour::WrongAudience, None);

    let err = verifier()
        .verify(&token, None, now_s())
        .expect_err("the product must refuse a token for another channel");

    assert_eq!(problem_code(err), "identity.bad_audience");
}

/// A token from another issuer is refused — with the product's code.
#[test]
fn a_wrong_issuer_token_is_refused() {
    let token = mock().mint(TokenFlavour::WrongIssuer, None);

    let err = verifier()
        .verify(&token, None, now_s())
        .expect_err("the product must refuse a token from another issuer");

    assert_eq!(problem_code(err), "identity.bad_issuer");
}

/// A token that carries a different nonce than the one the client presented
/// is refused — with the product's code. The mock hands out nonces so this
/// defence has something to bite on.
#[test]
fn a_nonce_mismatch_is_refused() {
    let token = mock().mint(TokenFlavour::NonceMismatch, Some("nonce-issued"));

    let err = verifier()
        .verify(&token, Some("nonce-presented"), now_s())
        .expect_err("the product must refuse a token whose nonce does not match");

    assert_eq!(problem_code(err), "identity.nonce_mismatch");
}
