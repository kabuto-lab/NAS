//! `nas2-presentation` · L1 HTTP + render adapters · ENTITY §2, §4.7, §10, §11.
//!
//! Today (W3 D4): router skeleton + AppState + health endpoints.
//!
//! Pending modules:
//!   - api/*       REST handlers (W3 D5 first endpoint; M2 expands)
//!   - middleware  request_id / capability_guard / csp_nonce
//!   - ssr         Leptos SSR for public pages (feature `leptos-ssr`)
//!   - admin       Leptos admin shell (M5+)

#![forbid(unsafe_code)]

pub mod app_state;
pub mod health;
pub mod router;
// pub mod api;          // W3 D5 — handlers land here
// pub mod middleware;   // request_id / capability_guard land later

pub use app_state::AppState;
pub use router::build_router;
