//! Persistence port for `Media` aggregates. **Minimal stub** — first
//! real method (`upload`) lands with image-pipeline at M3 G2.
//!
//! Today's surface lets downstream code depend on `MediaRepository`
//! without churn when the trait grows.

// mockall::automock (test-only) uses std::sync::Mutex internally —
// disallowed in production code (ENTITY §8.7) but acceptable in tests.
#![cfg_attr(test, allow(clippy::disallowed_types))]

use async_trait::async_trait;
use nas2_common::{AppError, MediaId, TenantId};
use nas2_domain::Media;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait MediaRepository: Send + Sync {
    async fn find_by_id(
        &self,
        tenant: TenantId,
        id: MediaId,
    ) -> Result<Option<Media>, AppError>;
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn _assert_object_safe(_: &dyn MediaRepository) {}
}
