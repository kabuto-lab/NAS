//! Shared application state for the HTTP router.
//!
//! Cheap to clone (`Arc` internally). Cloned per request by axum's
//! `with_state` mechanism.

use std::sync::Arc;

use nas2_application::ports::PostRepository;
use nas2_tenant::TenantResolverHandle;

/// One concrete state type for axum's `with_state`. Handlers stay
/// generic in their internals (use cases monomorphize over
/// `Arc<dyn PostRepository>`); AppState is the single boundary.
#[derive(Clone)]
pub struct AppState {
    pub post_repo: Arc<dyn PostRepository>,
    pub tenant_resolver: TenantResolverHandle,
}

impl AppState {
    #[must_use]
    pub fn new(
        post_repo: Arc<dyn PostRepository>,
        tenant_resolver: TenantResolverHandle,
    ) -> Self {
        Self {
            post_repo,
            tenant_resolver,
        }
    }
}
