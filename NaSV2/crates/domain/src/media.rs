//! Media aggregate placeholder · ENTITY §3.7.
//!
//! The full Media aggregate (with variant manifest, mime type,
//! checksum, alt text, …) lands with the image-pipeline at M3 G2. Today
//! we reserve the type so `MediaRepository` can compile and downstream
//! code can take a `Media` dependency without churn.

use chrono::{DateTime, Utc};
use nas2_common::{MediaId, TenantId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Media {
    pub id: MediaId,
    pub tenant_id: TenantId,
    /// CDN-resolved URL — thin placeholder. Becomes a richer
    /// `MediaVariants { thumb, medium, large, original }` at M3 G2.
    pub url: String,
    pub created_at: DateTime<Utc>,
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
    fn media_serde_roundtrip() {
        let m = Media {
            id: MediaId::new_v4(),
            tenant_id: TenantId::new_v4(),
            url: "https://cdn.example.com/a.png".into(),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: Media = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }
}
