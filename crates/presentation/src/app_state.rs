//! `AppState` — DI container для Axum handlers.
//!
//! Содержит Arc'ed traits для repos/resolvers + use cases (которые сами Arc-обёрнуты).

use std::sync::Arc;

use ax_application::ports::{CmsRepository, TenantResolver};
use ax_application::use_cases::cms::GetPublishedBySlug;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub cms_repo: Arc<dyn CmsRepository>,
    pub tenant_resolver: Arc<dyn TenantResolver>,
    pub get_published_by_slug: Arc<GetPublishedBySlug>,
}

impl AppState {
    /// Construct AppState из PgPool. Создаёт infrastructure adapters + use cases.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        let cms_repo: Arc<dyn CmsRepository> =
            Arc::new(ax_infrastructure::persistence::PgCmsRepository::new(pool.clone()));
        let tenant_resolver: Arc<dyn TenantResolver> =
            Arc::new(ax_infrastructure::PgTenantResolver::new(pool));
        let get_published_by_slug = Arc::new(GetPublishedBySlug::new(cms_repo.clone()));

        Self {
            cms_repo,
            tenant_resolver,
            get_published_by_slug,
        }
    }
}
