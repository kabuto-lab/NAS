//! Tenant resolver middleware — Host/Header/Query → `TenantContext`.
//!
//! Priority (audit §5.1):
//! 1. Header `X-Tenant-Slug`
//! 2. Subdomain `{slug}.{TENANT_ROOT_DOMAIN}` (single-level only)
//! 3. Query `?tenant=<slug>` (для SSE)
//!
//! Slug regex unified: `^[a-z0-9][a-z0-9-]{1,62}[a-z0-9]$` (3-64 chars; fixes SITE1 H1).
//! См. ADR-001 D6.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use ax_common::{AppError, TenantContext};

use crate::app_state::AppState;

const TENANT_HEADER: &str = "x-tenant-slug";

/// Pull request → resolve tenant → inject into extensions or pass-through without context.
pub async fn middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let root_domain = std::env::var("TENANT_ROOT_DOMAIN").unwrap_or_else(|_| "spa.me".into());
    let slug = extract_slug(&req, &root_domain);

    if let Some(slug) = slug {
        if let Some(ctx) = match state.tenant_resolver.resolve_by_slug(&slug).await {
            Ok(ctx) => ctx,
            Err(e) => return e.into_response(),
        } {
            req.extensions_mut().insert(ctx);
        }
    }

    next.run(req).await
}

/// Extract slug per priority order. Returns normalized (lowercase, trimmed) or None.
fn extract_slug(req: &Request, root_domain: &str) -> Option<String> {
    // 1. Header
    if let Some(v) = req.headers().get(TENANT_HEADER).and_then(|h| h.to_str().ok()) {
        if let Some(s) = normalize_slug(v) {
            return Some(s);
        }
    }

    // 2. Subdomain
    if let Some(host) = req.headers().get(axum::http::header::HOST).and_then(|h| h.to_str().ok()) {
        let clean = host.split(':').next().unwrap_or("").to_lowercase();
        let suffix = format!(".{}", root_domain.to_lowercase());
        if clean != root_domain && clean.ends_with(&suffix) {
            let candidate = &clean[..clean.len() - suffix.len()];
            if !candidate.contains('.') {
                if let Some(s) = normalize_slug(candidate) {
                    return Some(s);
                }
            }
        }
    }

    // 3. Query param
    let uri = req.uri();
    if let Some(q) = uri.query() {
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if k == "tenant" {
                    if let Some(s) = normalize_slug(v) {
                        return Some(s);
                    }
                }
            }
        }
    }

    None
}

/// Normalize + validate slug per ADR-001 D6 (fixed to 3-64 chars, matches schema CHECK).
fn normalize_slug(raw: &str) -> Option<String> {
    let s = raw.trim().to_lowercase();
    if s.len() < 3 || s.len() > 64 {
        return None;
    }
    let bytes = s.as_bytes();
    let is_alnum = |b: u8| b.is_ascii_digit() || (b'a'..=b'z').contains(&b);
    let is_mid = |b: u8| is_alnum(b) || b == b'-';

    if !is_alnum(bytes[0]) || !is_alnum(bytes[bytes.len() - 1]) {
        return None;
    }
    if !bytes[1..bytes.len() - 1].iter().all(|&b| is_mid(b)) {
        return None;
    }
    Some(s)
}

/// Axum extractor для `TenantContext`. Returns 401 если ctx отсутствует
/// (т.е. middleware не смог resolve), 403 если tenant suspended/archived.
pub struct RequireTenant(pub TenantContext);

#[axum::async_trait]
impl<S: Send + Sync> axum::extract::FromRequestParts<S> for RequireTenant {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let ctx = parts
            .extensions
            .get::<TenantContext>()
            .cloned()
            .ok_or(AppError::TenantNotResolved)?;

        if !ctx.status.is_active() {
            return Err(AppError::TenantNotActive(ctx.status));
        }
        Ok(Self(ctx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_accepts_valid() {
        assert_eq!(normalize_slug("imperiumspa"), Some("imperiumspa".into()));
        assert_eq!(normalize_slug("pilot-tenant"), Some("pilot-tenant".into()));
        assert_eq!(normalize_slug("IMPERIUM"), Some("imperium".into()));
        assert_eq!(normalize_slug("  trimme  "), Some("trimme".into()));
    }

    #[test]
    fn normalize_rejects_invalid() {
        assert_eq!(normalize_slug(""), None);
        assert_eq!(normalize_slug("ab"), None);
        assert_eq!(normalize_slug(&"a".repeat(65)), None);
        assert_eq!(normalize_slug("-bad"), None);
        assert_eq!(normalize_slug("bad-"), None);
        assert_eq!(normalize_slug("has.dot"), None);
        assert_eq!(normalize_slug("has/slash"), None);
    }
}
