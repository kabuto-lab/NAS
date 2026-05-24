//! `PgCmsRepository` — SQLx implementation of `CmsRepository`.
//!
//! **STATUS: skeleton (T11 in PLAN-001).** Использует `sqlx::query_as` (runtime
//! checked) вместо `sqlx::query_as!` (compile-time) до тех пор пока `.sqlx/`
//! offline metadata не сгенерирована командой `cargo sqlx prepare` против
//! реального Postgres. См. PLAN-001 Step 11 caveat.
//!
//! Реализация per ADR-001 D2 + audit §2.2 invariants:
//! - `with_tenant` обёртка (SET LOCAL → RLS POLICY срабатывает)
//! - Запрос к `cms_pages_v_active` (published-only filter at SQL level)
//! - 404 для cross-tenant, draft, archived, missing — same code `PAGE_NOT_FOUND`

use async_trait::async_trait;
use ax_application::ports::{CmsAdminRepository, CmsRepository};
use ax_common::{AppError, NotFoundDetail, TenantContext, TenantId};
use ax_domain::cms::{
    blocks::Block, value_objects::PageStatus, DraftPage, NewDraftPage, PageLocale, PageSlug,
    PublishedPage,
};
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::persistence::transaction::with_tenant;

/// PostgreSQL repository for cms_pages.
pub struct PgCmsRepository {
    pool: PgPool,
}

impl PgCmsRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Row struct для FromRow. Mirrors `cms_pages_v_active` view columns.
#[derive(Debug, FromRow)]
struct CmsPageRow {
    id: Uuid,
    tenant_id: Uuid,
    slug: String,
    locale: String,
    title: String,
    body: JsonValue,
    status: String,
    meta_title: Option<String>,
    meta_description: Option<String>,
    cover_image_key: Option<String>,
    author_user_id: Option<Uuid>,
    published_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[async_trait]
impl CmsRepository for PgCmsRepository {
    #[tracing::instrument(skip(self), fields(tenant_id = %ctx.tenant_id, slug = %slug, locale = %locale))]
    async fn find_published_by_slug(
        &self,
        ctx: &TenantContext,
        slug: &PageSlug,
        locale: PageLocale,
    ) -> Result<PublishedPage, AppError> {
        let slug_s = slug.as_str().to_string();
        let locale_s = locale.as_str().to_string();

        let row: Option<CmsPageRow> = with_tenant(&self.pool, ctx, move |tx| {
            Box::pin(async move {
                sqlx::query_as::<_, CmsPageRow>(
                    "SELECT id, tenant_id, slug, locale, title, body, status, \
                     meta_title, meta_description, cover_image_key, \
                     author_user_id, published_at, created_at, updated_at \
                     FROM cms_pages_v_active \
                     WHERE slug = $1 AND locale = $2 \
                     LIMIT 1",
                )
                .bind(&slug_s)
                .bind(&locale_s)
                .fetch_optional(&mut **tx)
                .await
                .map_err(|e| AppError::Database(e.to_string()))
            })
        })
        .await?;

        let row = row.ok_or_else(|| {
            AppError::NotFound(NotFoundDetail {
                code: "PAGE_NOT_FOUND",
                fields: serde_json::json!({
                    "slug": slug.as_str(),
                    "locale": locale.as_str(),
                }),
            })
        })?;

        map_row_to_aggregate(row).map_err(|e| {
            tracing::error!(error = %e, "DB row violates PublishedPage invariants — schema drift");
            AppError::Internal(eyre::eyre!("schema drift: {e}"))
        })
    }
}

fn map_row_to_aggregate(row: CmsPageRow) -> Result<PublishedPage, ax_domain::cms::aggregate::AggregateError> {
    let slug = PageSlug::parse(&row.slug)
        .map_err(|_| ax_domain::cms::aggregate::AggregateError::NotPublished(PageStatus::Draft))?;
    let locale = PageLocale::parse(&row.locale)
        .unwrap_or(PageLocale::Ru);
    let status = PageStatus::parse(&row.status)
        .unwrap_or(PageStatus::Draft);
    let body: Vec<Block> = serde_json::from_value(row.body).unwrap_or_default();

    PublishedPage::reconstitute(
        row.id,
        TenantId::new(row.tenant_id),
        slug,
        locale,
        row.title,
        body,
        status,
        row.meta_title,
        row.meta_description,
        row.cover_image_key,
        row.author_user_id,
        row.published_at,
        row.created_at,
        row.updated_at,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin write path — CmsAdminRepository impl
// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl CmsAdminRepository for PgCmsRepository {
    #[tracing::instrument(
        skip(self, draft),
        fields(tenant_id = %ctx.tenant_id, slug = %draft.slug, locale = %draft.locale)
    )]
    async fn insert_draft(
        &self,
        ctx: &TenantContext,
        draft: NewDraftPage,
    ) -> Result<DraftPage, AppError> {
        let tenant_uuid = ctx.tenant_id.0;
        let slug_s = draft.slug.as_str().to_owned();
        let locale_s = draft.locale.as_str().to_owned();
        let title = draft.title;
        let body_json = serde_json::to_value(&draft.body)
            .map_err(|e| AppError::BadRequest(format!("invalid body json: {e}")))?;
        let meta_title = draft.meta_title;
        let meta_description = draft.meta_description;
        let cover_image_key = draft.cover_image_key;
        let author_user_id = draft.author_user_id;

        let row: CmsPageRow = with_tenant(&self.pool, ctx, move |tx| {
            Box::pin(async move {
                sqlx::query_as::<_, CmsPageRow>(
                    "INSERT INTO cms_pages \
                     (tenant_id, slug, locale, title, body, status, \
                      meta_title, meta_description, cover_image_key, author_user_id) \
                     VALUES ($1, $2, $3, $4, $5, 'draft', $6, $7, $8, $9) \
                     RETURNING id, tenant_id, slug, locale, title, body, status, \
                               meta_title, meta_description, cover_image_key, \
                               author_user_id, published_at, created_at, updated_at",
                )
                .bind(tenant_uuid)
                .bind(&slug_s)
                .bind(&locale_s)
                .bind(&title)
                .bind(&body_json)
                .bind(&meta_title)
                .bind(&meta_description)
                .bind(&cover_image_key)
                .bind(author_user_id)
                .fetch_one(&mut **tx)
                .await
                .map_err(map_insert_error)
            })
        })
        .await?;

        map_row_to_draft(row)
            .map_err(|e| AppError::Internal(eyre::eyre!("draft reconstitute: {e}")))
    }
}

