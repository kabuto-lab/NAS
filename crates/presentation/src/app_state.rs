//! `AppState` — DI container для Axum handlers.
//!
//! Содержит Arc'ed traits для repos/resolvers + use cases (которые сами Arc-обёрнуты).

use std::sync::Arc;

use ax_application::ports::{CmsAdminRepository, CmsRepository, TenantResolver, UserRepository};
use ax_application::use_cases::auth::CapabilityResolver;
use ax_application::use_cases::cms::GetPublishedBySlug;
use ax_infrastructure::JwtVerifier;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub cms_repo: Arc<dyn CmsRepository>,
    pub cms_admin_repo: Arc<dyn CmsAdminRepository>,
    pub tenant_resolver: Arc<dyn TenantResolver>,
    pub get_published_by_slug: Arc<GetPublishedBySlug>,
    pub jwt_verifier: Arc<JwtVerifier>,
    pub user_repo: Arc<dyn UserRepository>,
    pub capability_resolver: Arc<CapabilityResolver>,
}

impl AppState {
    /// Construct AppState из PgPool + JWT secret.
    ///
    /// **CRITICAL:** `jwt_secret` должен быть тот же что у SITE1 (см. ADR-002 D1).
    #[must_use]
    pub fn new(pool: PgPool, jwt_secret: &str) -> Self {
        let cms_pg = Arc::new(
            ax_infrastructure::persistence::PgCmsRepository::new(pool.clone()),
        );
        let cms_repo: Arc<dyn CmsRepository> = cms_pg.clone();
        let cms_admin_repo: Arc<dyn CmsAdminRepository> = cms_pg;
        let tenant_resolver: Arc<dyn TenantResolver> = Arc::new(
            ax_infrastructure::PgTenantResolver::new(pool.clone()),
        );
        let get_published_by_slug = Arc::new(GetPublishedBySlug::new(cms_repo.clone()));

        let user_repo: Arc<dyn UserRepository> = Arc::new(
            ax_infrastructure::persistence::PgUserRepository::new(pool),
        );
        let capability_resolver = Arc::new(CapabilityResolver::new(user_repo.clone()));

        let jwt_verifier = Arc::new(JwtVerifier::new(jwt_secret));

        Self {
            cms_repo,
            cms_admin_repo,
            tenant_resolver,
            get_published_by_slug,
            jwt_verifier,
            user_repo,
            capability_resolver,
        }
    }
}
