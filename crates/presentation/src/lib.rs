//! `ax-presentation` · L1 HTTP + render adapters · `ENTITY.md §2, §4.9`
//!
//! Axum router, middleware (tenant_resolver, request_id, error_to_response),
//! API handlers. Leptos SSR через feature flag `leptos-ssr` (Phase B).

pub mod api;
pub mod app_state;
pub mod middleware;
pub mod router;

#[cfg(feature = "leptos-ssr")]
pub mod leptos;

pub use app_state::AppState;
pub use router::build_router;
