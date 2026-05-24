//! `UserRepository` port trait — auth + RBAC persistence interface.
//!
//! Implementation в `ax-infrastructure::persistence::PgUserRepository`.
//!
//! Phase A scope:
//! - `find_by_id` — для AuthenticatedUser hydration (rare per-request)
//! - `get_capabilities` — JOIN user_roles + role_capabilities, used by capability resolver

use async_trait::async_trait;
use ax_common::{AppError, CapabilitySet, TenantContext, UserId};
use ax_domain::user::User;

/// Repository contract for users + their RBAC mappings.
///
/// **Invariants для implementers:**
///
/// 1. **Tenant isolation:** queries MUST run inside `with_tenant(...)` (или эквивалент)
///    чтобы RLS POLICY `users_tenant_isolation` срабатывал.
/// 2. **Cross-tenant — None или TenantMismatch:** если user_id existed но в другом
///    tenant — implementation MUST return Ok(None) or Err(AppError::TenantMismatch).
///    Не возвращать user object для wrong tenant.
/// 3. **Unknown capability_key в DB:** skip + warn (forward-compat), не fail entire request.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Найти user by id within current tenant context.
    ///
    /// # Errors
    /// - `AppError::Database` — SQL error
    /// - `AppError::TenantMismatch` — defensive check если RLS bypass'нулся
    /// - `AppError::Internal` — invariant violation в reconstitute
    async fn find_by_id(
        &self,
        ctx: &TenantContext,
        user_id: UserId,
    ) -> Result<Option<User>, AppError>;

    /// Resolve all capabilities granted to user через user_roles → role_capabilities.
    /// Empty set если user не имеет roles или roles не имеют capabilities.
    ///
    /// # Errors
    /// - `AppError::Database` — SQL error
    async fn get_capabilities(
        &self,
        ctx: &TenantContext,
        user_id: UserId,
    ) -> Result<CapabilitySet, AppError>;
}
