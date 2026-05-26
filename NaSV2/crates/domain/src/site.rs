//! Site aggregate · ENTITY §15 (tenant→site relation), §21 (layout).
//!
//! One `Tenant` may host many `Site`s (multi-site CMS). Each site has a
//! tenant-unique slug — the public URL path component.
//!
//! ## TLA layers
//!
//! - **L1 Correctness** — `SiteSlug::try_new` enforces the slug
//!   invariant at construction. `Site` therefore cannot be built with
//!   an invalid slug.
//! - **L2 Performance** — `SiteSlug` is a thin wrapper over `String`;
//!   we accept the allocation here because slug parsing happens once
//!   per request at the routing layer. For repeated comparison against
//!   fixed strings, callers may keep the inner `&str` via `as_str()`.
//! - **L3 Scalability** — no global state. Aggregate ownership is
//!   per-request.
//! - **L4 Operability** — slug validation errors carry the original
//!   input truncated to 80 chars for log correlation.

use chrono::{DateTime, Utc};
use nas2_common::{AppError, SiteId, TenantId};
use serde::{Deserialize, Serialize};

/// Persisted Site aggregate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Site {
    pub id: SiteId,
    pub tenant_id: TenantId,
    pub slug: SiteSlug,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
}

/// Tenant-unique URL slug for a site.
///
/// Invariants (enforced by `try_new`):
///   - length 3..=64 bytes (= chars; ASCII-only)
///   - regex `^[a-z0-9](?:[a-z0-9]|-(?=[a-z0-9]))*$`
///     lowercase ASCII, digits, single hyphens only between
///     alphanumeric — no leading/trailing hyphen, no `--`.
///
/// Validation is hand-coded (no `regex` crate — `regex` is not on the
/// §2.6 domain allow-list).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct SiteSlug(String);

impl SiteSlug {
    pub fn try_new(raw: &str) -> Result<Self, AppError> {
        validate_slug(raw, 3, 64)
            .map(|()| Self(raw.to_owned()))
            .map_err(AppError::Validation)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SiteSlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Slug validation — shared with `PostSlug` (W2 D2) via `pub(crate)`.
///
/// `indexing_slicing` allowed: the length check makes `bytes[0]` and
/// `bytes[len-1]` safe; `windows(2)` yields exactly-2-element slices so
/// `w[0]` / `w[1]` are infallible.
#[allow(clippy::indexing_slicing)]
pub(crate) fn validate_slug(raw: &str, min_len: usize, max_len: usize) -> Result<(), String> {
    if !raw.is_ascii() {
        return Err(format!("slug must be ASCII: {:?}", truncate(raw, 80)));
    }
    let len = raw.len();
    if len < min_len || len > max_len {
        return Err(format!(
            "slug length {len} out of range {min_len}..={max_len}: {:?}",
            truncate(raw, 80)
        ));
    }
    let bytes = raw.as_bytes();
    let is_alnum = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    let is_hyphen = |b: u8| b == b'-';

    // Length check above guarantees bytes[0] / bytes[len-1] are safe.
    if !is_alnum(bytes[0]) {
        return Err(format!(
            "slug must start with [a-z0-9]: {:?}",
            truncate(raw, 80)
        ));
    }
    if !is_alnum(bytes[len - 1]) {
        return Err(format!(
            "slug must end with [a-z0-9]: {:?}",
            truncate(raw, 80)
        ));
    }
    for w in bytes.windows(2) {
        if is_hyphen(w[0]) && is_hyphen(w[1]) {
            return Err(format!(
                "slug must not contain `--`: {:?}",
                truncate(raw, 80)
            ));
        }
    }
    for &b in bytes {
        if !(is_alnum(b) || is_hyphen(b)) {
            return Err(format!(
                "slug char must be [a-z0-9-]: {:?} (byte 0x{b:02x})",
                truncate(raw, 80)
            ));
        }
    }
    Ok(())
}

/// ASCII-precondition holds at every caller — byte slice is
/// char-boundary safe.
#[allow(clippy::indexing_slicing)]
fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
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
    use uuid::Uuid;

    #[test]
    fn valid_examples() {
        assert!(SiteSlug::try_new("my-site").is_ok());
        assert!(SiteSlug::try_new("acme").is_ok());
        assert!(SiteSlug::try_new("site-2").is_ok());
        assert!(SiteSlug::try_new("a-b-c").is_ok());
    }

    #[test]
    fn rejects_too_short() {
        let err = SiteSlug::try_new("ab").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(err.to_string().contains("length"));
    }

    #[test]
    fn rejects_too_long() {
        let s = "a".repeat(65);
        assert!(SiteSlug::try_new(&s).is_err());
    }

    #[test]
    fn rejects_uppercase() {
        assert!(SiteSlug::try_new("MySite").is_err());
    }

    #[test]
    fn rejects_leading_hyphen() {
        let err = SiteSlug::try_new("-foo").unwrap_err();
        assert!(err.to_string().contains("start"));
    }

    #[test]
    fn rejects_trailing_hyphen() {
        let err = SiteSlug::try_new("foo-").unwrap_err();
        assert!(err.to_string().contains("end"));
    }

    #[test]
    fn rejects_double_hyphen() {
        let err = SiteSlug::try_new("foo--bar").unwrap_err();
        assert!(err.to_string().contains("--"));
    }

    #[test]
    fn rejects_non_ascii() {
        assert!(SiteSlug::try_new("кот").is_err());
    }

    #[test]
    fn rejects_underscore() {
        assert!(SiteSlug::try_new("foo_bar").is_err());
    }

    #[test]
    fn site_serde_roundtrip() {
        let site = Site {
            id: SiteId::new_v4(),
            tenant_id: TenantId::new_v4(),
            slug: SiteSlug::try_new("acme").unwrap(),
            display_name: "Acme Corp".to_owned(),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
        };
        let json = serde_json::to_string(&site).unwrap();
        let back: Site = serde_json::from_str(&json).unwrap();
        assert_eq!(site, back);
    }

    #[test]
    fn site_slug_is_serde_transparent() {
        let slug = SiteSlug::try_new("hello").unwrap();
        let json = serde_json::to_string(&slug).unwrap();
        assert_eq!(json, r#""hello""#);
    }

    #[test]
    fn validate_slug_truncates_long_error() {
        let huge = "0".repeat(200) + "!"; // first invalid char at index 200
        let err = SiteSlug::try_new(&huge).unwrap_err();
        // Truncated input in the error message <= ~100 chars total
        // (80 for input + format overhead).
        assert!(
            err.to_string().len() < 200,
            "error message must be bounded: {} chars",
            err.to_string().len()
        );
    }

    #[test]
    fn site_id_and_tenant_id_uses_correct_uuid_versions() {
        // Compile-test: SiteId and TenantId are newtypes over Uuid.
        let s = SiteId::from_uuid(Uuid::new_v4());
        let t = TenantId::from_uuid(Uuid::new_v4());
        assert_ne!(s.into_uuid(), t.into_uuid());
    }
}
