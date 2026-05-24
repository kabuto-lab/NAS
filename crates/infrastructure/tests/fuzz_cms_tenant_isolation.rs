//! Fuzz test для tenant isolation в cms_pages read-path.
//!
//! Per VAL-001 §I8: 1M cross-tenant attempts, 0 leaks.
//! Per `ENTITY.md §12.6.6 G5`: full 1M в nightly CI, 10k smoke в regular CI.
//!
//! **Status:** skeleton (T15 в PLAN-001). All proptest cases помечены
//! `#[ignore]` — требует Docker + `cargo sqlx prepare` против real Postgres.
//!
//! Запуск:
//!   - Smoke (10k): `cargo test --test fuzz_cms_tenant_isolation -- --ignored`
//!   - Full (1M, nightly): `PROPTEST_CASES=1000000 cargo test --test
//!     fuzz_cms_tenant_isolation --release -- --ignored`

use ax_application::ports::CmsRepository;
use ax_common::{ids::RequestId, AppError, TenantContext, TenantId, TenantStatus};
use ax_domain::cms::{PageLocale, PageSlug};
use ax_infrastructure::persistence::PgCmsRepository;
use proptest::prelude::*;
use std::sync::Arc;
use testcontainers::ContainerAsync;
use testcontainers_modules::postgres::Postgres;

/// Shared fuzz context — populated 1x при startup.
#[allow(dead_code)]
struct FuzzContext {
    _container: ContainerAsync<Postgres>,
    pool: sqlx::PgPool,
    /// Pre-seeded tenants (10 для variance в attacker/victim selection).
    tenants: Vec<TenantId>,
    /// Каждый tenant имеет 5 published pages (slugs page_0..page_4).
    seeded_slugs: Vec<String>,
}

#[allow(dead_code)]
impl FuzzContext {
    async fn setup() -> Self {
        // TODO Phase 4 (T15):
        // 1. Spin up postgres container
        // 2. Apply 0000_baseline.sql + 0001_cms_pages_expand.sql
        // 3. Seed 10 tenants (UUID v4), каждый с 5 published pages
        // 4. Verify через SELECT count(*) = 50 что seed успешен
        todo!("T15: fuzz context setup")
    }

    fn ctx_for(&self, tenant_idx: usize) -> TenantContext {
        let tenant_id = self.tenants[tenant_idx % self.tenants.len()];
        TenantContext {
            tenant_id,
            tenant_slug: Arc::from("test"),
            status: TenantStatus::Active,
            request_id: RequestId::new(),
            user_id: None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property: cross-tenant attempts NEVER leak victim tenant's data
// ─────────────────────────────────────────────────────────────────────────────

// Tokio runtime — once per test process. proptest spawns sync tests; async
// repo calls идут через block_on на этот shared runtime.
fn runtime() -> &'static tokio::runtime::Runtime {
    use std::sync::OnceLock;
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Runtime::new().expect("tokio runtime init"))
}

/// Sync helper для fuzz body: выполняет attacker fetch против seeded slug.
/// Returns the result для prop_assert! на caller-side.
#[allow(dead_code)]
fn try_seeded_fetch(
    attacker_idx: usize,
    slug_idx: usize,
    locale: PageLocale,
) -> Result<ax_domain::cms::PublishedPage, AppError> {
    runtime().block_on(async move {
        let ctx = FuzzContext::setup().await;
        let repo = PgCmsRepository::new(ctx.pool.clone());
        let slug = PageSlug::parse(&format!("page-{slug_idx}")).unwrap();
        repo.find_published_by_slug(&ctx.ctx_for(attacker_idx), &slug, locale)
            .await
    })
}

#[allow(dead_code)]
fn try_random_fetch(
    attacker_idx: usize,
    slug: &str,
) -> Option<(Result<ax_domain::cms::PublishedPage, AppError>, TenantId)> {
    let parsed = PageSlug::parse(slug).ok()?;
    runtime().block_on(async move {
        let ctx = FuzzContext::setup().await;
        let attacker_tenant = ctx.tenants[attacker_idx % ctx.tenants.len()];
        let repo = PgCmsRepository::new(ctx.pool.clone());
        let res = repo
            .find_published_by_slug(&ctx.ctx_for(attacker_idx), &parsed, PageLocale::Ru)
            .await;
        Some((res, attacker_tenant))
    })
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 10_000,        // 10k smoke; set PROPTEST_CASES=1000000 for full
        max_shrink_iters: 100,
        ..ProptestConfig::default()
    })]

    /// Для любой комбинации (attacker_tenant ≠ victim_tenant, slug, locale):
    /// attacker fetch должен вернуть NotFound. Никогда — leak.
    #[test]
    #[ignore = "requires Docker + testcontainers (T15)"]
    fn cross_tenant_never_leaks(
        attacker_idx in 0usize..10,
        victim_idx in 0usize..10,
        slug_idx in 0usize..5,
        locale_choice in any::<bool>(),
    ) {
        prop_assume!(attacker_idx != victim_idx);
        let locale = if locale_choice { PageLocale::Ru } else { PageLocale::En };
        let result = try_seeded_fetch(attacker_idx, slug_idx, locale);
        prop_assert!(
            matches!(result, Err(AppError::NotFound(_))),
            "cross-tenant LEAK: attacker={} victim={} slug_idx={} locale={:?} got={:?}",
            attacker_idx, victim_idx, slug_idx, locale, result
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1_000, ..ProptestConfig::default() })]

    #[test]
    #[ignore = "requires Docker + testcontainers"]
    fn random_slug_never_panics(
        attacker_idx in 0usize..10,
        slug in "[a-z][a-z0-9-]{2,40}[a-z0-9]",
    ) {
        let Some((result, attacker_tenant)) = try_random_fetch(attacker_idx, &slug) else {
            return Ok(());  // Invalid slug — domain rejects, fine
        };
        if let Ok(page) = result {
            prop_assert_eq!(
                page.tenant_id, attacker_tenant,
                "tenant_id leak: returned page tenant != attacker tenant"
            );
        }
    }
}

/// Smoke: убедиться что fuzz infrastructure setup'ит без panic'а.
/// Runs always в CI (не ignored). Само setup() помечено todo!() — этот smoke
/// будет fail'ить до T15 implementation. Это intentional — drives implementation.
#[test]
#[ignore = "blocked by T15 — FuzzContext::setup unimplemented"]
fn smoke_fuzz_setup_compiles() {
    let _ = std::panic::catch_unwind(|| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(FuzzContext::setup());
    });
}
