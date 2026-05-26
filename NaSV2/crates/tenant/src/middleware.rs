//! Axum middleware that resolves Host → `TenantContext` and inserts it
//! into `request.extensions()`.
//!
//! Mount with `axum::middleware::from_fn_with_state(resolver,
//! resolve_tenant)`. Downstream handlers extract via
//! `axum::Extension<TenantContext>`.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use nas2_common::AppError;

use crate::resolver::TenantResolver;

/// State value carried by `from_fn_with_state`. Held as `Arc<dyn>` so
/// the same middleware works with any resolver impl.
pub type TenantResolverHandle = Arc<dyn TenantResolver>;

pub async fn resolve_tenant(
    State(resolver): State<TenantResolverHandle>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            AppError::Validation("missing or non-ASCII Host header".to_owned())
        })?
        .to_owned();
    let ctx = resolver.resolve(&host).await?;
    req.extensions_mut().insert(ctx);
    Ok(next.run(req).await)
}
