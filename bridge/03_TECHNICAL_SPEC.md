# BRIDGE · 03 — TECHNICAL SPEC (cms_pages pilot — read-path в AX)

> Назначение: операционная спека для AI-кодера. Что компилировать, какие dependencies, какие endpoints, какой schema delta, какие тесты, какие perf-budget'ы. Не путать с RFC/ADR (это Phase 2 артефакт); этот файл — техническая база, на которую RFC/ADR ссылаются.
>
> Конституциональный приоритет: `barbie/AX/ENTITY.md` (§2 Four-Layer, §4 стек, §6 Tenant Context, §7 Errors, §11 Perf budget, §18 Security) > эта спека > SITE1 реализация.

---

## 1. Endpoint surface (что AX обслуживает в Phase A)

### 1.1 Public CMS endpoint

```
GET /api/v1/cms/pages/public/by-slug/{slug}?locale=ru
Headers (input):
  X-Tenant-Slug: <slug>          (или subdomain резолв)
  Accept: text/html, application/json   (content negotiation в Phase B; в Phase A — JSON)
  X-Request-Id: <ULID>           (опц., если не задан — middleware генерит)

Headers (output):
  Content-Type: application/json; charset=utf-8         (Phase A)
  Content-Type: text/html; charset=utf-8                (Phase B, когда Leptos SSR подключим)
  Cache-Control: no-store
  X-Request-Id: <propagated>
  X-Stack: ax                    (для diagnostic — отличить от SITE1 в логах)

Response 200 body (Phase A):
{
  "id": "<uuid>",
  "tenantId": "<uuid>",
  "slug": "<page-slug>",
  "locale": "ru",
  "title": "<title>",
  "body": [ /* CmsBlocks discriminated union */ ],
  "status": "published",
  "metaTitle": null | "<str>",
  "metaDescription": null | "<str>",
  "coverImageKey": null | "<s3-key>",
  "publishedAt": "<iso8601>",
  "createdAt": "<iso8601>",
  "updatedAt": "<iso8601>"
}

Response 404 body:
{
  "code": "PAGE_NOT_FOUND",
  "slug": "<slug>",
  "locale": "<locale>"
}

Response 500 body (any internal error):
{
  "code": "INTERNAL_ERROR",
  "requestId": "<ULID>"            (для пользователя — единственный handle для support)
}
```

### 1.2 Health/diagnostic endpoints

```
GET /health                  → 200 "ok\n"  (для Caddy probe)
GET /api/v1/version          → 200 { "version": "0.1.0", "git_sha": "<sha>", "build_time": "..." }
GET /debug/dump              → 200 JSON snapshot (только в debug build; production gate'нет на feature flag)
```

### 1.3 Out of scope для Phase A

- write/update/delete `/api/v1/cms/pages` — SITE1 canonical writer
- `/api/v1/auth/*` — AX переиспользует JWT, выпущенный SITE1, верификация через shared secret (`JWT_SECRET` env)
- `/api/v1/admin/*` — UI остаётся в Next.js
- Любые другие модули (services, salons, …)

---

## 2. Schema delta (expand-only, `migrations/0001_cms_pages_expand.sql`)

