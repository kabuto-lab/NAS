//! `RoleKey` newtype + Role descriptor.
//!
//! Spec: `ADR-002 §D3` — roles is `(tenant_id, key, display_name)` triple in DB,
//! attached к user через user_roles join.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tenant-scoped role key. Owner per `(tenant_id, key)` unique constraint.
///
/// Examples: `"admin"`, `"editor"`, `"viewer"`. Length-bounded 1-64.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoleKey(String);

#[derive(Debug, thiserror::Error)]
pub enum RoleKeyError {
    #[error("role key empty")]
    Empty,
    #[error("role key too long ({0} > 64)")]
    TooLong(usize),
    #[error("role key has invalid character")]
    InvalidChar,
}

impl RoleKey {
    /// Parse + validate. Whitelisted: a-z, 0-9, underscore; first char a-z.
    pub fn parse(raw: impl Into<String>) -> Result<Self, RoleKeyError> {
        let s = raw.into();
        if s.is_empty() {
            return Err(RoleKeyError::Empty);
        }
        if s.len() > 64 {
            return Err(RoleKeyError::TooLong(s.len()));
        }
        let bytes = s.as_bytes();
        if !bytes[0].is_ascii_lowercase() {
            return Err(RoleKeyError::InvalidChar);
        }
        for &b in &bytes[1..] {
            if !(b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
                return Err(RoleKeyError::InvalidChar);
            }
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for RoleKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Descriptor для role row из DB. Used by capability resolve flow.
#[derive(Clone, Debug)]
pub struct Role {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub key: RoleKey,
    pub display_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_valid() {
        assert!(RoleKey::parse("admin").is_ok());
        assert!(RoleKey::parse("editor").is_ok());
        assert!(RoleKey::parse("viewer").is_ok());
        assert!(RoleKey::parse("tenant_admin").is_ok());
        assert!(RoleKey::parse("role99").is_ok());
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(matches!(RoleKey::parse(""), Err(RoleKeyError::Empty)));
    }

    #[test]
    fn parse_rejects_too_long() {
        let s = "a".repeat(65);
        assert!(matches!(RoleKey::parse(s), Err(RoleKeyError::TooLong(_))));
    }

    #[test]
    fn parse_rejects_leading_non_alpha() {
        assert!(matches!(RoleKey::parse("1admin"), Err(RoleKeyError::InvalidChar)));
        assert!(matches!(RoleKey::parse("_admin"), Err(RoleKeyError::InvalidChar)));
        assert!(matches!(RoleKey::parse("Admin"), Err(RoleKeyError::InvalidChar)));
    }

    #[test]
    fn parse_rejects_invalid_chars() {
        assert!(matches!(RoleKey::parse("admin-user"), Err(RoleKeyError::InvalidChar)));
        assert!(matches!(RoleKey::parse("admin.user"), Err(RoleKeyError::InvalidChar)));
        assert!(matches!(RoleKey::parse("admin user"), Err(RoleKeyError::InvalidChar)));
    }

    #[test]
    fn display_matches_input() {
        let key = RoleKey::parse("admin").unwrap();
        assert_eq!(format!("{key}"), "admin");
    }
}
