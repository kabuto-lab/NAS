//! Top-level axum router builder.
//!
//! Mount order:
//!   1. routes (health + first REST endpoint)
//!   2. tenant_resolver middleware (runs FIRST on request —
//!      middleware order is request-from-outermost)
//!   3. with_state(AppState)
//!
//! ## Trade-off: middleware applies to /health/* too
//!
//! A load-balancer probe that doesn't set Host (or sets an unmapped
//! one) will get 400/404 from the resolver. If LB probes flap, refactor
//! to nest the health routes outside the resolver layer via
//! `Router::nest` or split into a second Router::merge'd group.

use axum::{Router, middleware::from_fn_with_state, routing::get};
use nas2_tenant::resolve_tenant;

use crate::api;
use crate::app_state::AppState;
use crate::health;

/// Construct the full router. Apps wire this into `axum::serve`.
pub fn build_router(state: AppState) -> Router {
    let resolver = state.tenant_resolver.clone();
    Router::new()
        .route("/health/live", get(health::live))
        .route("/health/ready", get(health::ready))
        .route("/api/v1/pages/{slug}", get(api::pages::get_page_by_slug))
        .layer(from_fn_with_state(resolver, resolve_tenant))
        .with_state(state)
}
