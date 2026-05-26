//! `GET /api/v1/pages/:slug` — fetch a published page by slug.
//!
//! ## TLA layers
//!
//! - **L1 Correctness** — `PostSlug::try_new` validates the path
//!   parameter (400 on malformed); use case composes capability gate +
//!   repo + published-status filter (404 on draft/unknown).
//! - **L2 Performance** — `state.post_repo.clone()` is `Arc::clone`
//!   (cheap, no alloc). Use case is constructed per request — zero
//!   stateful object overhead.
//! - **L3 Scalability** — repo behind Arc<dyn>, pool fan-out handled
//!   by the adapter. Cache wraps the handler later (M10).
//! - **L4 Operability** — handler-level tracing span lives in
//!   `tracing::instrument` on a future middleware (M2). Today: raw.

use axum::{
    Json,
    extract::{Extension, Path, State},
};
use nas2_application::queries::GetPublishedPageBySlug;
use nas2_common::AppError;
use nas2_domain::{Post, PostSlug};
use nas2_tenant::TenantContext;

use crate::app_state::AppState;
use crate::caps::extract_caps_for_today;

/// Public page lookup. Capability requirement: `cms.page.read`.
///
/// caps.require(cms.page.read)
///
/// Marker above is mandatory — `xtask capability-coverage` (W4 D2)
/// greps for `caps.require(<cap>)` near handlers. Removing it without
/// renaming the extractor pattern is a CI-breaking change.
pub async fn get_page_by_slug(
    State(state): State<AppState>,
    Extension(tctx): Extension<TenantContext>,
    Path(slug_raw): Path<String>,
) -> Result<Json<Post>, AppError> {
    let slug = PostSlug::try_new(&slug_raw)?;
    let caps = extract_caps_for_today();
    let uc = GetPublishedPageBySlug::new(state.post_repo.clone());
    let post = uc
        .execute(tctx.tenant_id, tctx.site_id, &slug, &caps)
        .await?;
    Ok(Json(post))
}
