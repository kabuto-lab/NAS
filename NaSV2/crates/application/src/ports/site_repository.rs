//! Persistence port for `Site` aggregates · ENTITY §2 (hex layer 2).

// mockall::automock (test-only) uses std::sync::Mutex internally —
// disallowed in production code (ENTITY §8.7) but acceptable in tests.
#![cfg_attr(test, allow(clippy::disallowed_types))]

use async_trait::async_trait;
use nas2_common::{AppError, SiteId, TenantId};
use nas2_domain::{Site, SiteSlug};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SiteRepository: Send + Sync {
    async fn find_by_id(
        &self,
        tenant: TenantId,
        id: SiteId,
    ) -> Result<Option<Site>, AppError>;

    async fn find_by_slug(
        &self,
        tenant: TenantId,
        slug: &SiteSlug,
    ) -> Result<Option<Site>, AppError>;

    async fn list_for_tenant(&self, tenant: TenantId) -> Result<Vec<Site>, AppError>;

    async fn insert(&self, site: &Site) -> Result<(), AppError>;
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn _assert_object_safe(_: &dyn SiteRepository) {}
}