fn map_insert_error(e: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db_err) = &e {
        if let Some(code) = db_err.code() {
            // 23505 — unique_violation (slug already exists per (tenant, slug, locale))
            if code == "23505" {
                return AppError::Conflict("page slug already exists".to_owned());
            }
            // 42501 — insufficient_privilege (RLS WITH CHECK rejected)
            // 23514 — check_violation (catch-all CHECK constraint)
            if code == "42501" || code == "23514" {
                return AppError::TenantMismatch;
            }
        }
    }
    AppError::Database(e.to_string())
}

fn map_row_to_draft(row: CmsPageRow) -> Result<DraftPage, ax_domain::cms::draft::DraftError> {
    let slug = PageSlug::parse(&row.slug)
        .map_err(|_| ax_domain::cms::draft::DraftError::NotDraft(PageStatus::Archived))?;
    let locale = PageLocale::parse(&row.locale).unwrap_or(PageLocale::Ru);
    let status = PageStatus::parse(&row.status).unwrap_or(PageStatus::Archived);
    let body: Vec<Block> = serde_json::from_value(row.body).unwrap_or_default();

    DraftPage::reconstitute(
        row.id,
        TenantId::new(row.tenant_id),
        slug,
        locale,
        row.title,
        body,
        status,
        row.meta_title,
        row.meta_description,
        row.cover_image_key,
        row.author_user_id,
        row.published_at,
        row.created_at,
        row.updated_at,
    )
}
