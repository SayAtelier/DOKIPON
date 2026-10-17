//! RFC 9457 Problem Details, extended with `code`, `traceId`, `retryable`
//! as required by the atlas API contract.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemDetails {
    /// `type` URI reference.
    #[serde(rename = "type")]
    pub type_: String,
    pub title: String,
    pub status: u16,
    /// Stable machine-readable error code, e.g. `checkout.conflict`.
    pub code: String,
    pub detail: String,
    #[serde(rename = "traceId")]
    pub trace_id: String,
    /// Drives client retry policy. Never true for authz failures.
    pub retryable: bool,
}

impl ProblemDetails {
    pub fn new(
        type_: impl Into<String>,
        title: impl Into<String>,
        status: StatusCode,
        code: impl Into<String>,
        detail: impl Into<String>,
        trace_id: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            type_: type_.into(),
            title: title.into(),
            status: status.as_u16(),
            code: code.into(),
            detail: detail.into(),
            trace_id: trace_id.into(),
            retryable,
        }
    }
}

impl IntoResponse for ProblemDetails {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = serde_json::to_vec(&self).unwrap_or_else(|_| b"{}".to_vec());
        let mut resp = Response::new(axum::body::Body::from(body));
        *resp.status_mut() = status;
        resp.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/problem+json"),
        );
        resp
    }
}

impl std::fmt::Display for ProblemDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]: {}", self.title, self.code, self.detail)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Problem(ProblemDetails),
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn with_trace(mut self, trace_id: &str) -> Self {
        match &mut self {
            AppError::Problem(p) if p.trace_id.is_empty() => p.trace_id = trace_id.to_string(),
            AppError::Problem(p) => p.trace_id = trace_id.to_string(),
            _ => {}
        }
        self
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Problem(p) => p.into_response(),
            AppError::Internal(msg) => {
                // Never leak internals to the client body; log the detail instead.
                tracing::error!(error = %msg, "internal error");
                ProblemDetails::new(
                    "https://api.gemarc.example/problems/internal",
                    "Internal Server Error",
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "server.internal",
                    "an internal error occurred",
                    "",
                    false,
                )
                .into_response()
            }
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Internal(format!("db: {e}"))
    }
}

pub mod status {
    use super::*;

    pub fn bad_request(code: &str, detail: impl Into<String>, trace: &str) -> AppError {
        AppError::Problem(ProblemDetails::new(
            "https://api.gemarc.example/problems/bad-request",
            "Bad Request",
            StatusCode::BAD_REQUEST,
            code,
            detail,
            trace,
            false,
        ))
    }

    pub fn unauthorized(code: &str, detail: impl Into<String>, trace: &str) -> AppError {
        AppError::Problem(ProblemDetails::new(
            "https://api.gemarc.example/problems/unauthorized",
            "Unauthorized",
            StatusCode::UNAUTHORIZED,
            code,
            detail,
            trace,
            false,
        ))
    }

    pub fn forbidden(code: &str, detail: impl Into<String>, trace: &str) -> AppError {
        AppError::Problem(ProblemDetails::new(
            "https://api.gemarc.example/problems/forbidden",
            "Forbidden",
            StatusCode::FORBIDDEN,
            code,
            detail,
            trace,
            false,
        ))
    }

    pub fn not_found(code: &str, detail: impl Into<String>, trace: &str) -> AppError {
        AppError::Problem(ProblemDetails::new(
            "https://api.gemarc.example/problems/not-found",
            "Not Found",
            StatusCode::NOT_FOUND,
            code,
            detail,
            trace,
            false,
        ))
    }

    /// 409 — used by idempotency-key payload conflicts and optimistic-lock misses.
    pub fn conflict(code: &str, detail: impl Into<String>, trace: &str) -> AppError {
        AppError::Problem(ProblemDetails::new(
            "https://api.gemarc.example/problems/conflict",
            "Conflict",
            StatusCode::CONFLICT,
            code,
            detail,
            trace,
            false,
        ))
    }

    pub fn unprocessable(code: &str, detail: impl Into<String>, trace: &str) -> AppError {
        AppError::Problem(ProblemDetails::new(
            "https://api.gemarc.example/problems/unprocessable",
            "Unprocessable Entity",
            StatusCode::UNPROCESSABLE_ENTITY,
            code,
            detail,
            trace,
            false,
        ))
    }

    pub fn too_many_requests(detail: impl Into<String>, trace: &str) -> AppError {
        AppError::Problem(ProblemDetails::new(
            "https://api.gemarc.example/problems/too-many-requests",
            "Too Many Requests",
            StatusCode::TOO_MANY_REQUESTS,
            "rate.limited",
            detail,
            trace,
            true,
        ))
    }

    pub fn service_unavailable(code: &str, detail: impl Into<String>, trace: &str) -> AppError {
        AppError::Problem(ProblemDetails::new(
            "https://api.gemarc.example/problems/service-unavailable",
            "Service Unavailable",
            StatusCode::SERVICE_UNAVAILABLE,
            code,
            detail,
            trace,
            true,
        ))
    }
}
