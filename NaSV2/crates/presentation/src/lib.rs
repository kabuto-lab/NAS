//! `nas2-presentation` · L1 HTTP + render adapters · ENTITY §2, §4.7, §10, §11.
//!
//! Public surface today:
//!   - `AppState` — single concrete state type for axum's `with_state`
//!   - `build_router` — `/health/{live,ready}` + `/api/v1/pages/:slug`
//!     gated behind `resolve_tenant` middleware
//!   - `api::pages::get_page_by_slug` — first REST handler
//!   - `caps::extract_caps_for_today` — stub capability extractor
//!     (JWT-derived form lands M2 W3)

#![forbid(unsafe_code)]

pub mod api;
pub mod app_state;
pub mod caps;
pub mod health;
pub mod router;

pub use app_state::AppState;
pub use router::build_router;
