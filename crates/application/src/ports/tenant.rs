//! `TenantResolver` port — resolves slug → TenantContext.
//!
//! Implementation в `ax-infrastructure::tenant::PgTenantResolver` (с moka LRU
//! 5 min TTL per ADR-001 D6).

use async_trait::async_trait;
use ax_common::{AppError, TenantContext};

#[async_trait]
pub trait TenantResolver: Send + Sync {
    /// Resolve tenant slug → context. Returns `Ok(None)` если slug не найден
    /// (resolver не бросает — guard на endpoint'е сам решает 404 vs 401).
    ///
    /// # Errors
    /// - `AppError::Database` если SQL запрос упал
    async fn resolve_by_slug(&self, slug: &str) -> Result<Option<TenantContext>, AppError>;
}
