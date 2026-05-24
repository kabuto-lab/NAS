//! `CmsRepository` port trait — interface к persistence layer для CMS.
//!
//! Implementation в `ax-infrastructure::persistence::PgCmsRepository`.
//! Phase A scope: read-only, single use case (`find_published_by_slug`).

use async_trait::async_trait;
use ax_common::{AppError, TenantContext};
use ax_domain::cms::{DraftPage, NewDraftPage, PageLocale, PageSlug, PublishedPage};

/// Repository contract для CMS pages.
///
/// **Invariants для implementers:**
///
/// 1. **Tenant isolation:** implementation MUST call `with_tenant(pool, ctx, |tx| ...)`
///    или эквивалент, чтобы RLS POLICY `rls_cms_pages_tenant_isolation` срабатывал
///    через `SET LOCAL app.current_tenant_id = $1`.
///
/// 2. **Published-only:** запросы идут к view `cms_pages_v_active` (filter
///    `WHERE status='published'` на SQL level), не к table `cms_pages` напрямую.
///
/// 3. **Cross-tenant attempts:** MUST вернуть `AppError::NotFound` с code
///    `PAGE_NOT_FOUND` — никогда `Forbidden`, не utечка существования (audit §2.2).
///
/// 4. **Locale fallback:** caller обязан передавать explicit `locale` (use case
///    делает `?? Ru` default раньше).
#[async_trait]
pub trait CmsRepository: Send + Sync {
    /// Найти опубликованную страницу по (tenant, slug, locale).
    ///
    /// # Errors
    /// - `AppError::NotFound` — страница не существует, draft, archived, чужой тенант
    /// - `AppError::Database` — SQL ошибка
    /// - `AppError::Internal` — invariant violation в DB row → aggregate mapping
    async fn find_published_by_slug(
        &self,
        ctx: &TenantContext,
        slug: &PageSlug,
        locale: PageLocale,
    ) -> Result<PublishedPage, AppError>;
}

/// Admin write-path for CMS pages.
///
/// Phase A scope: single method (insert_draft). Update/publish/delete — follow-up RFCs.
///
/// **Invariants:**
/// 1. **Tenant isolation:** implementation MUST run inside `with_tenant(...)` so that
///    RLS POLICY `rls_cms_pages_tenant_isolation` validates `WITH CHECK` clause on
///    INSERT. Spoofed `tenant_id` in payload — DB rejects (postgres error 23514).
/// 2. **Status invariant:** new row inserted with `status='draft'` + `published_at IS NULL`.
/// 3. **Uniqueness:** if `(tenant_id, slug, locale)` already exists — return
///    `AppError::Conflict("page slug already exists")`.
#[async_trait]
pub trait CmsAdminRepository: Send + Sync {
    /// Insert a new draft page. Returns reconstituted `DraftPage`.
    ///
    /// # Errors
    /// - `AppError::Conflict` — unique constraint violation
    /// - `AppError::TenantMismatch` — defensive (RLS WITH CHECK already caught)
    /// - `AppError::Database` — other SQL errors
    async fn insert_draft(
        &self,
        ctx: &TenantContext,
        draft: NewDraftPage,
    ) -> Result<DraftPage, AppError>;
}
