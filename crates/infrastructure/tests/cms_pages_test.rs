//! Integration tests для CMS pages — testcontainers + real Postgres + RLS.
//!
//! Status: **skeleton (T14 в PLAN-001)**. Все тесты помечены `#[ignore]` потому что
//! требуют:
//! 1. Docker daemon running (для testcontainers)
//! 2. `.sqlx/` offline metadata generated (для query_as! macros — Phase B)
//!
//! Запуск локально: `cargo test --test cms_pages_test -- --ignored`.
//!
//! Coverage targets (VAL-001 success criteria):
//! - F1, F2, F5: contract identity (JSON shape, field order, 404 body)
//! - F6: locale default fallback to Ru
//! - F10: only Published returned (draft/archived → 404)
//! - I1: RLS POLICY active
//! - I4, I5: cross-tenant returns NotFound (даже без WHERE tenant_id — RLS режет)
//! - I6: suspended tenant → 403
//! - I8: 1M fuzz (separate test in tests/fuzz/, Phase 4)

#![cfg(feature = "integration-tests")]

use ax_application::ports::CmsRepository;
use ax_common::{ids::RequestId, TenantContext, TenantId, TenantStatus};
use ax_domain::cms::{PageLocale, PageSlug};
use ax_infrastructure::persistence::PgCmsRepository;
use chrono::Utc;
use std::sync::Arc;
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

/// Shared test context: containerized Postgres + applied migrations + seeded
/// tenants + pool.
#[allow(dead_code)]
struct TestContext {
    _container: ContainerAsync<Postgres>,
    pool: sqlx::PgPool,
    tenant_a: TenantId,
    tenant_b: TenantId,
}

#[allow(dead_code)]
impl TestContext {
    async fn setup() -> Self {
        // TODO Phase 4 (T14):
        // 1. Spin up postgres:16 container via testcontainers
        // 2. Apply migrations (0000_*.sql from SITE1 OR equivalent) + 0001_cms_pages_expand.sql
        // 3. Seed tenant_a + tenant_b в `tenants` table
        // 4. Seed minimum 2 cms_pages rows per tenant (published / draft / archived)
        // 5. Return TestContext с pool + tenant ids
        todo!("T14: testcontainers setup — needs 0000_baseline.sql import or programmatic CREATE TABLE")
    }

    fn ctx_for(&self, tenant_id: TenantId) -> TenantContext {
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
// F1, F10 — Published page returns successfully for owning tenant
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker + testcontainers + .sqlx metadata (T14)"]
async fn published_page_returns_200_for_owning_tenant() {
    let ctx = TestContext::setup().await;
    let repo = PgCmsRepository::new(ctx.pool.clone());
    let result = repo
        .find_published_by_slug(
            &ctx.ctx_for(ctx.tenant_a),
            &PageSlug::parse("home").unwrap(),
            PageLocale::Ru,
        )
        .await
        .expect("page found");
    assert_eq!(result.slug.as_str(), "home");
    assert_eq!(result.tenant_id, ctx.tenant_a);
}

// ─────────────────────────────────────────────────────────────────────────────
// I4 — Cross-tenant request returns NotFound (not Forbidden)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker + testcontainers"]
async fn cross_tenant_returns_not_found() {
    let ctx = TestContext::setup().await;
    let repo = PgCmsRepository::new(ctx.pool.clone());
    // tenant_b requests slug that exists in tenant_a
    let result = repo
        .find_published_by_slug(
            &ctx.ctx_for(ctx.tenant_b),
            &PageSlug::parse("home").unwrap(),
            PageLocale::Ru,
        )
        .await;
    assert!(matches!(
        result,
        Err(ax_common::AppError::NotFound(_))
    ));
}

// ─────────────────────────────────────────────────────────────────────────────
// I5 — RLS blocks even when WHERE tenant_id is omitted
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker + testcontainers"]
async fn rls_blocks_missing_tenant_filter() {
    let ctx = TestContext::setup().await;
    // Direct SQL без with_tenant() — должен вернуть 0 rows если RLS active.
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM cms_pages_v_active",
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("query");
    // Без SET LOCAL tenant — POLICY режет всё
    assert_eq!(count, 0, "RLS должен блокировать reads без SET LOCAL");
}

// ─────────────────────────────────────────────────────────────────────────────
// F10 — draft / archived статусы НЕ доступны через view
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker + testcontainers"]
async fn draft_status_returns_not_found() {
    let ctx = TestContext::setup().await;
    let repo = PgCmsRepository::new(ctx.pool.clone());
    let result = repo
        .find_published_by_slug(
            &ctx.ctx_for(ctx.tenant_a),
            &PageSlug::parse("draft-page").unwrap(),
            PageLocale::Ru,
        )
        .await;
    assert!(matches!(result, Err(ax_common::AppError::NotFound(_))));
}

#[tokio::test]
#[ignore = "requires Docker + testcontainers"]
async fn archived_status_returns_not_found() {
    let ctx = TestContext::setup().await;
    let repo = PgCmsRepository::new(ctx.pool.clone());
    let result = repo
        .find_published_by_slug(
            &ctx.ctx_for(ctx.tenant_a),
            &PageSlug::parse("old-page").unwrap(),
            PageLocale::Ru,
        )
        .await;
    assert!(matches!(result, Err(ax_common::AppError::NotFound(_))));
}

// ─────────────────────────────────────────────────────────────────────────────
// F6 — locale parameter defaults to Ru when not specified (in handler)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker + testcontainers"]
async fn locale_default_ru_fallback() {
    let ctx = TestContext::setup().await;
    let repo = PgCmsRepository::new(ctx.pool.clone());
    // Default locale (Ru) should work для tenant_a/home
    let result = repo
        .find_published_by_slug(
            &ctx.ctx_for(ctx.tenant_a),
            &PageSlug::parse("home").unwrap(),
            PageLocale::Ru, // explicit Ru — handler default делает это
        )
        .await;
    assert!(result.is_ok());
}

// ─────────────────────────────────────────────────────────────────────────────
// F2, F3, F4 — JSON contract identity tests (golden file)
// ─────────────────────────────────────────────────────────────────────────────
//
// Эти тесты проверяют serde shape vs golden fixture. Path
// `crates/infrastructure/tests/fixtures/cms_page_response.golden.json`
// должен быть commit'нут после первой реализации (T14).
//
// Test plan:
// 1. Seed known cms_pages row с фиксированными UUID + timestamps
// 2. Fetch через repo + конвертировать в CmsPageResponse
// 3. serde_json::to_value + compare с golden JSON byte-for-byte
//
// Currently — TODO placeholder. См. VAL-001 §F2/F3/F4.

// ─────────────────────────────────────────────────────────────────────────────
// Smoke test без Docker — verify domain invariants (бежит в CI)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn smoke_domain_types_compile() {
    let _ = TenantId::new(Uuid::nil());
    let _ = PageSlug::parse("home").unwrap();
    let _ = PageLocale::Ru;
    let _ = RequestId::new();
    let _ = Utc::now();
}
