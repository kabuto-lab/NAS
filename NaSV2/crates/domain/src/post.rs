//! Post aggregate · ENTITY §7 domain model.
//!
//! ## State machine (`PostStatus`)
//!
//! ```text
//!   Draft ─────► Scheduled ─────► Published ─────► Archived
//!     │                ▲              ▲                │
//!     └────────────────┴──────────────┘                │
//!                                                      ▼
//!                                          Archived is terminal
//! ```
//!
//! - `Draft` can fast-track straight to `Published` or to `Scheduled`.
//! - `Scheduled` is a future-publish placeholder; on time-trigger the
//!   caller transitions it to `Published`. `Scheduled` may also un-
//!   schedule back to `Draft` or jump to `Archived`.
//! - `Published` can be `Archived` (soft delete).
//! - `Archived` is terminal — un-archiving needs a deliberate workflow
//!   not modelled here (a `Restore` command would create a new Post).

use chrono::{DateTime, Utc};
use nas2_common::{AppError, PostId, SiteId, TenantId};
use serde::{Deserialize, Serialize};

use crate::block::Block;
use crate::site::validate_slug;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Post {
    pub id: PostId,
    pub site_id: SiteId,
    pub tenant_id: TenantId,
    pub slug: PostSlug,
    pub title: String,
    pub status: PostStatus,
    /// Set ⇔ `status` has ever been `Published`. Enforced by
    /// `transition_to`; preserved across `Published → Archived`.
    pub published_at: Option<DateTime<Utc>>,
    pub blocks: Vec<Block>,
    pub custom_type: Option<CustomPostType>,
}

impl Post {
    /// Apply a status transition. Returns `AppError::Conflict` if the
    /// transition is forbidden by the FSM.
    pub fn transition_to(
        &mut self,
        target: PostStatus,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        if !self.status.can_transition_to(target) {
            return Err(AppError::Conflict(format!(
                "post {id} cannot transition {from:?} → {target:?}",
                id = self.id,
                from = self.status
            )));
        }
        if matches!(target, PostStatus::Published) && self.published_at.is_none() {
            self.published_at = Some(now);
        }
        self.status = target;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostStatus {
    Draft,
    Scheduled,
    Published,
    Archived,
}

impl PostStatus {
    #[must_use]
    pub const fn can_transition_to(self, target: Self) -> bool {
        use PostStatus::{Archived, Draft, Published, Scheduled};
        matches!(
            (self, target),
            (Draft, Scheduled | Published | Archived)
                | (Scheduled, Draft | Published | Archived)
                | (Published, Archived) // Archived → * forbidden (terminal).
        )
    }
}

/// Per-site unique post slug. Same syntax rules as `SiteSlug`, but the
/// upper bound is 80 chars (posts often have longer URLs).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct PostSlug(String);

impl PostSlug {
    pub fn try_new(raw: &str) -> Result<Self, AppError> {
        validate_slug(raw, 3, 80)
            .map(|()| Self(raw.to_owned()))
            .map_err(AppError::Validation)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Placeholder for extension-defined post types (e.g. `event`,
/// `case-study`). No validation today; the Extension API (M9+) will
/// hold the canonical registry of valid type names.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct CustomPostType(pub String);

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_methods
)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    fn draft_post() -> Post {
        Post {
            id: PostId::new_v4(),
            site_id: SiteId::new_v4(),
            tenant_id: TenantId::new_v4(),
            slug: PostSlug::try_new("hello-world").unwrap(),
            title: "Hello".into(),
            status: PostStatus::Draft,
            published_at: None,
            blocks: vec![Block::Placeholder],
            custom_type: None,
        }
    }

    #[test]
    fn fsm_draft_to_published_allowed() {
        assert!(PostStatus::Draft.can_transition_to(PostStatus::Published));
    }

    #[test]
    fn fsm_archived_to_published_forbidden() {
        assert!(!PostStatus::Archived.can_transition_to(PostStatus::Published));
    }

    #[test]
    fn fsm_published_to_scheduled_forbidden() {
        assert!(!PostStatus::Published.can_transition_to(PostStatus::Scheduled));
    }

    #[test]
    fn fsm_scheduled_to_draft_allowed() {
        assert!(PostStatus::Scheduled.can_transition_to(PostStatus::Draft));
    }

    #[test]
    fn transition_sets_published_at_on_first_publish() {
        let mut p = draft_post();
        let t = now();
        p.transition_to(PostStatus::Published, t).unwrap();
        assert_eq!(p.published_at, Some(t));
        assert_eq!(p.status, PostStatus::Published);
    }

    #[test]
    fn transition_keeps_published_at_on_archive() {
        let mut p = draft_post();
        let t = now();
        p.transition_to(PostStatus::Published, t).unwrap();
        let t2 = t + chrono::Duration::days(1);
        p.transition_to(PostStatus::Archived, t2).unwrap();
        assert_eq!(p.published_at, Some(t), "publish time preserved");
        assert_eq!(p.status, PostStatus::Archived);
    }

    #[test]
    fn transition_returns_conflict_on_forbidden() {
        let mut p = draft_post();
        p.status = PostStatus::Archived;
        let err = p.transition_to(PostStatus::Published, now()).unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
        assert!(err.to_string().contains("cannot transition"));
    }

    #[test]
    fn post_slug_valid_examples() {
        assert!(PostSlug::try_new("hello").is_ok());
        assert!(PostSlug::try_new("hello-world").is_ok());
        assert!(PostSlug::try_new("a-1-b").is_ok());
    }

    #[test]
    fn post_slug_allows_up_to_80() {
        let s80 = "a".repeat(80);
        assert!(PostSlug::try_new(&s80).is_ok());
        let s81 = "a".repeat(81);
        assert!(PostSlug::try_new(&s81).is_err());
    }

    #[test]
    fn custom_post_type_serde_transparent() {
        let t = CustomPostType("event".into());
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r#""event""#);
        let back: CustomPostType = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn post_serde_roundtrip() {
        let p = draft_post();
        let json = serde_json::to_string(&p).unwrap();
        let back: Post = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn post_status_serde_snake_case() {
        let s = serde_json::to_string(&PostStatus::Draft).unwrap();
        assert_eq!(s, r#""draft""#);
        let s = serde_json::to_string(&PostStatus::Scheduled).unwrap();
        assert_eq!(s, r#""scheduled""#);
    }

    #[test]
    fn fsm_self_transition_forbidden() {
        // No self-loops in the FSM — Draft → Draft is forbidden.
        assert!(!PostStatus::Draft.can_transition_to(PostStatus::Draft));
        assert!(!PostStatus::Published.can_transition_to(PostStatus::Published));
    }
}
