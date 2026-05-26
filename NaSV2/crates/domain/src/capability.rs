//! Authorization capabilities · ENTITY §14.
//!
//! `Capability` is an *exhaustive typed enum* — NOT `&'static str` — so
//! that handlers cannot misspell a capability and only compile-time-known
//! capabilities can be checked. Extension capabilities (M9+) get their
//! own `ExtensionCapability` enum kept separate.
//!
//! `CapabilitySet` wraps `BTreeSet<Capability>` (NOT `HashSet`) so that
//! iteration order is deterministic — required for the cache-key
//! composition contract in ENTITY §3.9.1.

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

use nas2_common::AppError;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    CmsPageRead,
    CmsPagePublish,
    CmsPageDraft,
    MediaUpload,
    ManageUsers,
    ManageSite,
}

impl Capability {
    /// Stable string for HTTP / DB serialization. Matches WordPress's
    /// `current_user_can` dot-convention for migration familiarity.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CmsPageRead => "cms.page.read",
            Self::CmsPagePublish => "cms.page.publish",
            Self::CmsPageDraft => "cms.page.draft",
            Self::MediaUpload => "media.upload",
            Self::ManageUsers => "manage.users",
            Self::ManageSite => "manage.site",
        }
    }

    pub fn try_from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "cms.page.read" => Ok(Self::CmsPageRead),
            "cms.page.publish" => Ok(Self::CmsPagePublish),
            "cms.page.draft" => Ok(Self::CmsPageDraft),
            "media.upload" => Ok(Self::MediaUpload),
            "manage.users" => Ok(Self::ManageUsers),
            "manage.site" => Ok(Self::ManageSite),
            other => Err(AppError::Validation(format!("unknown capability: {other}"))),
        }
    }
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Ordered set of capabilities. Order is significant for
/// `cache_key_hash` determinism (§3.9.1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapabilitySet(BTreeSet<Capability>);

impl CapabilitySet {
    #[must_use]
    pub fn new() -> Self {
        Self(BTreeSet::new())
    }

    // `from_iter` inherent name collides with std::iter::FromIterator
    // method. We keep the inherent for ergonomic `CapabilitySet::from_iter([...])`
    // call style (more readable than `[...].into_iter().collect()`).
    #[allow(clippy::should_implement_trait)]
    pub fn from_iter<I: IntoIterator<Item = Capability>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }

    pub fn insert(&mut self, c: Capability) -> bool {
        self.0.insert(c)
    }

    #[must_use]
    pub fn contains(&self, c: Capability) -> bool {
        self.0.contains(&c)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = Capability> + '_ {
        self.0.iter().copied()
    }

    /// Non-cryptographic per-process-stable hash, used to compose cache
    /// keys (ENTITY §3.9.1). Two calls within the same process always
    /// produce the same value for equal sets; values may differ across
    /// Rust versions or processes.
    ///
    /// Cross-process cache-key stability (e.g. shared L2 cache) requires
    /// a stable hash function — switch to `xxh3` when the first
    /// cross-process consumer lands (M10 Dragonfly fan-out).
    #[must_use]
    pub fn cache_key_hash(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for c in &self.0 {
            c.hash(&mut h);
        }
        h.finish()
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_methods
)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const ALL: [Capability; 6] = [
        Capability::CmsPageRead,
        Capability::CmsPagePublish,
        Capability::CmsPageDraft,
        Capability::MediaUpload,
        Capability::ManageUsers,
        Capability::ManageSite,
    ];

    #[test]
    fn as_str_all_variants_distinct() {
        let set: HashSet<&str> = ALL.iter().map(|c| c.as_str()).collect();
        assert_eq!(set.len(), 6, "all 6 capability strings must be distinct");
    }

    #[test]
    fn try_from_str_roundtrip() {
        for c in ALL {
            let back = Capability::try_from_str(c.as_str()).unwrap();
            assert_eq!(back, c);
        }
    }

    #[test]
    fn try_from_str_unknown_returns_validation_err() {
        let err = Capability::try_from_str("nope.unknown").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn set_insert_returns_true_for_new_false_for_dup() {
        let mut set = CapabilitySet::new();
        assert!(set.insert(Capability::CmsPageRead));
        assert!(!set.insert(Capability::CmsPageRead));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn set_contains_works() {
        let mut set = CapabilitySet::new();
        assert!(!set.contains(Capability::CmsPageRead));
        set.insert(Capability::CmsPageRead);
        assert!(set.contains(Capability::CmsPageRead));
        assert!(!set.contains(Capability::MediaUpload));
    }

    #[test]
    fn set_hash_deterministic_within_process() {
        let a = CapabilitySet::from_iter([Capability::CmsPageRead, Capability::CmsPagePublish]);
        let b = CapabilitySet::from_iter([Capability::CmsPagePublish, Capability::CmsPageRead]);
        assert_eq!(
            a.cache_key_hash(),
            b.cache_key_hash(),
            "BTreeSet ordering guarantees equal hash regardless of insert order"
        );
    }

    #[test]
    fn set_hash_changes_with_membership() {
        let a = CapabilitySet::from_iter([Capability::CmsPageRead]);
        let b = CapabilitySet::from_iter([Capability::CmsPageRead, Capability::CmsPagePublish]);
        assert_ne!(a.cache_key_hash(), b.cache_key_hash());
    }

    #[test]
    fn set_serde_array_form() {
        let s = CapabilitySet::from_iter([Capability::CmsPageRead, Capability::MediaUpload]);
        let json = serde_json::to_string(&s).unwrap();
        assert!(
            json.starts_with('[') && json.ends_with(']'),
            "transparent over BTreeSet serializes as a JSON array, got: {json}"
        );
        let back: CapabilitySet = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn empty_set_default_and_is_empty() {
        let s = CapabilitySet::default();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }
}