```sql
-- 0001_cms_pages_expand.sql
-- Stage: AX Phase A pilot — cms_pages read-path
-- Owner: SITE1 (canonical writer для cms_pages таблицы)
-- AX: read-only через cms_pages_v_active + RLS

-- 1) Read-only view — публичный published-only срез
CREATE OR REPLACE VIEW cms_pages_v_active AS
SELECT
  id,
  tenant_id,
  slug,
  locale,
  title,
  body,
  status,
  meta_title,
  meta_description,
  cover_image_key,
  published_at,
  created_at,
  updated_at
FROM cms_pages
WHERE status = 'published';

-- 2) RLS — обязательная политика на view + базовую таблицу для безопасности RLS

-- Если RLS ещё не включён на cms_pages (SITE1 в Phase 0 пока не включал) — включаем
ALTER TABLE cms_pages ENABLE ROW LEVEL SECURITY;

-- Политика: только row'ы с tenant_id = текущий tenant context
CREATE POLICY rls_cms_pages_tenant_isolation
  ON cms_pages
  USING (tenant_id = current_setting('app.current_tenant_id', true)::uuid);

-- Для SITE1 — отдельная BYPASS-роль (SITE1 admin pool ходит как роль, у которой RLS bypass)
-- Это позволяет SITE1 admin endpoints читать все тенанты без SET LOCAL
CREATE ROLE site1_admin_role NOLOGIN;
GRANT ALL ON cms_pages TO site1_admin_role;
ALTER ROLE site1_admin_role BYPASSRLS;
-- SITE1 PgBouncer pool привязан к site1_admin_role; AX pool — к обычной роли с RLS

-- 3) GRANT на view для AX-роли (без BYPASSRLS)
CREATE ROLE ax_app_role NOLOGIN;
GRANT SELECT ON cms_pages_v_active TO ax_app_role;
GRANT SELECT ON cms_pages TO ax_app_role;
-- AX подключается через PgBouncer с этой ролью

-- 4) Application-level setting для контекста (используется в RLS policy + with_tenant)
-- (нет миграции — `SET LOCAL app.current_tenant_id` выполняется в каждой транзакции)

-- 5) Опциональная колонка для cache invalidation (если будем cache'ить SSR результат)
-- Можно опустить в Phase A, добавить отдельной миграцией если понадобится
-- ALTER TABLE cms_pages ADD COLUMN IF NOT EXISTS ax_render_cache_key TEXT NULL;

-- 6) Verification: N-1 compat
-- После applying, SITE1 на старой схеме читает cms_pages без проблем:
-- - view добавлен, не меняет существующее
-- - RLS включён, но site1_admin_role BYPASSRLS — SITE1 не замечает
-- - новой колонки нет (или nullable если добавили)
-- Verify: psql -c "SELECT 1 FROM cms_pages LIMIT 1" (от SITE1 роли) — должен вернуть row
```

**Rollback policy для этой миграции:**

- DROP VIEW можно (idempotent), но не нужно — view не мешает SITE1
- DROP POLICY — **опасно**, потому что лишает RLS; вместо этого revert через ALTER TABLE ... DISABLE ROW LEVEL SECURITY если RLS взаимодействует плохо с PgBouncer
- ROLE — оставить, безвредно
- На практике: migration НЕ откатывается, фиксы forward-only

---

## 3. Cargo workspace + dependencies (минимум для Phase A)

### 3.1 Root `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = ["crates/*", "xtask"]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.84"
license = "Proprietary"
publish = false

