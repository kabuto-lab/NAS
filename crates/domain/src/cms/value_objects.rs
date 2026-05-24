//! Value objects для CMS — `PageSlug`, `PageLocale`, `PageStatus`.
//!
//! Все имеют garde validation и serde roundtrip. Spec: `audit §4.1 + §4.3`,
//! `bridge/02 §2`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Validation errors для CMS value objects.
#[derive(Debug, Error)]
pub enum CmsValidationError {
    #[error("slug must be 3-80 chars, lowercase ASCII + digits + hyphens + slashes: {0}")]
    InvalidSlug(String),
    #[error("locale must be 'ru' or 'en': {0}")]
    InvalidLocale(String),
    #[error("status must be one of draft|published|archived: {0}")]
    InvalidStatus(String),
}

/// Page slug — `^[a-z0-9](?:[a-z0-9/-]{1,78}[a-z0-9])?$` (3-80 chars).
///
/// **Allows slash** для nested paths (`services/spa`). См. audit §4.1.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PageSlug(String);

impl PageSlug {
    /// Parse + validate. Принимает только lowercase, digits, hyphens, slashes;
    /// 3-80 chars; start/end — alphanumeric.
    pub fn parse(s: &str) -> Result<Self, CmsValidationError> {
        if s.len() < 3 || s.len() > 80 {
            return Err(CmsValidationError::InvalidSlug(s.to_string()));
        }
        let bytes = s.as_bytes();

        let is_alnum_lower = |b: u8| b.is_ascii_digit() || (b'a'..=b'z').contains(&b);
        let is_mid_char = |b: u8| is_alnum_lower(b) || b == b'-' || b == b'/';

        if !is_alnum_lower(bytes[0]) || !is_alnum_lower(bytes[bytes.len() - 1]) {
            return Err(CmsValidationError::InvalidSlug(s.to_string()));
        }
        if !bytes[1..bytes.len() - 1].iter().all(|&b| is_mid_char(b)) {
            return Err(CmsValidationError::InvalidSlug(s.to_string()));
        }

        Ok(Self(s.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PageSlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for PageSlug {
    type Error = CmsValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<PageSlug> for String {
    fn from(value: PageSlug) -> Self {
        value.0
    }
}

/// Page locale. SITE1 поддерживает только `ru` и `en` (audit §4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageLocale {
    Ru,
    En,
}

impl PageLocale {
    pub fn parse(s: &str) -> Result<Self, CmsValidationError> {
        match s {
            "ru" => Ok(Self::Ru),
            "en" => Ok(Self::En),
            other => Err(CmsValidationError::InvalidLocale(other.to_string())),
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ru => "ru",
            Self::En => "en",
        }
    }
}

impl Default for PageLocale {
    fn default() -> Self {
        Self::Ru
    }
}

impl std::fmt::Display for PageLocale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Page status — lifecycle FSM `draft → published → archived`.
///
/// Audit §1.5: invariant `status='Published' ⇔ published_at IS NOT NULL` — enforce'ится
/// service layer'ом, не DB. AX domain'у это нужно знать для aggregate reconstruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageStatus {
    Draft,
    Published,
    Archived,
}

impl PageStatus {
    pub fn parse(s: &str) -> Result<Self, CmsValidationError> {
        match s {
            "draft" => Ok(Self::Draft),
            "published" => Ok(Self::Published),
            "archived" => Ok(Self::Archived),
            other => Err(CmsValidationError::InvalidStatus(other.to_string())),
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Archived => "archived",
        }
    }

    #[must_use]
    pub const fn is_published(self) -> bool {
        matches!(self, Self::Published)
    }
}

impl std::fmt::Display for PageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_accepts_valid_examples() {
        assert!(PageSlug::parse("home").is_ok());
        assert!(PageSlug::parse("about").is_ok());
        assert!(PageSlug::parse("services/spa").is_ok());
        assert!(PageSlug::parse("services/spa-detox").is_ok());
        assert!(PageSlug::parse("a23").is_ok());
        // 80 chars
        let long = "a".repeat(80);
        assert!(PageSlug::parse(&long).is_ok());
    }

    #[test]
    fn slug_rejects_invalid_examples() {
        // Too short
        assert!(PageSlug::parse("").is_err());
        assert!(PageSlug::parse("ab").is_err());
        // Too long
        let too_long = "a".repeat(81);
        assert!(PageSlug::parse(&too_long).is_err());
        // Uppercase
        assert!(PageSlug::parse("ABout").is_err());
        // Leading slash/hyphen
        assert!(PageSlug::parse("/about").is_err());
        assert!(PageSlug::parse("-about").is_err());
        // Trailing slash/hyphen
        assert!(PageSlug::parse("about/").is_err());
        assert!(PageSlug::parse("about-").is_err());
        // Special chars
        assert!(PageSlug::parse("about?").is_err());
        assert!(PageSlug::parse("about.html").is_err());
    }

    #[test]
    fn slug_serde_roundtrip() {
        let slug = PageSlug::parse("services/spa").unwrap();
        let json = serde_json::to_string(&slug).unwrap();
        assert_eq!(json, r#""services/spa""#);
        let parsed: PageSlug = serde_json::from_str(&json).unwrap();
        assert_eq!(slug, parsed);
    }

    #[test]
    fn locale_parses_ru_en() {
        assert_eq!(PageLocale::parse("ru").unwrap(), PageLocale::Ru);
        assert_eq!(PageLocale::parse("en").unwrap(), PageLocale::En);
        assert!(PageLocale::parse("fr").is_err());
        assert!(PageLocale::parse("RU").is_err());
    }

    #[test]
    fn locale_default_is_ru() {
        assert_eq!(PageLocale::default(), PageLocale::Ru);
    }

    #[test]
    fn locale_serde_lowercase() {
        let json = serde_json::to_string(&PageLocale::Ru).unwrap();
        assert_eq!(json, r#""ru""#);
        let json = serde_json::to_string(&PageLocale::En).unwrap();
        assert_eq!(json, r#""en""#);
    }

    #[test]
    fn status_parses_all_variants() {
        assert_eq!(PageStatus::parse("draft").unwrap(), PageStatus::Draft);
        assert_eq!(PageStatus::parse("published").unwrap(), PageStatus::Published);
        assert_eq!(PageStatus::parse("archived").unwrap(), PageStatus::Archived);
        assert!(PageStatus::parse("pending").is_err());
    }

    #[test]
    fn status_is_published_only_for_published() {
        assert!(PageStatus::Published.is_published());
        assert!(!PageStatus::Draft.is_published());
        assert!(!PageStatus::Archived.is_published());
    }
}
