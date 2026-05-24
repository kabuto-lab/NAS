//! PostgreSQL implementation of `TenantResolver` + moka LRU cache.
//!
//! Per ADR-001 D6: cache TTL 5 min, capacity 10k entries.

use async_trait::async_trait;
use ax_application::ports::TenantResolver;
use ax_common::{ids::RequestId, AppError, TenantContext, TenantId, TenantStatus};
use moka::future::Cache;
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgTenantResolver {
    pool: PgPool,
    cache: Cache<String, Option<CachedTenant>>,
}

#[derive(Clone, Debug)]
struct CachedTenant {
    id: Uuid,
    slug: String,
    status: String,
}

impl PgTenantResolver {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        let cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(300))
            .build();
        Self { pool, cache }
    }
}

#[async_trait]
impl TenantResolver for PgTenantResolver {
    async fn resolve_by_slug(&self, slug: &str) -> Result<Option<TenantContext>, AppError> {
        // Normalize slug: lowercase + trim. Stricter regex enforce'нут на extract в middleware.
        let slug_key = slug.trim().to_lowercase();
        if slug_key.is_empty() {
            return Ok(None);
        }

        if let Some(cached) = self.cache.get(&slug_key).await {
            return Ok(cached.map(build_context));
        }

        // sqlx::query_as (runtime checked; switch to query_as! after sqlx prepare)
        let row: Option<(Uuid, String, String)> = sqlx::query_as::<_, (Uuid, String, String)>(
            r"SELECT id, slug, status FROM tenants WHERE slug = $1 LIMIT 1",
        )
        .bind(&slug_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let cached = row.map(|(id, slug, status)| CachedTenant { id, slug, status });
        self.cache.insert(slug_key, cached.clone()).await;

        Ok(cached.map(build_context))
    }
}

fn build_context(cached: CachedTenant) -> TenantContext {
    let status = match cached.status.as_str() {
        "active" => TenantStatus::Active,
        "pending" => TenantStatus::Pending,
        "suspended" => TenantStatus::Suspended,
        _ => TenantStatus::Archived,
    };
    TenantContext {
        tenant_id: TenantId::new(cached.id),
        tenant_slug: Arc::from(cached.slug.as_str()),
        status,
        request_id: RequestId::new(),
        user_id: None,
    }
}
