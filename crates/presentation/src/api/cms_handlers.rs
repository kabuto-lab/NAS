//! CMS API handlers — public read-path.
//!
//! `GET /api/v1/cms/pages/public/by-slug/{slug}?locale=ru` per audit §3.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use ax_common::{AppError, TenantId};
use ax_domain::cms::{
    blocks::Block, value_objects::PageStatus, PageLocale, PageSlug, PublishedPage,
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::middleware::tenant_resolver::RequireTenant;

#[derive(Deserialize)]
pub struct LocaleQuery {
    locale: Option<String>,
}

/// JSON response per audit §4.2. **Critical:** field order matches SITE1 exactly,
/// `tenantId` absent (privacy), Option fields сериализуются как `null`.
#[derive(Serialize)]
pub struct CmsPageResponse {
    pub id: String,
    pub slug: String,
    pub locale: String,
    pub title: String,
    pub body: Vec<Block>,
    pub status: String,
    #[serde(rename = "metaTitle")]
    pub meta_title: Option<String>,
    #[serde(rename = "metaDescription")]
    pub meta_description: Option<String>,
    #[serde(rename = "coverImageKey")]
    pub cover_image_key: Option<String>,
    #[serde(rename = "authorUserId")]
    pub author_user_id: Option<String>,
    #[serde(rename = "publishedAt")]
    pub published_at: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

impl From<PublishedPage> for CmsPageResponse {
    fn from(page: PublishedPage) -> Self {
        let _ = TenantId::new(page.tenant_id.0); // explicit: tenant_id НЕ exposed in JSON
        Self {
            id: page.id.to_string(),
            slug: page.slug.to_string(),
            locale: page.locale.as_str().to_string(),
            title: page.title,
            body: page.body,
            status: match page.status {
                PageStatus::Draft => "draft",
                PageStatus::Published => "published",
                PageStatus::Archived => "archived",
            }
            .to_string(),
            meta_title: page.meta_title,
            meta_description: page.meta_description,
            cover_image_key: page.cover_image_key,
            author_user_id: page.author_user_id.map(|u| u.to_string()),
            published_at: Some(format_ts(page.published_at)),
            created_at: format_ts(page.created_at),
            updated_at: format_ts(page.updated_at),
        }
    }
}

/// ISO8601 RFC3339 с Z-suffix per audit §4.2.
fn format_ts(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Handler: `GET /api/v1/cms/pages/public/by-slug/{slug}?locale=ru`.
#[axum::debug_handler]
pub async fn get_published_by_slug(
    State(state): State<AppState>,
    RequireTenant(ctx): RequireTenant,
    Path(slug): Path<String>,
    Query(q): Query<LocaleQuery>,
) -> Result<Json<CmsPageResponse>, AppError> {
    let slug_parsed = PageSlug::parse(&slug)
        .map_err(|_| AppError::NotFound(ax_common::NotFoundDetail {
            code: "PAGE_NOT_FOUND",
            fields: serde_json::json!({ "slug": slug, "locale": q.locale }),
        }))?;
    let locale = q
        .locale
        .as_deref()
        .map(PageLocale::parse)
        .transpose()
        .map_err(|_| AppError::NotFound(ax_common::NotFoundDetail {
            code: "PAGE_NOT_FOUND",
            fields: serde_json::json!({ "slug": slug, "locale": q.locale }),
        }))?
        .unwrap_or_default();

    let page = state
        .get_published_by_slug
        .execute(&ctx, &slug_parsed, locale)
        .await?;

    Ok(Json(CmsPageResponse::from(page)))
}
