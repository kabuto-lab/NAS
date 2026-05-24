//! `PublishedPage` aggregate — domain root для public cms_page read.
//!
//! Invariants (audit §1.4, §2.2):
//! 1. `status == Published` (этот aggregate представляет только published — view enforce'ит)
//! 2. `published_at IS NOT NULL` (когда status='Published', SITE1 service гарантирует)
//! 3. `(tenant_id, slug, locale)` unique
//!
//! `PublishedPage::reconstitute` валидирует invariants при construction из DB row;
//! если row нарушает invariants — returns error (data corruption signal).

use ax_common::TenantId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::cms::{
    blocks::Block,
    value_objects::{PageLocale, PageSlug, PageStatus},
};

#[derive(Debug, Error)]
pub enum AggregateError {
    #[error("PublishedPage invariant violated: status must be Published, got {0:?}")]
    NotPublished(PageStatus),
    #[error("PublishedPage invariant violated: published_at must be Some when status=Published")]
    MissingPublishedAt,
}

/// Domain aggregate для опубликованной страницы.
///
/// Конструируется через `reconstitute` (из DB row, проверяет invariants) или через
/// builder (для тестов).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PublishedPage {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub slug: PageSlug,
    pub locale: PageLocale,
    pub title: String,
    pub body: Vec<Block>,
    pub status: PageStatus,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub cover_image_key: Option<String>,
    pub author_user_id: Option<Uuid>,
    pub published_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PublishedPage {
    /// Construct + validate invariants. Use when reading from DB.
    ///
    /// # Errors
    /// - `NotPublished` если status != Published
    /// - `MissingPublishedAt` если published_at is None
    #[allow(clippy::too_many_arguments)]
    pub fn reconstitute(
        id: Uuid,
        tenant_id: TenantId,
        slug: PageSlug,
        locale: PageLocale,
        title: String,
        body: Vec<Block>,
        status: PageStatus,
        meta_title: Option<String>,
        meta_description: Option<String>,
        cover_image_key: Option<String>,
        author_user_id: Option<Uuid>,
        published_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, AggregateError> {
        if status != PageStatus::Published {
            return Err(AggregateError::NotPublished(status));
        }
        let published_at = published_at.ok_or(AggregateError::MissingPublishedAt)?;

        Ok(Self {
            id,
            tenant_id,
            slug,
            locale,
            title,
            body,
            status,
            meta_title,
            meta_description,
            cover_image_key,
            author_user_id,
            published_at,
            created_at,
            updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_args() -> (
        Uuid,
        TenantId,
        PageSlug,
        PageLocale,
        String,
        Vec<Block>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<Uuid>,
        Option<DateTime<Utc>>,
        DateTime<Utc>,
        DateTime<Utc>,
    ) {
        (
            Uuid::nil(),
            TenantId::new(Uuid::nil()),
            PageSlug::parse("home").unwrap(),
            PageLocale::Ru,
            "Title".into(),
            vec![],
            None,
            None,
            None,
            None,
            Some(Utc::now()),
            Utc::now(),
            Utc::now(),
        )
    }

    #[test]
    fn reconstitute_accepts_published_with_timestamp() {
        let (id, t, s, l, title, body, mt, md, ck, ai, pa, ca, ua) = fixture_args();
        let p = PublishedPage::reconstitute(
            id,
            t,
            s,
            l,
            title,
            body,
            PageStatus::Published,
            mt,
            md,
            ck,
            ai,
            pa,
            ca,
            ua,
        );
        assert!(p.is_ok());
    }

    #[test]
    fn reconstitute_rejects_draft_status() {
        let (id, t, s, l, title, body, mt, md, ck, ai, pa, ca, ua) = fixture_args();
        let p = PublishedPage::reconstitute(
            id,
            t,
            s,
            l,
            title,
            body,
            PageStatus::Draft,
            mt,
            md,
            ck,
            ai,
            pa,
            ca,
            ua,
        );
        assert!(matches!(p, Err(AggregateError::NotPublished(_))));
    }

    #[test]
    fn reconstitute_rejects_archived_status() {
        let (id, t, s, l, title, body, mt, md, ck, ai, _pa, ca, ua) = fixture_args();
        let p = PublishedPage::reconstitute(
            id,
            t,
            s,
            l,
            title,
            body,
            PageStatus::Archived,
            mt,
            md,
            ck,
            ai,
            Some(Utc::now()),
            ca,
            ua,
        );
        assert!(matches!(p, Err(AggregateError::NotPublished(_))));
    }

    #[test]
    fn reconstitute_rejects_missing_published_at() {
        let (id, t, s, l, title, body, mt, md, ck, ai, _pa, ca, ua) = fixture_args();
        let p = PublishedPage::reconstitute(
            id,
            t,
            s,
            l,
            title,
            body,
            PageStatus::Published,
            mt,
            md,
            ck,
            ai,
            None,
            ca,
            ua,
        );
        assert!(matches!(p, Err(AggregateError::MissingPublishedAt)));
    }
}