[workspace.dependencies]
# core async / HTTP
tokio = { version = "1.42", features = ["rt-multi-thread", "macros", "signal", "time", "fs"] }
axum = { version = "0.8", features = ["macros", "http2"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["trace", "cors", "compression-gzip", "request-id"] }
hyper = "1.5"

# DB
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio-rustls", "uuid", "chrono", "json", "macros"] }

# serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# types
uuid = { version = "1.11", features = ["v4", "v7", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
ulid = "1.1"

# validation
garde = { version = "0.20", features = ["derive", "url", "regex"] }

# errors
thiserror = "2"
eyre = "0.6"

# observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-opentelemetry = "0.28"
opentelemetry = "0.27"
opentelemetry-otlp = "0.27"

# config
config = "0.14"
dotenvy = "0.15"

# leptos (Phase B; Phase A может ограничиться feature-flag)
leptos = { version = "0.7", features = ["ssr"] }
leptos_axum = "0.7"

# dev
tokio-test = "0.4"
testcontainers = "0.23"
testcontainers-modules = { version = "0.11", features = ["postgres"] }
proptest = "1.5"
rstest = "0.23"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "symbols"
panic = "abort"   # см. AX §10.5 — separate runtimes, abort предпочтительнее в production

[profile.dev]
opt-level = 0
debug = true
```

### 3.2 Crate manifests (только cms-relevant deps)

`crates/common/Cargo.toml`:
```toml
[package]
name = "ax-common"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
ulid = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
axum = { workspace = true }   # для IntoResponse on AppError
http = "1"
```

`crates/domain/Cargo.toml`:
```toml
[package]
name = "ax-domain"
version.workspace = true
edition.workspace = true

[dependencies]
ax-common = { path = "../common" }
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
garde = { workspace = true }
# ZERO: no sqlx, no axum, no tokio
```

`crates/application/Cargo.toml`:
```toml
[package]
name = "ax-application"
version.workspace = true
edition.workspace = true

[dependencies]
ax-common = { path = "../common" }
ax-domain = { path = "../domain" }
async-trait = "0.1"
serde = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
# ZERO: no sqlx, no axum
```

`crates/infrastructure/Cargo.toml`:
```toml
[package]
name = "ax-infrastructure"
version.workspace = true
edition.workspace = true

[dependencies]
ax-common = { path = "../common" }
ax-domain = { path = "../domain" }
ax-application = { path = "../application" }
sqlx = { workspace = true }
tokio = { workspace = true }
async-trait = "0.1"
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
tracing = { workspace = true }
eyre = { workspace = true }
```

`crates/presentation/Cargo.toml`:
```toml
[package]
name = "ax-presentation"
version.workspace = true
edition.workspace = true

[dependencies]
ax-common = { path = "../common" }
ax-application = { path = "../application" }
ax-infrastructure = { path = "../infrastructure" }   # только для DI factory
axum = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
ulid = { workspace = true }
http = "1"
base64 = "0.22"   # для ?td= decode

[features]
default = []
leptos-ssr = ["dep:leptos", "dep:leptos_axum"]   # Phase B feature

[dependencies.leptos]
workspace = true
optional = true

[dependencies.leptos_axum]
workspace = true
optional = true
```

**Boundary enforcement:** `cargo xtask architecture-check` парсит каждый `Cargo.toml` + проверяет, что:
- `domain` зависит только от `ax-common` + permitted external (serde, uuid, chrono, garde)
- `application` → не зависит от `sqlx`, `axum`, `tower`
- `presentation` → не зависит напрямую от `sqlx` (только через `ax-application` traits)

---

## 4. Port traits (Application layer contract)

`crates/application/src/ports/cms.rs`:

```rust
use async_trait::async_trait;
use ax_common::{TenantContext, AppError};
use ax_domain::cms::{PublishedPage, PageSlug, PageLocale};

#[async_trait]
pub trait CmsRepository: Send + Sync {
    /// Returns the published page for given (tenant, slug, locale), or NotFound.
    ///
    /// Tenant isolation:
    /// - Implementation MUST call `with_tenant(pool, ctx, |tx| ...)` so RLS context is set.
    /// - Implementation MUST query `cms_pages_v_active` view (not `cms_pages` table) —
    ///   this enforces published-only semantics at SQL level, not application level.
    /// - Cross-tenant request MUST return `AppError::NotFound`, never `AppError::Forbidden`.
    async fn find_published_by_slug(
        &self,
        ctx: &TenantContext,
        slug: &PageSlug,
        locale: PageLocale,
    ) -> Result<PublishedPage, AppError>;
}

#[async_trait]
pub trait CmsRenderer: Send + Sync {
    /// Renders a PublishedPage to HTML string suitable for HTTP response body.
    ///
    /// Inputs:
    /// - page: validated aggregate from CmsRepository
    /// - design_tokens: per-tenant theme (CSS-vars source)
    /// - td_overrides: optional preview overrides (from ?td=base64)
    ///
    /// Output: complete HTML document with <link rel="stylesheet" Google Fonts>,
    /// inline <style> with CSS-vars, <body> with rendered sections.
    ///
    /// Phase A: this trait MAY return JSON snapshot of intermediate representation
    /// (DesignedPage struct) — fully migrating SSR to Leptos is Phase B.
    fn render_html(
        &self,
        page: &PublishedPage,
        tokens: &TenantDesignTokens,
        overrides: Option<&TdOverrides>,
    ) -> Result<String, AppError>;
}
```

---

## 5. Use case (Application layer business logic)

`crates/application/src/use_cases/cms/get_published_by_slug.rs`:

```rust
use std::sync::Arc;
use ax_common::{TenantContext, AppError};
use ax_domain::cms::{PageSlug, PageLocale, PublishedPage};
use crate::ports::CmsRepository;

pub struct GetPublishedBySlug {
    repo: Arc<dyn CmsRepository>,
}

impl GetPublishedBySlug {
    pub fn new(repo: Arc<dyn CmsRepository>) -> Self {
        Self { repo }
    }

    #[tracing::instrument(skip(self), fields(tenant = %ctx.tenant_id, slug = %slug.as_str(), locale = ?locale))]
    pub async fn execute(
        &self,
        ctx: &TenantContext,
        slug: &PageSlug,
        locale: PageLocale,
    ) -> Result<PublishedPage, AppError> {
        // No business logic beyond delegating — this use case is intentionally thin.
        // FUTURE: add caching layer here (lookup → put in TenantContext-scoped cache)
        // FUTURE: add tracing event for cache hit/miss
        self.repo.find_published_by_slug(ctx, slug, locale).await
    }
}
```

**Note:** use case **тонкий** — это нормально. Если позже добавится бизнес-логика (например, проверка feature flag тенанта, A/B-вариант страницы) — она ляжет именно сюда, а не в repo и не в handler.

---

## 6. Infrastructure adapter (SQLx impl)

`crates/infrastructure/src/persistence/cms_pages_repo.rs`:

```rust
use async_trait::async_trait;
use sqlx::PgPool;
use ax_common::{TenantContext, AppError};
use ax_application::ports::CmsRepository;
use ax_domain::cms::{PublishedPage, PageId, PageSlug, PageLocale, PageStatus, Block};

use crate::persistence::transaction::with_tenant;

pub struct PgCmsRepository {
    pool: PgPool,
}

impl PgCmsRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl CmsRepository for PgCmsRepository {
    async fn find_published_by_slug(
        &self,
        ctx: &TenantContext,
        slug: &PageSlug,
        locale: PageLocale,
    ) -> Result<PublishedPage, AppError> {
        with_tenant(&self.pool, ctx, |tx| Box::pin(async move {
            // sqlx::query_as! — compile-time проверка типов; `.sqlx/` мета закоммичена,
            // CI работает offline (см. AX §16)
            let row = sqlx::query_as!(
                CmsPageRow,
                r#"
                SELECT id, tenant_id, slug, locale, title, body as "body: sqlx::types::Json<Vec<Block>>",
                       status, meta_title, meta_description, cover_image_key,
                       published_at, created_at, updated_at
                FROM cms_pages_v_active
                WHERE slug = $1 AND locale = $2
                LIMIT 1
                "#,
                slug.as_str(),
                locale.as_str(),
            )
            .fetch_optional(&mut **tx)
            .await
            .map_err(AppError::Database)?;

            match row {
                Some(r) => Ok(map_row_to_aggregate(r)),
                None => Err(AppError::NotFound(NotFoundDetail {
                    code: "PAGE_NOT_FOUND",
                    fields: serde_json::json!({ "slug": slug.as_str(), "locale": locale.as_str() }),
                })),
            }
        })).await
    }
}

#[derive(Debug)]
struct CmsPageRow {
    id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    slug: String,
    locale: String,
    title: String,
    body: sqlx::types::Json<Vec<Block>>,
    status: String,
    meta_title: Option<String>,
    meta_description: Option<String>,
    cover_image_key: Option<String>,
    published_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

fn map_row_to_aggregate(r: CmsPageRow) -> PublishedPage {
    // Validated mapping: row → domain aggregate.
    // Любая невалидность здесь — internal error (DB schema diverged from domain);
    // НЕ trust input, проверяем invariants
    PublishedPage::reconstitute(/* arguments here */)
        .expect("DB row must satisfy aggregate invariants; schema drift")
}
```

**Query budget (`AX/ENTITY.md §11.6`):**
- ✓ Не `SELECT *` — все колонки явно
- ✓ Не `OFFSET` — single-row lookup
- ✓ Использует составной индекс `cms_pages_tenant_slug_locale_uniq` (после фильтрации tenant_id RLS-ом)
- EXPLAIN ANALYZE snapshot в `docs/perf/explain/cms_pages_get_published.txt` после первой реализации

---

## 7. Presentation handler

`crates/presentation/src/api/cms_handlers.rs`:

```rust
use std::sync::Arc;
use axum::{extract::{Path, Query, State}, response::{IntoResponse, Response}, http::StatusCode, Json};
use ax_common::{TenantContext, AppError};
use ax_application::use_cases::cms::GetPublishedBySlug;
use ax_domain::cms::{PageSlug, PageLocale};
use crate::app_state::AppState;
use crate::extractors::TenantContextExt;   // custom extractor reading from req.extensions

#[derive(serde::Deserialize)]
pub struct LocaleQuery {
    locale: Option<String>,
}

#[derive(serde::Serialize)]
pub struct CmsPageResponse {
    id: String,
    tenant_id: String,
    slug: String,
    locale: String,
    title: String,
    body: Vec<Block>,
    status: String,
    meta_title: Option<String>,
    meta_description: Option<String>,
    cover_image_key: Option<String>,
    published_at: String,
    created_at: String,
    updated_at: String,
}

#[axum::debug_handler]
pub async fn get_published_by_slug(
    State(state): State<AppState>,
    TenantContextExt(ctx): TenantContextExt,
    Path(slug): Path<String>,
    Query(q): Query<LocaleQuery>,
) -> Result<Json<CmsPageResponse>, AppError> {
    let slug = PageSlug::parse(&slug).map_err(AppError::Validation)?;
    let locale = PageLocale::parse(q.locale.as_deref().unwrap_or("ru"))
        .map_err(AppError::Validation)?;

    let page = state.use_cases.get_published_by_slug
        .execute(&ctx, &slug, locale)
        .await?;

    Ok(Json(map_to_response(page)))
}
```

---

## 8. Router wiring

`crates/presentation/src/lib.rs` (упрощённый bootstrap):

```rust
use axum::{Router, routing::get};
use tower_http::{trace::TraceLayer, request_id::SetRequestIdLayer};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/cms/pages/public/by-slug/:slug", get(api::cms_handlers::get_published_by_slug))
        .route("/health", get(|| async { "ok" }))
        .route("/api/v1/version", get(api::version_handler))
        .layer(axum::middleware::from_fn(middleware::tenant_resolver::tenant_resolver))
        .layer(axum::middleware::from_fn(middleware::request_id::request_id))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
```

---

## 9. Tests

### 9.1 Integration test (testcontainers)

`tests/integration/cms_pages_test.rs`:

```rust
use testcontainers_modules::postgres::Postgres;
use testcontainers::{runners::AsyncRunner, ContainerAsync};

struct TestContext {
    pg: ContainerAsync<Postgres>,
    pool: PgPool,
    tenant_a: TenantId,
    tenant_b: TenantId,
}

#[tokio::test]
async fn published_page_returns_200_for_owning_tenant() {
    let ctx = TestContext::setup().await;
    ctx.seed_page(ctx.tenant_a, "home", PageStatus::Published).await;

    let repo = PgCmsRepository::new(ctx.pool.clone());
    let result = repo.find_published_by_slug(
        &TenantContext { tenant_id: ctx.tenant_a, ..ctx.default_ctx() },
        &PageSlug::parse("home").unwrap(),
        PageLocale::Ru,
    ).await.unwrap();

    assert_eq!(result.slug.as_str(), "home");
    assert_eq!(result.tenant_id, ctx.tenant_a);
}

#[tokio::test]
async fn cross_tenant_returns_not_found() {
    let ctx = TestContext::setup().await;
    ctx.seed_page(ctx.tenant_a, "secret", PageStatus::Published).await;

    // tenant_b пытается прочитать tenant_a page
    let repo = PgCmsRepository::new(ctx.pool.clone());
    let result = repo.find_published_by_slug(
        &TenantContext { tenant_id: ctx.tenant_b, ..ctx.default_ctx() },
        &PageSlug::parse("secret").unwrap(),
        PageLocale::Ru,
    ).await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn draft_status_returns_not_found_even_for_owning_tenant() {
    // View `cms_pages_v_active` фильтрует на status='published'
}

#[tokio::test]
async fn archived_status_returns_not_found() { /* same */ }
```

### 9.2 Tenant isolation fuzz

`tests/fuzz/cms_tenant_isolation.rs`:

```rust
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1_000_000,   // 1M attempts per AX §12.6.6 G5
        timeout: 600_000,    // 10 min budget
        ..ProptestConfig::default()
    })]

    #[test]
    fn cross_tenant_never_leaks(
        attacker_tenant_idx in 0usize..10,
        victim_tenant_idx in 0usize..10,
        slug in "[a-z0-9-]{1,255}",
    ) {
        prop_assume!(attacker_tenant_idx != victim_tenant_idx);
        // ... seed victim page, attacker fetch → MUST be 404 or "not found";
        // assert no row from victim_tenant returned
    }
}
```

### 9.3 Performance benchmark vs SITE1

`benches/cms_pages_get_published.rs` (criterion):

```rust
// Дополнительно: запустить oha против реального API:
// $ oha -n 10000 -c 100 -H "X-Tenant-Slug: pilot-tenant-1" http://ax:7000/api/v1/cms/pages/public/by-slug/home?locale=ru
// $ oha -n 10000 -c 100 -H "X-Tenant-Slug: pilot-tenant-1" http://site1:3010/v1/cms/pages/public/by-slug/home?locale=ru
// Сравнить p50/p95/p99 + RSS пиков.
```

---

## 10. Performance budget (success criteria из RFC)

| Метрика | SITE1 baseline (Phase 1 audit будет точное) | AX target (Phase A) |
|---------|--------------------------------------------|---------------------|
| GET p50 | ~50ms | ≤ 20ms |
| GET p95 | ~120ms | ≤ 60ms |
| GET p99 | ~250ms | ≤ 100ms |
| RSS под 100 RPS | ~500MB Node | ≤ 200MB |
| RSS / RPS ratio | baseline | ≤ 50% baseline |
| Cold start | ~3s Node + Next | ≤ 200ms Rust release build |
| Alloc per request | (не измерено в Node) | ≤ 50KB (dhat-rs limit) |

**Не gate**, но **отслеживается:**
- CPU usage под нагрузкой
- DB connection pool saturation
- p99/p50 ratio (≤ 5× — иначе tail latency болеет)

---

## 11. Observability (Phase A minimum)

### 11.1 Tracing spans

```rust
#[tracing::instrument(skip(state), fields(tenant = %ctx.tenant_id, slug = %slug.as_str()))]
async fn get_published_by_slug(...) -> Result<...> { }
```

Span attributes:
- `tenant_id` (UUID)
- `tenant_slug` (string)
- `request_id` (ULID)
- `cms.page.slug`
- `cms.page.locale`
- `cms.page.status` (для родительского span'а)

### 11.2 Metrics (PromQL names)

```
nas_request_duration_seconds{stack="ax", route, status_class}       histogram
nas_request_total{stack="ax", route, status_class}                  counter
nas_alloc_bytes_per_request{stack="ax", route}                      histogram
nas_db_pool_active{stack="ax", pool="api"}                          gauge
nas_tenant_mismatch_total{stack="ax"}                                counter  -- ALERT > 0
nas_cms_page_not_found_total{stack="ax", reason="cross_tenant|draft|missing"}  counter
```

### 11.3 Logs

- Format: JSON
- Level: `info` for handler entry/exit; `warn` for 4xx; `error` for 5xx
- Mandatory fields: `timestamp`, `level`, `target`, `request_id`, `tenant_id` (when known), `route`, `status`, `duration_ms`

### 11.4 Sentry

- Capture все `AppError::Internal` + `AppError::Database`
- **`AppError::TenantMismatch`** — high severity, отдельный alert, tag'и `tenant_id`, `user_id`, `request_id`
- Tag всех events: `stack: ax`, `migration_phase: pilot|stage|ga`

---

## 12. Caddy routing (cutover config)

`ops/caddy/Caddyfile.snippets/cms-ax-pilots.caddy`:

```caddy
# Pilot tenant whitelist для AX CMS read-path
# Routing logic:
#   - GET /<slug>/<page> для whitelisted tenants → AX
#   - всё остальное (POST, write, /admin, не-whitelisted tenants) → SITE1

(cms_ax_pilots) {
  @ax_cms_pilots {
    host pilot-tenant-1.spa.me
    method GET
    not path /admin/* /api/v1/admin/* /api/v1/cms/pages/*  # admin + write пути → SITE1
  }
  handle @ax_cms_pilots {
    reverse_proxy ax-server:7000 {
      health_uri /health
      health_interval 5s
      health_timeout 1s
      fail_duration 30s
      max_fails 3
      transport http {
        keepalive 30s
        keepalive_idle_conns 10
      }
    }
  }
}

# В основном Caddyfile:
# spa.me, *.spa.me {
#   import cms_ax_pilots
#   reverse_proxy site1-web:3011         # fallback всё, что не AX
# }
```

---

## 13. Definition of Done — Phase A pilot

PR на cms_pages в AX считается готовым к Phase A (1 pilot tenant), когда:

- [ ] `cargo build --workspace --release` — green
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — green
- [ ] `cargo test --workspace` — green
- [ ] `cargo sqlx prepare --workspace --check` — green (offline meta committed)
- [ ] `cargo deny check` — green
- [ ] `cargo udeps --workspace` — нет неиспользуемых deps
- [ ] `cargo xtask architecture-check` — green (boundary не нарушен)
- [ ] Integration test (testcontainers) проходит локально
- [ ] Fuzz test 1M attempts — 0 leaks (можно меньше в CI, full run nightly)
- [ ] EXPLAIN ANALYZE snapshot в `docs/perf/explain/cms_pages_get_published.txt`
- [ ] oha baseline + AX result — committed в `docs/perf/`
- [ ] Caddy config готов (даже если deploy позже)
- [ ] Grafana dashboard JSON exported в `ops/observability/dashboards/`
- [ ] `docs/releases/v0.1.0/ROLLBACK.md` + `ROLLBACK_DRILL.md` (drill timestamp < 5 min)
- [ ] `docs/interop/OWNERSHIP.md` обновлён: cms_pages canonical=SITE1, reader=oba
- [ ] All 4 Phase 2 артефакта (RFC/ADR/Plan/Validation) sign'ed off
- [ ] `STATUS.md` обновлён с реальным состоянием pre-conditions

Если хоть один пункт не выполнен — это **PoC**, не Phase A pilot. Зафиксировать в `STATUS.md` явно.
