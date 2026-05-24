//! `User` aggregate — reconstitution + invariants.
//!
//! Pure domain — no sqlx, no axum, no tokio. Только serde/uuid/chrono.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User status. Только `Active` пропускается через auth middleware → handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Active,
    Suspended,
    Archived,
}

impl UserStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Archived => "archived",
        }
    }

    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    /// Parse string → status. Unknown → Archived (defensive — locks out unknown values).
    pub fn parse(s: &str) -> Result<Self, UserError> {
        match s {
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            "archived" => Ok(Self::Archived),
            other => Err(UserError::InvalidStatus(other.to_owned())),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UserError {
    #[error("email empty")]
    EmailEmpty,
    #[error("email too long ({0} > 320)")]
    EmailTooLong(usize),
    #[error("email missing @ separator")]
    EmailNoAt,
    #[error("invalid status: {0}")]
    InvalidStatus(String),
}

/// User aggregate — reconstituted from DB row.
///
/// Construction только через `reconstitute()` — гарантирует что все invariants
/// проверены до публикации aggregate в use case layer.
#[derive(Clone, Debug)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Reconstitute from DB row (no business validation — DB invariants assumed).
    /// Validates only что email non-empty + contains '@'.
    ///
    /// **Caveat:** password_hash НЕ в aggregate — хранится только в repository layer
    /// и используется для verify_password (Phase B). AX Phase A не делает password verify
    /// (login flow остаётся SITE1).
    pub fn reconstitute(
        id: Uuid,
        tenant_id: Uuid,
        email: String,
        display_name: Option<String>,
        status: UserStatus,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, UserError> {
        Self::validate_email(&email)?;
        Ok(Self {
            id,
            tenant_id,
            email,
            display_name,
            status,
            created_at,
            updated_at,
        })
    }

    fn validate_email(email: &str) -> Result<(), UserError> {
        if email.is_empty() {
            return Err(UserError::EmailEmpty);
        }
        if email.len() > 320 {
            return Err(UserError::EmailTooLong(email.len()));
        }
        if !email.contains('@') {
            return Err(UserError::EmailNoAt);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt() -> DateTime<Utc> {
        Utc::now()
    }

    #[test]
    fn reconstitute_accepts_valid_user() {
        let u = User::reconstitute(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "admin@example.com".to_owned(),
            Some("Admin".to_owned()),
            UserStatus::Active,
            dt(),
            dt(),
        );
        assert!(u.is_ok());
    }

    #[test]
    fn reconstitute_rejects_empty_email() {
        let u = User::reconstitute(
            Uuid::new_v4(),
            Uuid::new_v4(),
            String::new(),
            None,
            UserStatus::Active,
            dt(),
            dt(),
        );
        assert_eq!(u.unwrap_err(), UserError::EmailEmpty);
    }

    #[test]
    fn reconstitute_rejects_email_without_at() {
        let u = User::reconstitute(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "notanemail".to_owned(),
            None,
            UserStatus::Active,
            dt(),
            dt(),
        );
        assert_eq!(u.unwrap_err(), UserError::EmailNoAt);
    }

    #[test]
    fn reconstitute_rejects_email_too_long() {
        let email = format!("{}@example.com", "a".repeat(310));
        let u = User::reconstitute(
            Uuid::new_v4(),
            Uuid::new_v4(),
            email,
            None,
            UserStatus::Active,
            dt(),
            dt(),
        );
        assert!(matches!(u.unwrap_err(), UserError::EmailTooLong(_)));
    }

    #[test]
    fn user_status_parse_roundtrip() {
        for s in [UserStatus::Active, UserStatus::Suspended, UserStatus::Archived] {
            let str = s.as_str();
            let parsed = UserStatus::parse(str).unwrap();
            assert_eq!(s, parsed);
        }
    }

    #[test]
    fn user_status_parse_rejects_unknown() {
        assert_eq!(
            UserStatus::parse("pending").unwrap_err(),
            UserError::InvalidStatus("pending".to_owned())
        );
    }

    #[test]
    fn user_status_active_only_active() {
        assert!(UserStatus::Active.is_active());
        assert!(!UserStatus::Suspended.is_active());
        assert!(!UserStatus::Archived.is_active());
    }
}
