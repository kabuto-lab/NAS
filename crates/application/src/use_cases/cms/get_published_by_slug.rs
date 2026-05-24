//! `GetPublishedBySlug` use case — thin coordinator над `CmsRepository`.
//!
//! Спецификация: ADR-001 D2 + audit §2.2. **Thin by design** — нет
//! бизнес-логики кроме делегации в repo. FUTURE markers для:
//! - cache layer (Phase B, moka per-tenant)
//! - feature flag checks (Phase B+)
//! - A/B variant routing (через `ax-application/ccd` per ENTITY §25)

use std::sync::Arc;

use ax_common::{AppError, TenantContext};
use ax_domain::cms::{PageLocale, PageSlug, PublishedPage};

use crate::ports::CmsRepository;

/// Use case для public read-path: `GET /api/v1/cms/pages/public/by-slug/:slug`.
pub struct GetPublishedBySlug {
    repo: Arc<dyn CmsRepository>,
}

impl GetPublishedBySlug {
    #[must_use]
    pub fn new(repo: Arc<dyn CmsRepository>) -> Self {
        Self { repo }
    }

    /// Execute use case.
    ///
    /// # Errors
    /// Pass-through `AppError` из `CmsRepository::find_published_by_slug`.
    #[tracing::instrument(
        skip(self),
        fields(
            tenant_id = %ctx.tenant_id,
            tenant_slug = %ctx.tenant_slug,
            cms.page.slug = %slug,
            cms.page.locale = %locale,
        ),
    )]
    pub async fn execute(
        &self,
        ctx: &TenantContext,
        slug: &PageSlug,
        locale: PageLocale,
    ) -> Result<PublishedPage, AppError> {
        // FUTURE: cache lookup в tenant-scoped moka here
        // FUTURE: CCD variant resolution per ENTITY §25
        // FUTURE: feature flag check
        self.repo.find_published_by_slug(ctx, slug, locale).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ax_common::{ids::RequestId, TenantId, TenantStatus};
    use ax_domain::cms::{PageStatus, PublishedPage};
    use chrono::Utc;
    use std::sync::Arc;
    use uuid::Uuid;

    struct StubRepo {
        result: Result<PublishedPage, AppError>,
    }

    #[async_trait::async_trait]
    impl CmsRepository for StubRepo {
        async fn find_published_by_slug(
            &self,
            _ctx: &TenantContext,
            _slug: &PageSlug,
            _locale: PageLocale,
        ) -> Result<PublishedPage, AppError> {
            match &self.result {
                Ok(p) => Ok(p.clone()),
                Err(e) => match e {
                    AppError::NotFound(_) => Err(AppError::NotFound(ax_common::NotFoundDetail {
                        code: "PAGE_NOT_FOUND",
                        fields: serde_json::json!({}),
                    })),
                    _ => Err(AppError::Database("stub".into())),
                },
            }
        }
    }

    fn make_ctx() -> TenantContext {
        TenantContext {
            tenant_id: TenantId::new(Uuid::nil()),
            tenant_slug: std::sync::Arc::from("test"),
            status: TenantStatus::Active,
            request_id: RequestId::new(),
            user_id: None,
        }
    }

    fn make_page() -> PublishedPage {
        PublishedPage::reconstitute(
            Uuid::nil(),
            TenantId::new(Uuid::nil()),
            PageSlug::parse("home").unwrap(),
            PageLocale::Ru,
            "Home".into(),
            vec![],
            PageStatus::Published,
            None,
            None,
            None,
            None,
            Some(Utc::now()),
            Utc::now(),
            Utc::now(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn delegates_to_repo_on_success() {
        let page = make_page();
        let repo = Arc::new(StubRepo {
            result: Ok(page.clone()),
        });
        let uc = GetPublishedBySlug::new(repo);

        let result = uc
            .execute(&make_ctx(), &PageSlug::parse("home").unwrap(), PageLocale::Ru)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, page.id);
    }

    #[tokio::test]
    async fn passes_through_not_found() {
        let repo = Arc::new(StubRepo {
            result: Err(AppError::NotFound(ax_common::NotFoundDetail {
                code: "PAGE_NOT_FOUND",
                fields: serde_json::json!({}),
            })),
        });
        let uc = GetPublishedBySlug::new(repo);
        let result = uc
            .execute(&make_ctx(), &PageSlug::parse("home").unwrap(), PageLocale::Ru)
            .await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
