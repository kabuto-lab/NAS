//! Host → `TenantContext` resolution.
//!
//! Today: `InMemoryTenantResolver` backed by an `Arc<HashMap>`. Month-2
//! follow-up: `PgTenantResolver` with `moka` LRU (5-min TTL); same
//! trait surface.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use nas2_common::AppError;

use crate::context::TenantContext;

#[async_trait]
pub trait TenantResolver: Send + Sync + 'static {
    /// Resolve a request `Host:` header value to a `TenantContext`.
    ///
    /// Returns `Err(AppError::NotFound)` if the host is not mapped.
    async fn resolve(&self, host: &str) -> Result<TenantContext, AppError>;
}

/// Static in-memory map. Cheaply clonable (`Arc` inside).
///
/// Intended for dev/tests. Production uses `PgTenantResolver` (M2)
/// which adds moka LRU + DB lookup.
#[derive(Clone, Default)]
pub struct InMemoryTenantResolver {
    map: Arc<HashMap<String, TenantContext>>,
}

impl InMemoryTenantResolver {
    #[must_use]
    pub fn new(map: HashMap<String, TenantContext>) -> Self {
        Self { map: Arc::new(map) }
    }

    #[must_use]
    pub fn builder() -> InMemoryTenantResolverBuilder {
        InMemoryTenantResolverBuilder::default()
    }
}

#[async_trait]
impl TenantResolver for InMemoryTenantResolver {
    async fn resolve(&self, host: &str) -> Result<TenantContext, AppError> {
        self.map
            .get(host)
            .copied()
            .ok_or_else(|| AppError::NotFound(format!("unknown host: {host}")))
    }
}

#[derive(Default)]
pub struct InMemoryTenantResolverBuilder {
    map: HashMap<String, TenantContext>,
}

impl InMemoryTenantResolverBuilder {
    pub fn insert(mut self, host: impl Into<String>, ctx: TenantContext) -> Self {
        self.map.insert(host.into(), ctx);
        self
    }

    #[must_use]
    pub fn build(self) -> InMemoryTenantResolver {
        InMemoryTenantResolver::new(self.map)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use nas2_common::{SiteId, TenantId};

    #[tokio::test]
    async fn returns_context_for_known_host() {
        let tid = TenantId::new_v4();
        let sid = SiteId::new_v4();
        let r = InMemoryTenantResolver::builder()
            .insert("example.com", TenantContext::new(tid, sid))
            .build();
        let ctx = r.resolve("example.com").await.unwrap();
        assert_eq!(ctx.tenant_id, tid);
        assert_eq!(ctx.site_id, sid);
    }

    #[tokio::test]
    async fn returns_not_found_for_unknown_host() {
        let r = InMemoryTenantResolver::default();
        let err = r.resolve("nope.example.com").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
        assert!(err.to_string().contains("unknown host"));
    }
}
