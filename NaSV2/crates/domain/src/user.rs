//! User aggregate + `Email` value object · ENTITY §14.

use chrono::{DateTime, Utc};
use nas2_common::{AppError, RoleId, TenantId, UserId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: UserId,
    pub tenant_id: TenantId,
    pub email: Email,
    pub role_ids: Vec<RoleId>,
    pub created_at: DateTime<Utc>,
}

/// RFC-5322-light email validation. Sufficient for MVP — full RFC
/// parsing is deferred (Ideas-for-next-month).
///
/// Invariants (enforced by `try_new`):
///   - length 6..=254 bytes (RFC 5321 envelope limit)
///   - no ASCII whitespace
///   - exactly one `@`
///   - local-part non-empty; domain non-empty
///   - domain contains at least one `.`
///
/// Stored lowercased (ASCII fold; non-ASCII local-parts are accepted
/// per RFC 6531 but not case-folded).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct Email(String);

impl Email {
    pub fn try_new(raw: &str) -> Result<Self, AppError> {
        let len = raw.len();
        if !(6..=254).contains(&len) {
            return Err(AppError::Validation(format!(
                "email length {len} not in 6..=254"
            )));
        }
        if raw.chars().any(char::is_whitespace) {
            return Err(AppError::Validation("email contains whitespace".into()));
        }
        let mut parts = raw.split('@');
        let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err(AppError::Validation(
                "email must contain exactly one '@'".into(),
            ));
        };
        if local.is_empty() {
            return Err(AppError::Validation("email local-part empty".into()));
        }
        if domain.is_empty() || !domain.contains('.') {
            return Err(AppError::Validation("email domain invalid".into()));
        }
        Ok(Self(raw.to_ascii_lowercase()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
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
    use chrono::TimeZone;

    #[test]
    fn valid_examples() {
        assert!(Email::try_new("a@b.co").is_ok());
        assert!(Email::try_new("jane@example.com").is_ok());
        assert!(Email::try_new("x+tag@example.io").is_ok());
        assert!(Email::try_new("u@sub.example.com").is_ok());
    }

    #[test]
    fn too_short_rejected() {
        let err = Email::try_new("a@b.c").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn too_long_rejected() {
        let s = format!("{}@example.com", "a".repeat(250));
        assert!(Email::try_new(&s).is_err());
    }

    #[test]
    fn no_at_rejected() {
        assert!(Email::try_new("abcdef").is_err());
    }

    #[test]
    fn two_ats_rejected() {
        let err = Email::try_new("a@b@c.com").unwrap_err();
        assert!(err.to_string().contains("exactly one '@'"));
    }

    #[test]
    fn no_domain_dot_rejected() {
        let err = Email::try_new("a@localhost").unwrap_err();
        assert!(err.to_string().contains("domain"));
    }

    #[test]
    fn whitespace_rejected() {
        assert!(Email::try_new("a b@c.com").is_err());
        assert!(Email::try_new("a@c .com").is_err());
    }

    #[test]
    fn lowercased_on_construction() {
        let e = Email::try_new("A@B.Com").unwrap();
        assert_eq!(e.as_str(), "a@b.com");
    }

    #[test]
    fn user_serde_roundtrip() {
        let u = User {
            id: UserId::new_v4(),
            tenant_id: TenantId::new_v4(),
            email: Email::try_new("jane@example.com").unwrap(),
            role_ids: vec![RoleId::new_v4(), RoleId::new_v4()],
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
        };
        let json = serde_json::to_string(&u).unwrap();
        let back: User = serde_json::from_str(&json).unwrap();
        assert_eq!(u, back);
    }
}
