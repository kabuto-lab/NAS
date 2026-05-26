//! Persistence port for `Post` aggregates · ENTITY §2 (hex layer 2).
//!
//! Adapters implement this trait in `crates/infrastructure` (e.g.
//! `PgPostRepository`). Use cases generic over `R: PostRepository`
//! (static dispatch, §8.3) OR via `Arc<dyn PostRepository>`
//! (presentation `AppState` convenience).
//!
//! ## TLA layers
//!
//! - **L1 Correctness** — every method requires `(tenant, site)` ID
//!   pair so the adapter can pin RLS via `set_config('app.tenant_id', $1)`
//!   in the same transaction as the query (§3.5).
//! - **L2 Performance** — `list_published` returns `Page<Post>` in one
//!   DB roundtrip; no N+1 by contract.
//! - **L3 Scalability** — repo is stateless; clones share the underlying
//!   pool (`Arc` internally in the adapter).
//! - **L4 Operability** — adapter implementations MUST instrument each
//!   method with `tracing::instrument` carrying `tenant_id`.

// `mockall::automock` (test-only) expands to code that internally uses
// `std::sync::Mutex`. ENTITY §8.7 disallows it in production code; in
// test builds it's fine. Scope the allow to `cfg(test)` so production
// compiles never see the relaxation.
#![cfg_attr(test, allow(clippy::disallowed_types))]

use async_trait::async_trait;
use nas2_common::{AppError, Page, PostId, SiteId, TenantId};
use nas2_domain::{Post, PostSlug};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PostRepository: Send + Sync {
    async fn find_by_slug(
        &self,
        tenant: TenantId,
        site: SiteId,
        slug: &PostSlug,
    ) -> Result<Option<Post>, AppError>;

    async fn find_by_id(
        &self,
        tenant: TenantId,
        id: PostId,
    ) -> Result<Option<Post>, AppError>;

    async fn list_published(
        &self,
        tenant: TenantId,
        site: SiteId,
        page: u32,
        per_page: u32,
    ) -> Result<Page<Post>, AppError>;

    async fn insert(&self, post: &Post) -> Result<(), AppError>;

    async fn update(&self, post: &Post) -> Result<(), AppError>;
}

// Blanket forwarder: `Arc<T: PostRepository + ?Sized>` IS a
// `PostRepository`. Enables `GetPublishedPageBySlug::new(Arc<dyn ...>)`
// at the handler boundary while keeping the use case generic.
#[async_trait]
impl<T: PostRepository + ?Sized> PostRepository for std::sync::Arc<T> {
    async fn find_by_slug(
        &self,
        tenant: TenantId,
        site: SiteId,
        slug: &PostSlug,
    ) -> Result<Option<Post>, AppError> {
        (**self).find_by_slug(tenant, site, slug).await
    }

    async fn find_by_id(
        &self,
        tenant: TenantId,
        id: PostId,
    ) -> Result<Option<Post>, AppError> {
        (**self).find_by_id(tenant, id).await
    }

    async fn list_published(
        &self,
        tenant: TenantId,
        site: SiteId,
        page: u32,
        per_page: u32,
    ) -> Result<Page<Post>, AppError> {
        (**self)
            .list_published(tenant, site, page, per_page)
            .await
    }

    async fn insert(&self, post: &Post) -> Result<(), AppError> {
        (**self).insert(post).await
    }

    async fn update(&self, post: &Post) -> Result<(), AppError> {
        (**self).update(post).await
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// Compile-time assertion: trait is dyn-safe so `AppState` can store
    /// `Arc<dyn PostRepository>`.
    #[allow(dead_code)]
    fn _assert_object_safe(_: &dyn PostRepository) {}

    #[tokio::test]
    async fn mock_find_by_slug_returns_configured_value() {
        let mut mock = MockPostRepository::new();
        mock.expect_find_by_slug()
            .returning(|_, _, _| Ok(None));
        let res = mock
            .find_by_slug(
                TenantId::new_v4(),
                SiteId::new_v4(),
                &PostSlug::try_new("hello").unwrap(),
            )
            .await;
        assert!(matches!(res, Ok(None)));
    }
}
