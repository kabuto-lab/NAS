//! `Capability` — typed permission enum + `CapabilitySet` collection.
//!
//! Spec: `ENTITY.md §7.3` (RBAC pattern), `ADR-002 §D10` (naming + enum scope).
//!
//! Capability key format: `<resource>:<action>[:<modifier>]` — мирорит SITE1 RBAC strings.
//! Phase A scope: 4 variants (Posts*). Расширение per follow-up RFCs.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str::FromStr;

/// Typed capability enum. Compile-time guarantees что `RequireCapability<Cap>`
/// и `cap_resolver.require(user, Cap)` принимают только well-known values.
///
/// Persistence layer хранит как `capability_key` varchar(128) PK in `capabilities` table.
/// String round-trip через `as_key()` / `FromStr` for DB ↔ enum mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    PostsCreate,
    PostsEdit,
    PostsPublish,
    PostsDelete,
}

impl Capability {
    /// Канонический string-key для DB / cross-stack comparison.
    #[must_use]
    pub const fn as_key(self) -> &'static str {
        match self {
            Self::PostsCreate => "posts:create",
            Self::PostsEdit => "posts:edit",
            Self::PostsPublish => "posts:publish",
            Self::PostsDelete => "posts:delete",
        }
    }

    /// All known variants — для seed scripts + capability registry initialization.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::PostsCreate,
            Self::PostsEdit,
            Self::PostsPublish,
            Self::PostsDelete,
        ]
    }
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_key())
    }
}

/// Error при parse string → Capability. Unknown key — security-relevant signal
/// (potential capability registry drift), но не security event для AppError mapping.
#[derive(Debug, thiserror::Error)]
#[error("unknown capability key: {0}")]
pub struct UnknownCapability(pub String);

impl FromStr for Capability {
    type Err = UnknownCapability;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "posts:create" => Ok(Self::PostsCreate),
            "posts:edit" => Ok(Self::PostsEdit),
            "posts:publish" => Ok(Self::PostsPublish),
            "posts:delete" => Ok(Self::PostsDelete),
            other => Err(UnknownCapability(other.to_owned())),
        }
    }
}

/// Bounded set of capabilities, attached to user после resolve.
///
/// `HashSet` chosen over bitset для Phase A: <12 variants, hash overhead marginal,
/// reads use `.contains()` which is O(1). Per ENTITY §11.5 alloc budget — single
/// allocation per resolve, cached в moka.
#[derive(Clone, Debug, Default)]
pub struct CapabilitySet {
    inner: HashSet<Capability>,
}

impl CapabilitySet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, cap: Capability) -> bool {
        self.inner.insert(cap)
    }

    #[must_use]
    pub fn contains(&self, cap: Capability) -> bool {
        self.inner.contains(&cap)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.inner.iter()
    }
}

impl FromIterator<Capability> for CapabilitySet {
    fn from_iter<T: IntoIterator<Item = Capability>>(iter: T) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_key_format_stable() {
        assert_eq!(Capability::PostsCreate.as_key(), "posts:create");
        assert_eq!(Capability::PostsEdit.as_key(), "posts:edit");
        assert_eq!(Capability::PostsPublish.as_key(), "posts:publish");
        assert_eq!(Capability::PostsDelete.as_key(), "posts:delete");
    }

    #[test]
    fn from_str_roundtrip_all_variants() {
        for cap in Capability::all() {
            let key = cap.as_key();
            let parsed: Capability = key.parse().expect("known key parses");
            assert_eq!(*cap, parsed);
        }
    }

    #[test]
    fn from_str_unknown_returns_error() {
        let err = Capability::from_str("posts:nuke").unwrap_err();
        assert_eq!(err.0, "posts:nuke");
    }

    #[test]
    fn display_matches_as_key() {
        assert_eq!(format!("{}", Capability::PostsCreate), "posts:create");
    }

    #[test]
    fn set_insert_dedup() {
        let mut s = CapabilitySet::new();
        assert!(s.insert(Capability::PostsCreate));
        assert!(!s.insert(Capability::PostsCreate));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn set_contains_after_insert() {
        let mut s = CapabilitySet::new();
        s.insert(Capability::PostsEdit);
        assert!(s.contains(Capability::PostsEdit));
        assert!(!s.contains(Capability::PostsDelete));
    }

    #[test]
    fn set_from_iter_collects() {
        let s: CapabilitySet = [Capability::PostsCreate, Capability::PostsPublish]
            .into_iter()
            .collect();
        assert_eq!(s.len(), 2);
        assert!(s.contains(Capability::PostsCreate));
        assert!(s.contains(Capability::PostsPublish));
    }

    #[test]
    fn serde_roundtrip() {
        let cap = Capability::PostsCreate;
        let json = serde_json::to_string(&cap).unwrap();
        // serde uses snake_case rename
        assert_eq!(json, "\"posts_create\"");
        let parsed: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, parsed);
    }
}
