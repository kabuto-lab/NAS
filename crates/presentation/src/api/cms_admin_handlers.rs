//! CMS Admin API handlers — write-path для cms_pages.
//!
//! `POST /api/v1/cms/pages/admin` — create draft page.
//!
//! Auth chain (applied via middleware layers in router.rs):
//! 1. tenant_resolver → TenantContext в req.extensions
//! 2. request_id → RequestId в req.extensions
//! 3. auth middleware → AuthenticatedUser в req.extensions (or pass-through)
//! 4. handler: RequireTenant + RequireAuthenticated + capability check

use axum::{extract::State, http::StatusCode, Json};
use ax_common::{AppError, Capability, TenantId};
use ax_domain::cms::{
    blocks::Block, value_objects::PageStatus, DraftPage, NewDraftPage, PageLocale, PageSlug,
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::middleware::auth::RequireAuthenticated;
use crate::middleware::tenant_resolver::RequireTenant;

/// Input JSON для POST /api/v1/cms/pages/admin.
#[derive(Deserialize)]
pub struct CreatePageRequest {
    pub slug: String,
    #[serde(default = "default_locale")]
    pub locale: String,
    pub title: String,
    #[serde(default)]
    pub body: Vec<Block>,
    #[serde(default, rename = "metaTitle")]
    pub meta_title: Option<String>,
    #[serde(default, rename = "metaDescription")]
    pub meta_description: Option<String>,
    #[serde(default, rename = "coverImageKey")]
    pub cover_image_key: Option<String>,
}

fn default_locale() -> String {
    "ru".to_owned()
}

/// Response для admin create.
///
/// **NOT** identical to public CmsPageResponse — admin sees status + author_user_id.
/// Privacy: tenantId not exposed (consistent with public response shape).
#[derive(Serialize)]
pub struct AdminPageResponse {
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
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

impl From<DraftPage> for AdminPageResponse {
    fn from(d: DraftPage) -> Self {
        let _ = TenantId::new(d.tenant_id.0); // explicit: tenantId NOT exposed
        Self {
            id: d.id.to_string(),
            slug: d.slug.to_string(),
            locale: d.locale.as_str().to_owned(),
            title: d.title,
            body: d.body,
            status: match d.status {
                PageStatus::Draft => "draft",
                PageStatus::Published => "published",
                PageStatus::Archived => "archived",
            }
            .to_owned(),
            meta_title: d.meta_title,
            meta_description: d.meta_description,
            cover_image_key: d.cover_image_key,
            author_user_id: d.author_user_id.map(|u| u.to_string()),
            created_at: format_ts(d.created_at),
            updated_at: format_ts(d.updated_at),
        }
    }
}

fn format_ts(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Handler: `POST /api/v1/cms/pages/admin`.
///
/// Capability required: `Capability::PostsCreate` (`posts:create` key).
///
/// Returns 201 Created + JSON body. Errors:
/// - 401 NOT_AUTHENTICATED — no JWT
/// - 401 INVALID_TOKEN / TOKEN_EXPIRED — JWT verify fail
/// - 403 TENANT_OWNERSHIP_MISMATCH — JWT tenant ≠ X-Tenant-Slug tenant
/// - 403 MISSING_CAPABILITY — user lacks posts:create
/// - 400 VALIDATION_FAILED — invalid input (empty title, malformed slug, etc.)
/// - 409 CONFLICT — slug+locale already exists для tenant
#[axum::debug_handler]
pub async fn create_page_admin(
    State(state): State<AppState>,
    RequireTenant(ctx): RequireTenant,
    RequireAuthenticated(user): RequireAuthenticated,
    Json(payload): Json<CreatePageRequest>,
) -> Result<(StatusCode, Json<AdminPageResponse>), AppError> {
    state
        .capability_resolver
        .require(&ctx, user.user_id, Capability::PostsCreate)
        .await?;

    let slug = PageSlug::parse(&payload.slug)
        .map_err(|e| AppError::Validation(format!("invalid slug: {e}")))?;
    let locale = PageLocale::parse(&payload.locale)
        .map_err(|e| AppError::Validation(format!("invalid locale: {e}")))?;

    let draft = NewDraftPage {
        slug,
        locale,
        title: payload.title,
        body: payload.body,
        meta_title: payload.meta_title,
        meta_description: payload.meta_description,
        cover_image_key: payload.cover_image_key,
        author_user_id: Some(user.user_id.0),
    };

    draft
        .validate()
        .map_err(|e| AppError::Validation(format!("draft validation: {e}")))?;

    let saved = state.cms_admin_repo.insert_draft(&ctx, draft).await?;
    Ok((StatusCode::CREATED, Json(AdminPageResponse::from(saved))))
}
