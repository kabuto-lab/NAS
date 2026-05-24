//! `PgUserRepository` — SQLx implementation of `UserRepository` port.
//!
//! Spec: `ADR-002 §D8` (users RLS), `ADR-002 §D3` (capability storage via joins).
//!
//! Phase A queries:
//! - `find_by_id(ctx, user_id)` — for `AuthenticatedUser` injection (rare; cached at handler-level)
//! - `get_capabilities(ctx, user_id)` — JOIN user_roles + role_capabilities, returns set
//!
//! Both run inside `with_tenant` — RLS POLICY ensures cross-tenant isolation
//! даже если caller forgets WHERE tenant_id.

use async_trait::async_trait;
use ax_application::ports::UserRepository;
use ax_common::{AppError, Capability, CapabilitySet, TenantContext, UserId};
use ax_domain::user::{User, UserStatus};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::str::FromStr;
use uuid::Uuid;

use crate::persistence::transaction::with_tenant;

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
struct UserRow {
    id: Uuid,
    tenant_id: Option<Uuid>,
    email: String,
    display_name: Option<String>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[async_trait]
impl UserRepository for PgUserRepository {
    #[tracing::instrument(skip(self), fields(tenant_id = %ctx.tenant_id, user_id = %user_id))]
    async fn find_by_id(
        &self,
        ctx: &TenantContext,
        user_id: UserId,
    ) -> Result<Option<User>, AppError> {
        let row: Option<UserRow> = with_tenant(&self.pool, ctx, move |tx| {
            Box::pin(async move {
                sqlx::query_as::<_, UserRow>(
                    "SELECT id, tenant_id, email, display_name, status, created_at, updated_at \
                     FROM users \
                     WHERE id = $1 \
                     LIMIT 1",
                )
                .bind(user_id.0)
                .fetch_optional(&mut **tx)
                .await
                .map_err(|e| AppError::Database(e.to_string()))
            })
        })
        .await?;

        let Some(row) = row else { return Ok(None) };

        // tenant_id NOT NULL invariant — must match ctx (RLS already enforced, defensive)
        let row_tenant = row.tenant_id.ok_or_else(|| {
            tracing::error!(user_id = %user_id, "user row missing tenant_id");
            AppError::Internal(eyre::eyre!("user row missing tenant_id"))
        })?;
        if row_tenant != ctx.tenant_id.0 {
            tracing::warn!(user_id = %user_id, "tenant mismatch — RLS bypassed?");
            return Err(AppError::TenantMismatch);
        }

        let status = UserStatus::parse(&row.status)
            .map_err(|e| AppError::Database(format!("invalid user status: {e}")))?;
        let user = User::reconstitute(
            row.id,
            row_tenant,
            row.email,
            row.display_name,
            status,
            row.created_at,
            row.updated_at,
        )
        .map_err(|e| AppError::Internal(eyre::eyre!("user reconstitute: {e}")))?;
        Ok(Some(user))
    }

    #[tracing::instrument(skip(self), fields(tenant_id = %ctx.tenant_id, user_id = %user_id))]
    async fn get_capabilities(
        &self,
        ctx: &TenantContext,
        user_id: UserId,
    ) -> Result<CapabilitySet, AppError> {
        let keys: Vec<String> = with_tenant(&self.pool, ctx, move |tx| {
            Box::pin(async move {
                sqlx::query_scalar::<_, String>(
                    "SELECT DISTINCT rc.capability_key \
                     FROM user_roles ur \
                     JOIN roles r ON r.id = ur.role_id \
                     JOIN role_capabilities rc ON rc.role_id = r.id \
                     WHERE ur.user_id = $1",
                )
                .bind(user_id.0)
                .fetch_all(&mut **tx)
                .await
                .map_err(|e| AppError::Database(e.to_string()))
            })
        })
        .await?;

        let mut set = CapabilitySet::new();
        for key in keys {
            match Capability::from_str(&key) {
                Ok(cap) => {
                    set.insert(cap);
                }
                Err(unknown) => {
                    // Forward-compat: unknown capability в DB — log + skip
                    tracing::warn!(unknown = %unknown.0, "unknown capability key in DB");
                }
            }
        }
        Ok(set)
    }
}
