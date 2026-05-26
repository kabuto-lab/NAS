//! Use case: fetch a published page by site + slug, gated by
//! `Capability::CmsPageRead`.
//!
//! ## TLA layers
//!
//! - **L1 Correctness** — capability gate FIRST, then repo lookup. 404
//!   vs 403 distinction explicit: capability fail → Forbidden; repo
//!   None OR non-Published status → NotFound. *Critical:* draft posts
//!   return NotFound (not Forbidden) so admin/non-admin response shapes
//!   are indistinguishable — prevents "draft enumeration" via timing
//!   or status differences.
//! - **L2 Performance** — one DB roundtrip (`repo.find_by_slug`).
//!   Capability check is in-memory `BTreeSet::contains` — O(log n).
//! - **L3 Scalability** — generic over `R: PostRepository`; static
//!   dispatch keeps the call site inline-friendly. AppState can choose
//!   to wrap in `Arc<dyn PostRepository>` for storage convenience.
//! - **L4 Operability** — the caller (handler) instruments the span;
//!   this use case stays free of `tracing` so it remains testable
//!   without a subscriber.

use nas2_common::{AppError, SiteId, TenantId};
use nas2_domain::{Capability, CapabilitySet, Post, PostSlug, PostStatus};

use crate::ports::PostRepository;

/// Generic over the repository for static dispatch.
///
/// Constructed by handlers per request OR cached at `AppState` level —
/// the type is cheap to clone (one ref + zero state) when `R` is.
pub struct GetPublishedPageBySlug<R> {
    repo: R,
}

impl<R: PostRepository> GetPublishedPageBySlug<R> {
    pub const fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        tenant: TenantId,
        site: SiteId,
        slug: &PostSlug,
        caps: &CapabilitySet,
    ) -> Result<Post, AppError> {
        // 1. Capability gate.
        if !caps.contains(Capability::CmsPageRead) {
            return Err(AppError::Forbidden(
                Capability::CmsPageRead.as_str().to_owned(),
            ));
        }

        // 2. Repository lookup.
        let maybe = self.repo.find_by_slug(tenant, site, slug).await?;

        // 3. Published-status filter — Draft / Scheduled / Archived all
        //    surface as NotFound to prevent enumeration via response-
        //    shape differences. The repo call always happens (constant
        //    work) regardless of status — denies side-channel timing
        //    inference of which slugs exist as drafts.
        match maybe {
            Some(p) if p.status == PostStatus::Published => Ok(p),
            _ => Err(AppError::NotFound(format!(
                "page slug={slug} on site={site}",
                slug = slug.as_str()
            ))),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use chrono::Utc;
    use nas2_common::PostId;
    use nas2_domain::{Capability, CapabilitySet, Post, PostSlug, PostStatus};

    use super::*;
    use crate::ports::MockPostRepository;

    fn caps_with_read() -> CapabilitySet {
        CapabilitySet::from_iter([Capability::CmsPageRead])
    }

    fn caps_empty() -> CapabilitySet {
        CapabilitySet::new()
    }

    fn sample_post(status: PostStatus) -> Post {
        Post {
            id: PostId::new_v4(),
            site_id: SiteId::new_v4(),
            tenant_id: TenantId::new_v4(),
            slug: PostSlug::try_new("hello").unwrap(),
            title: "Hello".into(),
            status,
            published_at: matches!(status, PostStatus::Published).then(Utc::now),
            blocks: Vec::new(),
            custom_type: None,
        }
    }

    #[tokio::test]
    async fn returns_post_when_published_and_caps_ok() {
        let post = sample_post(PostStatus::Published);
        let expected_id = post.id;
        let mut repo = MockPostRepository::new();
        repo.expect_find_by_slug()
            .returning(move |_, _, _| Ok(Some(post.clone())));
        let uc = GetPublishedPageBySlug::new(repo);
        let got = uc
            .execute(
                TenantId::new_v4(),
                SiteId::new_v4(),
                &PostSlug::try_new("hello").unwrap(),
                &caps_with_read(),
            )
            .await
            .unwrap();
        assert_eq!(got.id, expected_id);
    }

    #[tokio::test]
    async fn returns_forbidden_when_caps_missing() {
        let repo = MockPostRepository::new(); // no expectations → must not be called
        let uc = GetPublishedPageBySlug::new(repo);
        let err = uc
            .execute(
                TenantId::new_v4(),
                SiteId::new_v4(),
                &PostSlug::try_new("hello").unwrap(),
                &caps_empty(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn returns_not_found_when_repo_returns_none() {
        let mut repo = MockPostRepository::new();
        repo.expect_find_by_slug().returning(|_, _, _| Ok(None));
        let uc = GetPublishedPageBySlug::new(repo);
        let err = uc
            .execute(
                TenantId::new_v4(),
                SiteId::new_v4(),
                &PostSlug::try_new("hello").unwrap(),
                &caps_with_read(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn returns_not_found_when_post_in_draft_state() {
        let post = sample_post(PostStatus::Draft);
        let mut repo = MockPostRepository::new();
        repo.expect_find_by_slug()
            .returning(move |_, _, _| Ok(Some(post.clone())));
        let uc = GetPublishedPageBySlug::new(repo);
        let err = uc
            .execute(
                TenantId::new_v4(),
                SiteId::new_v4(),
                &PostSlug::try_new("hello").unwrap(),
                &caps_with_read(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn propagates_repo_error() {
        let mut repo = MockPostRepository::new();
        repo.expect_find_by_slug()
            .returning(|_, _, _| Err(AppError::Internal));
        let uc = GetPublishedPageBySlug::new(repo);
        let err = uc
            .execute(
                TenantId::new_v4(),
                SiteId::new_v4(),
                &PostSlug::try_new("hello").unwrap(),
                &caps_with_read(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Internal));
    }
}
