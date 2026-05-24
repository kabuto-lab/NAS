//! `AppError` — централизованный error enum + HTTP response mapping.
//!
//! Спецификация: `ENTITY.md §7`, `ADR-001 §D7`.
//!
//! HTTP status mapping:
//! - `NotFound` → 404 с `{ code, ... }`
//! - `TenantNotResolved` → 401
//! - `TenantNotActive` → 403
//! - `TenantMismatch` → 403 (**security event** — high severity Sentry alert)
//! - `Validation` → 400
//! - `Unauthorized` → 401
//! - `Forbidden` → 403
//! - `Conflict` → 409
//! - `BadRequest` → 400
//! - `RateLimited` → 429
//! - `Internal` / `Database` → 500 с `{ code: "INTERNAL_ERROR", requestId }`

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

use crate::tenant::TenantStatus;

/// Структурированная деталь 404 NotFound — формат byte-for-byte SITE1.
///
/// Пример: `{ "code": "PAGE_NOT_FOUND", "slug": "about", "locale": "ru" }`.
#[derive(Debug, Clone, Serialize)]
pub struct NotFoundDetail {
    pub code: &'static str,
    #[serde(flatten)]
    pub fields: serde_json::Value,
}

impl std::fmt::Display for NotFoundDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.fields)
    }
}

/// Application error enum. Все application + infrastructure ошибки преобразуются
/// сюда через `From<...>` или explicit construction.
///
/// `IntoResponse` маппит в HTTP. **`TenantMismatch`** — отдельный variant потому что
/// это security event: Sentry capture с severity=high.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(NotFoundDetail),

    #[error("validation: {0}")]
    Validation(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("tenant not resolved")]
    TenantNotResolved,

    #[error("tenant not active: {0}")]
    TenantNotActive(TenantStatus),

    /// Security event — Sentry alert.
    #[error("tenant mismatch")]
    TenantMismatch,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("rate limited")]
    RateLimited,

    #[error("internal: {0}")]
    Internal(#[from] eyre_for_app::Eyre),

    #[error("database: {0}")]
    Database(String),
}

// Нет прямой `#[from] sqlx::Error` потому что `ax-common` не может зависеть от sqlx
// (boundary check). Conversion из sqlx::Error → AppError::Database происходит в
// infrastructure crate.

/// Wrapper для interop с `eyre::Report` без зависимости на eyre в common
/// (если позже захотим разнести).
pub mod eyre_for_app {
    pub use eyre::Report as Eyre;
}

impl AppError {
    /// Помечает событие как security-relevant (для Sentry severity).
    #[must_use]
    pub const fn is_security_event(&self) -> bool {
        matches!(self, Self::TenantMismatch)
    }

    /// HTTP status code для этого error.
    #[must_use]
    pub const fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Validation(_) | Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized | Self::TenantNotResolved => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) | Self::TenantNotActive(_) | Self::TenantMismatch => {
                StatusCode::FORBIDDEN
            }
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal(_) | Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = match &self {
            Self::NotFound(d) => serde_json::to_value(d).unwrap_or_else(|_| {
                serde_json::json!({ "code": "PAGE_NOT_FOUND" })
            }),
            Self::Validation(s) => serde_json::json!({ "code": "VALIDATION_FAILED", "message": s }),
            Self::Unauthorized => serde_json::json!({ "code": "NOT_AUTHENTICATED" }),
            Self::Forbidden(s) => serde_json::json!({ "code": "FORBIDDEN", "message": s }),
            Self::TenantNotResolved => serde_json::json!({ "code": "TENANT_NOT_RESOLVED" }),
            Self::TenantNotActive(s) => serde_json::json!({
                "code": "TENANT_NOT_ACTIVE",
                "status": s.as_str(),
            }),
            Self::TenantMismatch => serde_json::json!({ "code": "TENANT_OWNERSHIP_MISMATCH" }),
            Self::Conflict(s) => serde_json::json!({ "code": "CONFLICT", "message": s }),
            Self::BadRequest(s) => serde_json::json!({ "code": "BAD_REQUEST", "message": s }),
            Self::RateLimited => serde_json::json!({ "code": "RATE_LIMITED" }),
            Self::Internal(_) | Self::Database(_) => serde_json::json!({
                "code": "INTERNAL_ERROR"
            }),
        };

        // TODO: при наличии RequestId в request extensions — append "requestId" поле для
        // Internal/Database variants. Делается в presentation/middleware/error_to_response.rs.

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_code_mapping_complete() {
        // Sanity check: каждый variant имеет HTTP code
        let _ = AppError::NotFound(NotFoundDetail {
            code: "TEST",
            fields: serde_json::json!({}),
        })
        .status_code();
        let _ = AppError::Validation("x".into()).status_code();
        let _ = AppError::Unauthorized.status_code();
        let _ = AppError::Forbidden("x".into()).status_code();
        let _ = AppError::TenantNotResolved.status_code();
        let _ = AppError::TenantNotActive(TenantStatus::Suspended).status_code();
        let _ = AppError::TenantMismatch.status_code();
        let _ = AppError::Conflict("x".into()).status_code();
        let _ = AppError::BadRequest("x".into()).status_code();
        let _ = AppError::RateLimited.status_code();
        let _ = AppError::Database("x".into()).status_code();
    }

    #[test]
    fn tenant_mismatch_is_security_event() {
        assert!(AppError::TenantMismatch.is_security_event());
        assert!(!AppError::Unauthorized.is_security_event());
        assert!(!AppError::TenantNotResolved.is_security_event());
    }

    #[test]
    fn not_found_serializes_with_flat_fields() {
        let detail = NotFoundDetail {
            code: "PAGE_NOT_FOUND",
            fields: serde_json::json!({ "slug": "about", "locale": "ru" }),
        };
        let json = serde_json::to_value(&detail).unwrap();
        assert_eq!(json["code"], "PAGE_NOT_FOUND");
        assert_eq!(json["slug"], "about");
        assert_eq!(json["locale"], "ru");
    }
}
