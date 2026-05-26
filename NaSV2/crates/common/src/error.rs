//! Cross-layer error type · ENTITY §20.
//!
//! Variants chosen to cover every HTTP semantic the public API surface
//! needs to express. Internal layers (domain / application /
//! infrastructure) all return `AppError`; presentation maps it to HTTP
//! via the `IntoResponse` impl below.
//!
//! ENTITY §20 invariant: errors NEVER leak DB query text, file paths,
//! or stack traces to the client. The `Display` impls below produce
//! human-readable summaries only; full context flows to operators via
//! `tracing::error!` at the call site.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

/// The canonical application error.
///
/// All variants carry a single short message string — no embedded
/// structured payload. Reason: the surface stays auditable; richer
/// structures get expressed in domain-specific errors that
/// `From`-convert into `AppError::Validation` etc.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("internal error")]
    Internal,
}

impl AppError {
    /// HTTP status code each variant maps to.
    #[must_use]
    pub const fn http_status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Short stable identifier for client error-handling. Suitable for
    /// `error.code` field in JSON responses.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "not_found",
            Self::Forbidden(_) => "forbidden",
            Self::Validation(_) => "validation_failed",
            Self::Conflict(_) => "conflict",
            Self::Internal => "internal_error",
        }
    }
}

#[derive(Debug, Serialize)]
struct WireError<'a> {
    code: &'a str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.http_status();
        let body = WireError {
            code: self.code(),
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_methods,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use serde_json::Value;

    #[test]
    fn display_includes_message() {
        let e = AppError::NotFound("page".to_string());
        assert!(e.to_string().contains("not found: page"));
    }

    #[test]
    fn http_status_mapping_complete() {
        assert_eq!(
            AppError::NotFound(String::new()).http_status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::Forbidden(String::new()).http_status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            AppError::Validation(String::new()).http_status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::Conflict(String::new()).http_status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            AppError::Internal.http_status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn code_strings_are_stable() {
        assert_eq!(AppError::NotFound(String::new()).code(), "not_found");
        assert_eq!(AppError::Forbidden(String::new()).code(), "forbidden");
        assert_eq!(
            AppError::Validation(String::new()).code(),
            "validation_failed"
        );
        assert_eq!(AppError::Conflict(String::new()).code(), "conflict");
        assert_eq!(AppError::Internal.code(), "internal_error");
    }

    #[tokio::test]
    async fn into_response_not_found_returns_404() {
        let resp = AppError::NotFound("x".to_string()).into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn into_response_internal_returns_500() {
        let resp = AppError::Internal.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn response_body_carries_code_and_message() {
        let resp = AppError::Validation("bad slug".to_string()).into_response();
        let body = resp.into_body();
        let bytes = to_bytes(body, 4096).await.expect("collect body");
        let parsed: Value = serde_json::from_slice(&bytes).expect("json parse");
        assert_eq!(parsed["code"], "validation_failed");
        assert!(
            parsed["message"]
                .as_str()
                .unwrap_or("")
                .contains("bad slug")
        );
    }

    #[test]
    fn internal_variant_leaks_no_detail() {
        // ENTITY §20 invariant: Internal must never carry payload.
        let e = AppError::Internal;
        assert_eq!(e.to_string(), "internal error");
    }

    #[test]
    fn is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AppError>();
    }
}
