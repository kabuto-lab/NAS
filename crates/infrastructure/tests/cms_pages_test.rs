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
    /// Spin up Postgres container, apply schema + migration, seed test data.
    ///
    /// Per VAL-001 § Integration tests. Requires Docker daemon — fails fast
    /// если Docker недоступен.
    async fn setup() -> Self {
        // 1. Start Postgres container
        let container = Postgres::default()
            .start()
            .await
            .expect("postgres container start (Docker daemon required)");
        let host = container.get_host().await.expect("container host");
        let port = container
            .get_host_port_ipv4(5432)
            .await
            .expect("container port mapping");
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

        // 2. Connect as superuser (без RLS enforcement в seed phase)
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(&url)
            .await
            .expect("pool connect");

        // 3. Apply baseline schema (минимальный subset SITE1 0000 для cms_pages
        //    deps: tenants + users + cms_pages с indexes + FK)
        apply_baseline_schema(&pool).await;

        // 4. Apply 0001_cms_pages_expand.sql (view + RLS POLICY + 2 roles)
        let migration_sql = include_str!("../../../migrations/0001_cms_pages_expand.sql");
        sqlx::raw_sql(migration_sql)
            .execute(&pool)
            .await
            .expect("apply 0001_cms_pages_expand.sql");

        // 5. Seed 2 tenants + pages
        let tenant_a = TenantId::new(Uuid::new_v4());
        let tenant_b = TenantId::new(Uuid::new_v4());
        seed_tenant(&pool, tenant_a, "tenant-a").await;
        seed_tenant(&pool, tenant_b, "tenant-b").await;
        seed_pages(&pool, tenant_a).await;
        seed_pages(&pool, tenant_b).await;

        Self {
            _container: container,
            pool,
            tenant_a,
            tenant_b,
        }
    }

    #[allow(clippy::unused_self)]
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

/// Minimal subset SITE1 0000_deep_gamma_corps.sql — только то, на что зависит
/// cms_pages (tenants, users — FK targets — и cms_pages с composite indexes).
/// Воспроизведено программно вместо import'а 600-строчного SITE1 SQL.
#[allow(dead_code)]
async fn apply_baseline_schema(pool: &sqlx::PgPool) {
    sqlx::raw_sql(
        r"
        CREATE TABLE IF NOT EXISTS tenants (
            id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
            slug        varchar(64) NOT NULL UNIQUE,
            name        varchar(255) NOT NULL,
            status      varchar(20) NOT NULL DEFAULT 'active',
            contact_email varchar(320) NOT NULL DEFAULT 'noreply@example.com',
            timezone    varchar(64) NOT NULL DEFAULT 'UTC',
            locale      varchar(8) NOT NULL DEFAULT 'ru',
            created_at  timestamp NOT NULL DEFAULT now(),
            updated_at  timestamp NOT NULL DEFAULT now()
        );

        CREATE TABLE IF NOT EXISTS users (
            id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
            email       varchar(320) NOT NULL UNIQUE,
            created_at  timestamp NOT NULL DEFAULT now()
        );

        CREATE TABLE IF NOT EXISTS cms_pages (
            id                  uuid PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id           uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            slug                varchar(255) NOT NULL,
            locale              varchar(8) NOT NULL DEFAULT 'ru',
            title               varchar(500) NOT NULL,
            body                jsonb NOT NULL DEFAULT '[]'::jsonb,
            status              varchar(20) NOT NULL DEFAULT 'draft',
            meta_title          varchar(255),
            meta_description    text,
            cover_image_key     varchar(500),
            author_user_id      uuid REFERENCES users(id) ON DELETE SET NULL,
            published_at        timestamp,
            created_at          timestamp NOT NULL DEFAULT now(),
            updated_at          timestamp NOT NULL DEFAULT now()
        );

        CREATE UNIQUE INDEX IF NOT EXISTS cms_pages_tenant_slug_locale_uniq
            ON cms_pages (tenant_id, slug, locale);
        CREATE INDEX IF NOT EXISTS cms_pages_tenant_status_idx
            ON cms_pages (tenant_id, status);
        CREATE INDEX IF NOT EXISTS cms_pages_tenant_published_idx
            ON cms_pages (tenant_id, published_at DESC NULLS LAST)
            WHERE status = 'published';
        ",
    )
    .execute(pool)
    .await
    .expect("apply baseline schema");
}

#[allow(dead_code)]
async fn seed_tenant(pool: &sqlx::PgPool, id: TenantId, slug: &str) {
    sqlx::query(
        "INSERT INTO tenants (id, slug, name, status) VALUES ($1, $2, $3, 'active')",
    )
    .bind(id.0)
    .bind(slug)
    .bind(format!("Test tenant {slug}"))
    .execute(pool)
    .await
    .expect("seed tenant");
}

/// Seed 5 pages per tenant:
/// - home (Ru, published)
/// - about (Ru, published)
/// - draft-page (Ru, draft) — для F10 test
/// - old-page (Ru, archived) — для F10 test
/// - secret (Ru, published) — для cross-tenant tests
#[allow(dead_code)]
async fn seed_pages(pool: &sqlx::PgPool, tenant_id: TenantId) {
    let now = Utc::now();
    let pages = [
        ("home", "Home", "published", Some(now)),
        ("about", "About", "published", Some(now)),
        ("draft-page", "Draft", "draft", None),
        ("old-page", "Old", "archived", Some(now)),
        ("secret", "Secret", "published", Some(now)),
    ];
    for (slug, title, status, published_at) in pages {
        sqlx::query(
            "INSERT INTO cms_pages (tenant_id, slug, locale, title, status, published_at) \
             VALUES ($1, $2, 'ru', $3, $4, $5)",
        )
        .bind(tenant_id.0)
        .bind(slug)
        .bind(title)
        .bind(status)
        .bind(published_at)
        .execute(pool)
        .await
        .expect("seed page");
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
