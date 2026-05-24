//! Auth middleware — extract `Authorization: Bearer <jwt>` → verify → inject
//! `AuthenticatedUser` into request extensions.
//!
//! Spec: ADR-002 §D1, §D5 (lazy capability resolve in handler — not here).
//!
//! Failure modes:
//! - missing header → pass-through (handler decides via RequireAuthenticated extractor)
//! - malformed Bearer → 401 InvalidToken
//! - verify fail → 401 InvalidToken or TokenExpired

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use ax_common::{AppError, TenantContext, UserId};
use ax_infrastructure::Claims;
use std::str::FromStr;
use uuid::Uuid;

use crate::app_state::AppState;

const AUTHZ_HEADER: &str = "authorization";
const BEARER_PREFIX: &str = "Bearer ";

/// Authenticated user — what middleware injects into req.extensions.
///
/// Populated from JWT claims. Capabilities NOT resolved here (lazy in handler).
#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub user_id: UserId,
    pub tenant_id: Uuid,
    pub role: String,
    pub kind: String,
    pub claims: Claims,
}

/// Auth middleware: extract Bearer → verify → inject AuthenticatedUser.
///
/// Tenant binding: compares JWT.tenant_id with TenantContext (set by upstream
/// tenant_resolver middleware) — mismatch → 403 TenantMismatch (security event).
pub async fn middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let header_val = req.headers().get(AUTHZ_HEADER).cloned();
    let Some(header_val) = header_val else {
        return next.run(req).await;
    };

    let Ok(header_str) = header_val.to_str() else {
        return AppError::InvalidToken.into_response();
    };

    let Some(token) = header_str.strip_prefix(BEARER_PREFIX) else {
        return AppError::InvalidToken.into_response();
    };

    let claims = match state.jwt_verifier.verify(token) {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    let user_uuid = match Uuid::from_str(&claims.sub) {
        Ok(u) => u,
        Err(_) => return AppError::InvalidToken.into_response(),
    };
    let tenant_uuid = match Uuid::from_str(&claims.tenant_id) {
        Ok(u) => u,
        Err(_) => return AppError::InvalidToken.into_response(),
    };

    // Tenant binding check: if tenant_resolver middleware uже установил TenantContext,
    // и JWT tenant_id не совпадает — это TenantMismatch (high-severity security event).
    if let Some(ctx) = req.extensions().get::<TenantContext>() {
        if ctx.tenant_id.0 != tenant_uuid {
            tracing::warn!(
                jwt_tenant = %tenant_uuid,
                req_tenant = %ctx.tenant_id,
                "JWT tenant_id does not match resolved tenant"
            );
            return AppError::TenantMismatch.into_response();
        }
    }

    let role = claims.role.clone();
    let kind = claims.kind.clone();
    let user = AuthenticatedUser {
        user_id: UserId::new(user_uuid),
        tenant_id: tenant_uuid,
        role,
        kind,
        claims,
    };

    req.extensions_mut().insert(user);

    next.run(req).await
}

/// Extractor — requires `AuthenticatedUser` в extensions. 401 если absent.
pub struct RequireAuthenticated(pub AuthenticatedUser);

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for RequireAuthenticated {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .map(Self)
            .ok_or(AppError::Unauthorized)
    }
}
