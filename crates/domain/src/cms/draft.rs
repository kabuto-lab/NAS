//! `DraftPage` aggregate + `NewDraftPage` input DTO для admin create flow.
//!
//! Spec: ADR-002 §D7 (RLS WITH CHECK), PLAN-002 Phase 4.
//!
//! Draft pages — те которые в базе со status='draft' и published_at IS NULL.
//! Mirror SITE1 invariant: новые pages создаются как draft, publish — separate flow.

use ax_common::TenantId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::cms::{
    blocks::Block,
    value_objects::{PageLocale, PageSlug, PageStatus},
};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DraftError {
    #[error("DraftPage invariant violated: status must be Draft, got {0:?}")]
    NotDraft(PageStatus),
    #[error("DraftPage invariant violated: published_at must be None for status=Draft")]
    UnexpectedPublishedAt,
    #[error("title empty")]
    TitleEmpty,
    #[error("title too long ({0} > 500)")]
    TitleTooLong(usize),
}

/// Input DTO для admin create. Sanitized + validated в use case before INSERT.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewDraftPage {
    pub slug: PageSlug,
    pub locale: PageLocale,
    pub title: String,
    pub body: Vec<Block>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub cover_image_key: Option<String>,
    pub author_user_id: Option<Uuid>,
}

impl NewDraftPage {
    /// Validate input invariants (title non-empty, length).
    ///
    /// # Errors
    /// - `TitleEmpty` if title is empty after trim
    /// - `TitleTooLong` if title > 500 chars
    pub fn validate(&self) -> Result<(), DraftError> {
        let t = self.title.trim();
        if t.is_empty() {
            return Err(DraftError::TitleEmpty);
        }
        if self.title.len() > 500 {
            return Err(DraftError::TitleTooLong(self.title.len()));
        }
        Ok(())
    }
}

/// Persisted draft page — what repository returns after INSERT.
#[derive(Clone, Debug, PartialEq)]
pub struct DraftPage {
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DraftPage {
    /// Construct + validate. Used after `INSERT ... RETURNING *`.
    ///
    /// # Errors
    /// - `NotDraft` если status != Draft
    /// - `UnexpectedPublishedAt` если published_at Some
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
    ) -> Result<Self, DraftError> {
        if status != PageStatus::Draft {
            return Err(DraftError::NotDraft(status));
        }
        if published_at.is_some() {
            return Err(DraftError::UnexpectedPublishedAt);
        }
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
            created_at,
            updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_draft() -> NewDraftPage {
        NewDraftPage {
            slug: PageSlug::parse("draft-test").unwrap(),
            locale: PageLocale::Ru,
            title: "Title".into(),
            body: vec![],
            meta_title: None,
            meta_description: None,
            cover_image_key: None,
            author_user_id: None,
        }
    }

    #[test]
    fn new_draft_validate_accepts_normal_title() {
        assert!(new_draft().validate().is_ok());
    }

    #[test]
    fn new_draft_validate_rejects_empty_title() {
        let mut d = new_draft();
        d.title = String::new();
        assert_eq!(d.validate().unwrap_err(), DraftError::TitleEmpty);
    }

    #[test]
    fn new_draft_validate_rejects_whitespace_only_title() {
        let mut d = new_draft();
        d.title = "   ".into();
        assert_eq!(d.validate().unwrap_err(), DraftError::TitleEmpty);
    }

    #[test]
    fn new_draft_validate_rejects_too_long_title() {
        let mut d = new_draft();
        d.title = "x".repeat(501);
        assert!(matches!(d.validate().unwrap_err(), DraftError::TitleTooLong(_)));
    }

    #[test]
    fn reconstitute_accepts_valid_draft() {
        let d = DraftPage::reconstitute(
            Uuid::new_v4(),
            TenantId::new(Uuid::new_v4()),
            PageSlug::parse("draft").unwrap(),
            PageLocale::Ru,
            "T".into(),
            vec![],
            PageStatus::Draft,
            None,
            None,
            None,
            None,
            None,
            Utc::now(),
            Utc::now(),
        );
        assert!(d.is_ok());
    }

    #[test]
    fn reconstitute_rejects_published_status() {
        let d = DraftPage::reconstitute(
            Uuid::new_v4(),
            TenantId::new(Uuid::new_v4()),
            PageSlug::parse("draft").unwrap(),
            PageLocale::Ru,
            "T".into(),
            vec![],
            PageStatus::Published,
            None,
            None,
            None,
            None,
            None,
            Utc::now(),
            Utc::now(),
        );
        assert!(matches!(d.unwrap_err(), DraftError::NotDraft(_)));
    }

    #[test]
    fn reconstitute_rejects_published_at_some_for_draft() {
        let d = DraftPage::reconstitute(
            Uuid::new_v4(),
            TenantId::new(Uuid::new_v4()),
            PageSlug::parse("draft").unwrap(),
            PageLocale::Ru,
            "T".into(),
            vec![],
            PageStatus::Draft,
            None,
            None,
            None,
            None,
            Some(Utc::now()),
            Utc::now(),
            Utc::now(),
        );
        assert_eq!(d.unwrap_err(), DraftError::UnexpectedPublishedAt);
    }
}
