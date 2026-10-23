//! A JSON body extractor that answers with ProblemDetails.
//!
//! ## Why this exists
//!
//! Axum's built-in `Json<T>` extractor rejects a malformed body with a
//! **plain-text** response:
//!
//! ```text
//! HTTP/1.1 422 Unprocessable Entity
//! content-type: text/plain
//!
//! Failed to deserialize the JSON body into the target type: missing field `expected_version`
//! ```
//!
//! Every other failure in this codebase answers with an RFC 9457
//! `ProblemDetails` document, so a client that parses errors has to special-
//! case one endpoint. And the plain-text body leaks the **internal field
//! name** of the struct that failed to parse, which tells an attacker the
//! server's shape.
//!
//! This extractor maps the rejection into the same ProblemDetails shape
//! with a stable machine-readable `code`, and does not leak the internals.
//!
//! ## Use
//!
//! Replace `Json(body): Json<T>` with `Json(body): JsonBody<T>`. The
//! rejection path is the only difference; a well-formed body behaves
//! identically.

use axum::{
    extract::{FromRequest, Request},
    http::header::CONTENT_TYPE,
    Json,
};
use serde::de::DeserializeOwned;

use crate::error::{status, AppError};

/// A JSON body extractor whose rejection is an RFC 9457 ProblemDetails.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonBody<T>(pub T);

impl<T, S> FromRequest<S> for JsonBody<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(JsonBody(value)),
            Err(rejection) => {
                // The rejection's own text is not sent to the client: it
                // names internal fields. The detail says what the client
                // needs to fix, and the specifics go to the log.
                tracing::debug!(error = %rejection, "rejected a malformed JSON body");
                Err(status::unprocessable(
                    "request.malformed_json",
                    "the request body is not valid JSON for this endpoint",
                    "http.json",
                ))
            }
        }
    }
}

/// A JSON body extractor that also requires a JSON content type.
///
/// A request that says `text/plain` but carries a JSON body is a client
/// mistake worth naming: some HTTP clients default to `text/plain` and the
/// failure would otherwise be a confusing parse error.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonBodyStrict<T>(pub T);

impl<T, S> FromRequest<S> for JsonBodyStrict<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !content_type.starts_with("application/json") {
            return Err(status::unsupported_media_type(
                "request.unsupported_media_type",
                "this endpoint requires content-type: application/json",
                "http.json",
            ));
        }
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(JsonBodyStrict(value)),
            Err(rejection) => {
                tracing::debug!(error = %rejection, "rejected a malformed JSON body");
                Err(status::unprocessable(
                    "request.malformed_json",
                    "the request body is not valid JSON for this endpoint",
                    "http.json",
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request as HttpRequest};
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Payload {
        quantity: u32,
    }

    async fn body_of(req: HttpRequest<Body>) -> Result<Payload, AppError> {
        let JsonBody(payload) = JsonBody::<Payload>::from_request(req, &()).await?;
        Ok(payload)
    }

    #[tokio::test]
    async fn a_well_formed_body_is_accepted() {
        let request = HttpRequest::builder()
            .method("POST")
            .uri("/")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"quantity":2}"#))
            .unwrap();
        let payload = body_of(request).await.expect("should parse");
        assert_eq!(payload.quantity, 2);
    }

    /// A malformed body must produce a ProblemDetails, not a plain-text
    /// rejection that names the internal struct field.
    #[tokio::test]
    async fn a_malformed_body_is_a_problem_details() {
        let request = HttpRequest::builder()
            .method("POST")
            .uri("/")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"quantity":"not a number"}"#))
            .unwrap();
        let err = body_of(request).await.expect_err("must reject");
        match err {
            AppError::Problem(p) => {
                assert_eq!(p.status, 422);
                assert_eq!(p.code, "request.malformed_json");
                // The internal field name must not be in the response.
                assert!(!p.detail.contains("quantity"), "detail leaked the field name");
            }
            other => panic!("expected a problem, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_missing_field_is_also_a_problem_details() {
        let request = HttpRequest::builder()
            .method("POST")
            .uri("/")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{}"#))
            .unwrap();
        let err = body_of(request).await.expect_err("must reject");
        match err {
            AppError::Problem(p) => {
                assert_eq!(p.status, 422);
                assert_eq!(p.code, "request.malformed_json");
            }
            other => panic!("expected a problem, got {other:?}"),
        }
    }
}
