//! Persistence port for `User` aggregates · ENTITY §2 (hex layer 2).

// mockall::automock (test-only) uses std::sync::Mutex internally —
// disallowed in production code (ENTITY §8.7) but acceptable in tests.
#![cfg_attr(test, allow(clippy::disallowed_types))]

use async_trait::async_trait;
use nas2_common::{AppError, TenantId, UserId};
use nas2_domain::{Email, User};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(
        &self,
        tenant: TenantId,
        id: UserId,
    ) -> Result<Option<User>, AppError>;

    async fn find_by_email(
        &self,
        tenant: TenantId,
        email: &Email,
    ) -> Result<Option<User>, AppError>;

    async fn insert(&self, user: &User) -> Result<(), AppError>;

    async fn update(&self, user: &User) -> Result<(), AppError>;
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn _assert_object_safe(_: &dyn UserRepository) {}
}
