//! Router wiring — Axum routes + middleware stack.

use axum::{
    middleware,
    routing::get,
    Router,
};
use tower_http::trace::TraceLayer;

use crate::api::{cms_handlers, health_handler, version_handler};
use crate::app_state::AppState;
use crate::middleware::{request_id, tenant_resolver};

/// Build full router with all routes and middleware.
#[must_use]
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/v1/cms/pages/public/by-slug/{slug}",
            get(cms_handlers::get_published_by_slug),
        )
        .route("/api/v1/version", get(version_handler::handler))
        .route("/health", get(health_handler::liveness))
        .route("/health/ready", get(health_handler::readiness))
        .layer(middleware::from_fn_with_state(state.clone(), tenant_resolver::middleware))
        .layer(middleware::from_fn(request_id::middleware))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
