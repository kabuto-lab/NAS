//! `CapabilityResolver` — cached lookup of CapabilitySet by UserId.
//!
//! Spec: `ADR-002 §D4` (moka future, 60s TTL, 5000 cap), `§D6` (lazy resolve in handler).
//!
//! Cache key: `UserId` (not `(UserId, tenant_id)`) — user belongs to single tenant
//! per ADR D8 invariant. If user moves tenant — manual cache invalidation (Phase B).

use ax_common::{AppError, Capability, CapabilitySet, TenantContext, UserId};
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

use crate::ports::UserRepository;

/// Capability resolver — wraps UserRepository with moka cache.
pub struct CapabilityResolver {
    user_repo: Arc<dyn UserRepository>,
    cache: Cache<UserId, CapabilitySet>,
}

impl CapabilityResolver {
    /// Construct with default cache config per ADR-002 D4.
    #[must_use]
    pub fn new(user_repo: Arc<dyn UserRepository>) -> Self {
        let cache = Cache::builder()
            .max_capacity(5_000)
            .time_to_live(Duration::from_secs(60))
            .build();
        Self { user_repo, cache }
    }

    /// Construct с explicit cache config (for tests).
    #[must_use]
    pub fn with_cache(
        user_repo: Arc<dyn UserRepository>,
        cache: Cache<UserId, CapabilitySet>,
    ) -> Self {
        Self { user_repo, cache }
    }

    /// Resolve (cached or fresh) all capabilities for user.
    ///
    /// # Errors
    /// - `AppError::Database` if DB query fails
    #[tracing::instrument(skip(self), fields(user_id = %user_id, tenant_id = %ctx.tenant_id))]
    pub async fn resolve(
        &self,
        ctx: &TenantContext,
        user_id: UserId,
    ) -> Result<CapabilitySet, AppError> {
        // moka returns Option<T>; if hit, return clone'd CapabilitySet.
        if let Some(cached) = self.cache.get(&user_id).await {
            tracing::debug!("capability cache hit");
            return Ok(cached);
        }
        tracing::debug!("capability cache miss — DB lookup");
        let set = self.user_repo.get_capabilities(ctx, user_id).await?;
        self.cache.insert(user_id, set.clone()).await;
        Ok(set)
    }

    /// Verify user has specific capability. Convenience for handler call sites.
    ///
    /// # Errors
    /// - `AppError::MissingCapability(cap)` if user lacks the capability
    /// - propagates errors from `resolve()`
    pub async fn require(
        &self,
        ctx: &TenantContext,
        user_id: UserId,
        cap: Capability,
    ) -> Result<(), AppError> {
        let set = self.resolve(ctx, user_id).await?;
        if set.contains(cap) {
            Ok(())
        } else {
            Err(AppError::MissingCapability(cap))
        }
    }

    /// Invalidate cache entry. Used by future user-role mutation endpoints (Phase B).
    pub async fn invalidate(&self, user_id: UserId) {
        self.cache.invalidate(&user_id).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use ax_common::TenantId;
    use ax_domain::user::User;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use uuid::Uuid;

    struct StubRepo {
        capabilities: Vec<Capability>,
        call_count: AtomicUsize,
    }

    #[async_trait]
    impl UserRepository for StubRepo {
        async fn find_by_id(
            &self,
            _ctx: &TenantContext,
            _user_id: UserId,
        ) -> Result<Option<User>, AppError> {
            Ok(None)
        }
        async fn get_capabilities(
            &self,
            _ctx: &TenantContext,
            _user_id: UserId,
        ) -> Result<CapabilitySet, AppError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(self.capabilities.iter().copied().collect())
        }
    }

    fn ctx() -> TenantContext {
        TenantContext::new_for_test(TenantId::new(Uuid::new_v4()), "test")
    }

    #[tokio::test]
    async fn resolve_caches_after_first_call() {
        let repo = Arc::new(StubRepo {
            capabilities: vec![Capability::PostsCreate],
            call_count: AtomicUsize::new(0),
        });
        let resolver = CapabilityResolver::new(repo.clone());
        let uid = UserId::new(Uuid::new_v4());
        let c = ctx();
        let _ = resolver.resolve(&c, uid).await.unwrap();
        let _ = resolver.resolve(&c, uid).await.unwrap();
        assert_eq!(repo.call_count.load(Ordering::SeqCst), 1, "second call should be cache hit");
    }

    #[tokio::test]
    async fn require_returns_missing_capability_when_absent() {
        let repo = Arc::new(StubRepo {
            capabilities: vec![Capability::PostsCreate],
            call_count: AtomicUsize::new(0),
        });
        let resolver = CapabilityResolver::new(repo);
        let err = resolver
            .require(&ctx(), UserId::new(Uuid::new_v4()), Capability::PostsDelete)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::MissingCapability(Capability::PostsDelete)));
    }

    #[tokio::test]
    async fn require_returns_ok_when_capability_present() {
        let repo = Arc::new(StubRepo {
            capabilities: vec![Capability::PostsCreate, Capability::PostsEdit],
            call_count: AtomicUsize::new(0),
        });
        let resolver = CapabilityResolver::new(repo);
        let result = resolver
            .require(&ctx(), UserId::new(Uuid::new_v4()), Capability::PostsCreate)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn invalidate_clears_cache_entry() {
        let repo = Arc::new(StubRepo {
            capabilities: vec![Capability::PostsCreate],
            call_count: AtomicUsize::new(0),
        });
        let resolver = CapabilityResolver::new(repo.clone());
        let uid = UserId::new(Uuid::new_v4());
        let c = ctx();
        let _ = resolver.resolve(&c, uid).await.unwrap();
        resolver.invalidate(uid).await;
        let _ = resolver.resolve(&c, uid).await.unwrap();
        assert_eq!(repo.call_count.load(Ordering::SeqCst), 2, "after invalidate, DB called again");
    }
}
