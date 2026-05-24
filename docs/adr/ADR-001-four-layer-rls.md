# ADR-001 · Four-Layer architecture + RLS view для cms_pages read-path

| Field | Value |
|-------|-------|
| **Status** | Draft — pending sign-off |
| **Date** | 2026-05-24 |
| **Author** | Claude (AI assistant, session-scoped) |
| **Reviewers required** | user |
| **Phase** | P2 Architectural (`ENTITY.md §2.5`) |
| **Dependent on** | `RFC-001-cms_pages-migration.md` sign-off |
| **Unblocks** | `PLAN-001-cms_pages-pilot.md`, `VAL-001-cms_pages.md` |
| **Reversal cost** | **Low** (DROP VIEW + DISABLE RLS + revert Caddy route, < 5 min) |

---

## Context

`RFC-001` фиксирует *что* мы делаем и *зачем* (migration cms_pages read-path в AX с целями RLS + observability + perf headroom). Этот ADR фиксирует *как* — конкретные архитектурные решения с обоснованием отвергнутых альтернатив.

Audit (`docs/audit/AUDIT-cms_pages-2026-05-24.md`) выявил два **load-bearing structural facts**, которые направляют выбор архитектуры:

1. **SITE1 не имеет RLS** (C1) — миграция вводит защиту впервые, design свободнее чем при "повторить existing"
2. **SITE1 observability ≈ baseline** (C6) — мы не привязаны к существующей метрической модели, можем выбрать canonical AX подход

Plus три architectural constants из audit §10:
- Schema delta — expand-only (no DROP, no ALTER TYPE)
- Response JSON shape — byte-for-byte (без `tenantId`, поле order, null vs missing)
- PgBouncer transaction mode — load-bearing requirement (RFC C2)

---

## Decision

**Принят 4-layer architecture per `ENTITY.md §2` + RLS-enforced read-path через published-only view + role-separated PgBouncer pools.**

### D1 — Four-Layer crate structure

```
crates/
├── common/          L0 — newtype TenantId(Uuid), AppError, IDs, Page<T>
├── domain/          L3 — PublishedPage aggregate, Block enum, value objects
│                       (ZERO deps кроме serde, uuid, chrono, garde)
├── application/     L2 — trait CmsRepository, GetPublishedBySlug use case
│                       (ZERO deps кроме domain + common; NO sqlx, axum, tokio)
├── infrastructure/  L4 — PgCmsRepository impl, with_tenant, S3 adapter
│                       (implements application/ports/, depends on application + domain)
├── presentation/    L1 — Axum handler, middleware, Leptos SSR (Phase B feature flag)
│                       (depends on application + common; uses infrastructure для DI factory)
└── runtime/         TaskSupervisor (§4.8) — Phase 1+ for queue/background work
```

**Enforcement:** `cargo xtask architecture-check` парсит каждый `Cargo.toml` + детектит:
- `domain/` зависит только от: serde, uuid, chrono, garde, thiserror
- `application/` не содержит: sqlx, axum, tower, tokio (за исключением `async_trait`)
- `presentation/` не зависит напрямую от sqlx — только через application/ports/
- `infrastructure/` public API не возвращает sqlx::Row, PgRow, PgPool

CI gate: PR не мерджится если architecture-check падает.

### D2 — RLS implementation strategy

**Schema delta (`migrations/0001_cms_pages_expand.sql`):**

```sql
-- 1. Published-only view (read entry-point для AX)
CREATE OR REPLACE VIEW cms_pages_v_active AS
SELECT
  id, tenant_id, slug, locale, title, body, status,
  meta_title, meta_description, cover_image_key,
  author_user_id, published_at, created_at, updated_at
FROM cms_pages
WHERE status = 'published';

-- 2. RLS на базовой таблице
ALTER TABLE cms_pages ENABLE ROW LEVEL SECURITY;

CREATE POLICY rls_cms_pages_tenant_isolation
  ON cms_pages
  USING (tenant_id = current_setting('app.current_tenant_id', true)::uuid);

-- 3. Role separation
CREATE ROLE site1_admin_role NOLOGIN;
GRANT ALL ON cms_pages TO site1_admin_role;
ALTER ROLE site1_admin_role BYPASSRLS;
-- SITE1 PgBouncer pool — на эту роль; backward compat без изменений в коде SITE1

CREATE ROLE ax_app_role NOLOGIN;
GRANT SELECT ON cms_pages_v_active TO ax_app_role;
GRANT SELECT ON cms_pages TO ax_app_role;
-- НЕТ ALTER ROLE ax_app_role BYPASSRLS — RLS enforce'ится
```

**Why view-on-top, not raw table:**
- View фиксирует **published-only semantic at SQL level** (не application level)
- Audit §1.4 finding: invariants `status='published'` enforce'ятся только TS-кодом → AX полагается на view + ставит published filter в SQL, исчезает класс ошибок "забыл `WHERE status='published'`"
- View не блокирует SITE1 — table остаётся writable; view — read-only

**Why two roles:**
- BYPASSRLS на `site1_admin_role` сохраняет SITE1 behavior bit-for-bit (zero-touch migration)
- `ax_app_role` без BYPASSRLS — даже если AX SQL забыл `tenant_id` filter, POLICY режет
- Чёткое разделение ownership в `docs/interop/OWNERSHIP.md`

### D3 — `with_tenant` transaction helper

`crates/infrastructure/src/persistence/transaction.rs`:

```rust
pub async fn with_tenant<T, F>(
    pool: &PgPool,
    ctx: &TenantContext,
    f: F,
) -> Result<T, AppError>
where
    F: for<'a> FnOnce(&'a mut Transaction<'_, Postgres>)
        -> BoxFuture<'a, Result<T, AppError>>,
{
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL app.current_tenant_id = $1")
        .bind(ctx.tenant_id.0)
        .execute(&mut *tx).await?;
    let result = f(&mut tx).await?;
    tx.commit().await?;
    Ok(result)
}
```

**Invariants:**
- Каждый repo-метод который читает `cms_pages_v_active` или `cms_pages` **обязан** оборачиваться в `with_tenant`
- `SET LOCAL` действует только до COMMIT/ROLLBACK — изоляция между запросами автоматическая
- **PgBouncer ОБЯЗАТЕЛЬНО** в `transaction` pool mode (RFC C2); audit pre-deploy

**Enforcement:** `cargo xtask architecture-check` heuristic — `sqlx::query` macro outside `with_tenant` block в `infrastructure/persistence/` фейлит CI с allowlist для test setup и admin tools.

### D4 — Domain types — Rust enum vs flat struct

Audit §4.3 показал что SITE1 структура `CanvasElement` — **flat struct с optional fields per widget type** (не discriminated union):

```typescript
interface CanvasElement {
  id: string;
  type: 'heading' | 'text' | 'button' | 'divider' | 'spacer' | 'icon-box' | 'cta' | 'image';
  heading?: HeadingProps;
  text?: TextProps;
  button?: ButtonProps;
  // ... 8 widget types, 8 optional fields
  elStyle?: ElStyle;
}
```

**Decision:** AX domain использует **Rust enum** для type safety, **serde maps to flat JSON shape** for contract identity.

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum Widget {
    #[serde(rename = "heading")]
    Heading { #[serde(rename = "heading")] props: HeadingProps },
    #[serde(rename = "text")]
    Text { #[serde(rename = "text")] props: TextProps },
    // ... 8 variants
    #[serde(rename = "icon-box")]
    IconBox { #[serde(rename = "iconBox")] props: IconBoxProps },
    // ⚠ type discriminator "icon-box" с дефисом, но field "iconBox" в camelCase — точно как SITE1
}
```

**Why enum, not flat struct:**
- Compile-time exhaustiveness в `match` для render
- Невозможно создать `CanvasElement { type: 'heading', heading: None }` — illegal state unrepresentable
- Cost: custom serde shape mapping (~30 lines), one-time

**Top-level `Block` discriminated union** (6 типов hero/text/gallery/services/cta/custom) — то же, через `#[serde(tag = "type", content = "data")]`.

### D5 — Rendering rules

**Phase A (JSON only):**
- `IntoResponse for CmsPageResponse` — Axum native `Json<T>` через `serde_json`
- Поле order — declared field order в struct + `#[serde(rename = "...")]` для camelCase
- Optional поля — `Option<T>` с **БЕЗ** `skip_serializing_if = "Option::is_none"` (audit §4.2 contract identity)

**Phase B (HTML, Leptos SSR — feature flag `leptos-ssr`):**

| Decision | Rationale |
|----------|-----------|
| Render полноценный shell (CSS-vars + Google Fonts + style block) | C4 deliberate deviation: SITE1 broken `[slug]` route хардкодит bg — AX рендерит правильно через design tokens |
| Inline styles (НЕ Tailwind, НЕ external CSS) | Audit §6.6 — SITE1 EdRenderer использует `style={{...}}` inline; AX повторяет для byte-similarity (где не противоречит C4) |
| Widget colors **replicate hardcoded** SITE1 mapping (`#00FFCC` primary, etc.) | Это **внутренний инвентарь widget'а**, не "design tokens for tenant". Tenant tokens применяются к outer shell + heading colors. Widget visual ID = SITE1 contract |
| `ammonia` sanitize `text.html` на render | Defence in depth — audit H2; cost low (~5ms per page), безопасность wins |
| `extractEdSections` поведение 1:1 | Audit §6.8 — единственный реальный render path; берёт первый `{type:'custom', data:{ed:[...]}}` блок |

### D6 — Tenant resolution

`crates/presentation/src/middleware/tenant_resolver.rs`:

**Priority (повторяет SITE1 audit §5.1):**
1. Header `X-Tenant-Slug` (приоритет — для тестов и SSR fetch from Next.js)
2. Subdomain `{slug}.{TENANT_ROOT_DOMAIN}` (single-level only — `foo.bar.spa.me` отвергается)
3. Query `?tenant=<slug>` (для SSE / EventSource, легитимно через Caddy)

**Slug regex unified:** `^[a-z0-9][a-z0-9-]{1,62}[a-z0-9]$` (3-64 chars) — фиксит SITE1 H1 bug где resolver принимал только 1-40 chars vs schema CHECK 3-64.

**Cache:** moka LRU `slug → TenantContext`, TTL 5 минут, capacity 10k entries. SITE1 не кэширует (audit §5.1) — AX adds, low-risk perf win.

**`TenantContext` shape (audit §5.1):**
```rust
#[derive(Clone, Copy, Debug)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub tenant_slug: Arc<str>,           // Arc<str> чтобы Clone был cheap
    pub status: TenantStatus,
}
```

**`@Public()` + TenantGuard** equivalent: AX `tenant_resolver` middleware всегда выполняется (status check). Suspended tenant → 403 даже на public endpoint — audit §5.2 finding #10.

### D7 — Error model

```rust
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("not found: {0}")]      NotFound(NotFoundDetail),
    #[error("validation: {0}")]     Validation(#[from] garde::Report),
    #[error("unauthorized")]        Unauthorized,
    #[error("forbidden: {0}")]      Forbidden(String),
    #[error("tenant not resolved")] TenantNotResolved,
    #[error("tenant not active")]   TenantNotActive(TenantStatus),
    #[error("tenant mismatch")]     TenantMismatch,   // ⭐ security event
    #[error("conflict: {0}")]       Conflict(String),
    #[error("bad request: {0}")]    BadRequest(String),
    #[error("rate limited")]        RateLimited,
    #[error("internal: {0}")]       Internal(#[from] eyre::Report),
    #[error("database")]            Database(#[from] sqlx::Error),
}
```

**HTTP status mapping (audit §5.2):**

| Variant | HTTP | JSON body |
|---------|------|-----------|
| `NotFound(d)` | 404 | `{ code: "PAGE_NOT_FOUND", slug, locale }` (byte-for-byte SITE1) |
| `TenantNotResolved` | 401 | `{ code: "TENANT_NOT_RESOLVED" }` |
| `TenantNotActive(s)` | 403 | `{ code: "TENANT_NOT_ACTIVE", status }` |
| `TenantMismatch` | 403 | `{ code: "TENANT_OWNERSHIP_MISMATCH" }` + **Sentry alert** high severity |
| `Validation(r)` | 400 | `{ code: "VALIDATION_FAILED", issues: [...] }` |
| `Conflict(s)` | 409 | `{ code: "CONFLICT", message: s }` |
| `RateLimited` | 429 | `{ code: "RATE_LIMITED" }` |
| `Internal(e)`, `Database(e)` | 500 | `{ code: "INTERNAL_ERROR", requestId }` (Sentry capture) |

### D8 — Observability stack

**Tracing:** `tracing` + `tracing-subscriber` JSON output на stdout.

**Each request span tags:**
- `tenant_id` (UUID)
- `tenant_slug` (string)
- `request_id` (ULID)
- `route` (path)
- `cms.page.slug`, `cms.page.locale`, `cms.page.status`

**OTLP export:** `tracing-opentelemetry` → `opentelemetry-otlp` → Tempo / Honeycomb (env `OTLP_ENDPOINT`).

**Metrics (Prometheus exposition format):**
- `nas_request_duration_seconds{stack="ax", route, status_class}` histogram
- `nas_request_total{stack="ax", route, status_class}` counter
- `nas_alloc_bytes_per_request{stack="ax", route}` histogram (dhat sampling)
- `nas_db_pool_active{stack="ax"}` gauge
- `nas_tenant_mismatch_total{stack="ax"}` counter — **alert > 0**
- `nas_cms_page_not_found_total{stack="ax", reason="cross_tenant|draft|missing"}` counter

**Sentry:** `sentry-tracing` integration. Tag всех events: `stack: ax`, `migration_phase: pilot|stage|ga|retire`. `TenantMismatch` — high severity отдельный alert.

**Health:**
- `GET /health` — `{ ok, db: 'up'|'down', uptime, version, git_sha, env, timestamp }` (extended vs SITE1 with git_sha)
- `GET /health/ready` — DB ping + S3 HEAD bucket + pgmq probe (Phase B); 200 / 503

### D9 — Caddy routing

`ops/caddy/Caddyfile.snippets/cms-ax-pilots.caddy`:

```caddy
(cms_ax_pilots) {
  @ax_cms_pilots {
    host pilot-tenant-1.spa.me
    method GET
    path /api/v1/cms/pages/public/by-slug/*
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
```

**Path matching strict:** только GET `/api/v1/cms/pages/public/by-slug/*`. Любые другие пути (admin, write) → SITE1. Это предотвращает accidental AX serving write requests.

**Rollback procedure:** comment out `import cms_ax_pilots` in main Caddyfile + `caddy reload` = trivial revert. Drill в T17.

### D10 — Dependency choices (locked from ENTITY.md §4 baseline)

| Concern | Crate / version | Why |
|---------|-----------------|-----|
| Async runtime | `tokio` 1.43+ | Stable, mature, AX-canonical |
| HTTP | `axum` 0.8 | Tower ecosystem, extractor model |
| DB | `sqlx` 0.8 | Compile-time SQL validation, `.sqlx/` offline mode для CI |
| Validation | `garde` 0.20 | Derive-friendly, integration с serde |
| Errors (library) | `thiserror` 2 | Standard |
| Errors (app) | `eyre` 0.6 | Readable stacktraces |
| Tracing | `tracing` + `tracing-subscriber` 0.3 | Standard Rust observability |
| OTLP | `opentelemetry-otlp` 0.27 | Vendor-neutral export |
| Sentry | `sentry-tracing` | Auto breadcrumbs из spans |
| Metrics | `metrics` + `metrics-exporter-prometheus` | Standard exposition |
| Cache | `moka` 0.12 | Best-in-class LRU + TTL |
| TLS | `rustls` (через `axum-server`) | NOT openssl — supply-chain hygiene (ENTITY.md §19 ban-list) |
| Allocator | `jemallocator` (prod) | Fragmentation, introspection через `tikv-jemalloc-ctl` |
| HTML sanitize | `ammonia` 4 | Phase B render-time defence (H2) |

**`cargo-deny.toml` enforce'ит:**
- license allowlist: MIT, Apache-2.0, BSD-2/3, ISC, Unicode-DFS-2016, MPL-2.0
- ban list: openssl, openssl-sys, native-tls
- advisories: deny yanked, deny vulnerability

---

## Alternatives considered (rejected)

### A1 — Vertical-slice architecture (modules-by-feature, not by-layer)

**Idea:** `crates/cms/{domain, app, infra, presentation}/`, `crates/tenant/{...}/` — domain-grouped instead of layer-grouped.

**Rejected because:**
- VS easily вырождается в circular deps (cms depends on tenant, tenant depends on cms for menu)
- Layer-based enforce'ит unidirectional flow через Cargo workspace deps — VS требует additional discipline
- Audit §2 фиксирует Four-Layer как ENTITY.md §2 canonical; deviation требовал бы parent constitution change

### A2 — SeaORM / Diesel вместо raw SQLx

**Idea:** ORM с active-record / type-safe query builder.

**Rejected because:**
- SeaORM ломает domain purity — entities имеют `find()`, `save()` методы → mixing data + behavior
- Diesel — slower compile times, weaker async story (Diesel async still experimental в 2026)
- SQLx с `query_as!` + offline metadata в `.sqlx/` уже compile-time validates SQL; покрывает 95% use cases
- Audit §6 confirms: SITE1 cms_pages query — simple SELECT с 4 WHERE conditions, ORM избыточен

### A3 — Strict RLS без view (только POLICY)

**Idea:** AX query `SELECT FROM cms_pages WHERE slug AND locale AND status='published'`, RLS режет cross-tenant.

**Rejected because:**
- Published-only filter — semantic concern, не just security. View enforce'ит SQL-level, не application
- View переиспользуется (admin tools могут читать через view для preview-published-state)
- View — additive change, no risk; убрать позже trivial

### A4 — Postgres LISTEN/NOTIFY для cache invalidation (вместо TTL)

**Idea:** AX подписывается на `tenant_slug_change` notification → invalidate moka entry.

**Rejected because:**
- Phase A scope — accept eventual consistency 5min TTL
- LISTEN/NOTIFY requires persistent connection — complicates connection pool management
- Phase B+ возможно через pgmq event (clean alternative)

### A5 — Replicate broken SITE1 widget colors / hardcoded bg byte-for-byte

**Idea:** AX renders `#00FFCC` primary buttons, `#0E0F12` background — exactly как SITE1.

**Rejected (но reversible) because:**
- SITE1 baseline outright игнорирует tenant'овский bg (audit C3) — это **bug в SITE1**
- Tenant'ы платят за design tokens; обещание identity = обещание сохранить bug
- Cost: maintain hardcoded color map в AX навсегда
- Decision: shell deviates (uses tokens), widget internal colors replicate (`#00FFCC` etc.) — split based on "тенант контролирует" vs "widget внутренний".

### A6 — Eager Leptos SSR в Phase A

**Idea:** Phase A сразу рендерит HTML через Leptos, не JSON.

**Rejected because:**
- Contract identity verification сложнее (HTML diff vs JSON diff)
- Leptos 0.7 — production-readiness ещё развивается; risk concentration в pilot
- Phase A JSON → Phase B HTML — clean staging, можно сравнить incremental
- AX `presentation/` через `feature = "leptos-ssr"` flag — переключаемое без переписывания

### A7 — Single tokio runtime для всего

**Idea:** Не разделять runtime'ы для HTTP / queue / image — один runtime.

**Rejected (заложено в §10.5) — но не имплементируется в Phase A:**
- Phase A — только HTTP (нет queue, нет image processing). Single runtime достаточен.
- Phase B+ — добавляется queue (pgmq consumer) → separate runtime per §10.5
- Foundation в TaskSupervisor crate готова с Phase A, активация — Phase B

---

## Consequences

### Что станет проще

- **Compile-time tenant safety:** функция без `&TenantContext` параметра не может leak'ать данные через type system
- **SQL запросы — compile-time проверены** (`sqlx::query!` + `.sqlx/` offline mode)
- **Boundary violations невозможны** — Cargo workspace deps структурно отрицают
- **Observability rich по дефолту** — каждый request tagged tenant + slug + request_id
- **Read-path enforcement via RLS** — забыли WHERE → POLICY отрезает

### Что станет сложнее

- **Build times** — Rust release builds медленнее (~5 минут на cold workspace vs ~30s tsc); cached < 30s incremental
- **N+1 prevention discipline** — sqlx не имеет dataloader pattern out-of-box; manual batching через `query_as!` в integration tests + query-count assertion
- **Feature flag для Leptos** — `presentation/` с `leptos-ssr` feature; build matrix CI должен покрывать ON/OFF
- **`with_tenant` обёртка обязательна** — programmers must remember; CI architecture-check помогает, но не perfect
- **Custom serde для widget enum → flat JSON** — ~30 lines additional code, one-time cost

### Новые риски

- **R1 PgBouncer transaction mode** (mitigated в RFC)
- **R2 SITE1 invariant drift** (mitigated с integration tests)
- **R3 Visual deviation** (acknowledged в DECISION.md)
- **R4 Caddy reload race** (mitigated с graceful reload)
- **R5 Cross-role RLS misconfig** (mitigated с pre-deploy test)
- **R6 Sentry blind spot** (mitigated с DSN validation at boot)
- **R7 Cache staleness 5min** (accepted в Phase A)

---

## Reversal cost — **LOW**

| Action | Time | Risk |
|--------|------|------|
| Caddy revert (remove `import cms_ax_pilots`, reload) | < 5 sec | Zero — graceful reload drains in-flight |
| Disable AX systemd service | < 5 sec | Zero — Caddy already routes to SITE1 |
| DROP VIEW cms_pages_v_active | < 1 sec | Zero — SITE1 не читает view |
| DISABLE ROW LEVEL SECURITY ON cms_pages | < 1 sec | Zero — site1_admin_role уже BYPASSRLS |
| DROP POLICY rls_cms_pages_tenant_isolation | < 1 sec | Zero |
| DROP ROLE ax_app_role, site1_admin_role | < 1 sec | Если SITE1 connects через site1_admin_role — revert pool config first |

**Full rollback < 5 minutes**, validated через T17 drill. Per `ENTITY.md §12.6.5` rollback procedure.

**Caveat:** если AX живёт в production > 2 недель и SITE1 admin queries начнут опираться на `site1_admin_role` BYPASSRLS — DROP ROLE станет breaking. Mitigation: ROLE остаётся forever (zero cost); только VIEW + POLICY DROP'ятся при revert.

---

## Sign-off

**Required before P3 (PLAN-001) start:**
- [ ] User reviews this ADR
- [ ] D4 enum-vs-flat domain types — confirmed
- [ ] D5 widget hardcoded colors replicate — confirmed
- [ ] D8 metrics namespace `nas_*` — confirmed
- [ ] D10 dependency versions — locked

**Sign-off record:**

| Reviewer | Date | Decision | Notes |
|----------|------|----------|-------|
| _user_ | _pending_ | _pending_ | — |

---

## Related artifacts

- **Audit (input):** `docs/audit/AUDIT-cms_pages-2026-05-24.md`
- **RFC (parent):** `docs/rfc/RFC-001-cms_pages-migration.md`
- **PLAN (next):** `docs/plans/PLAN-001-cms_pages-pilot.md` — to be written after this sign-off
- **VAL (Phase 4):** `docs/validations/VAL-001-cms_pages.md` — to be written before T14
- **Migration:** `migrations/0001_cms_pages_expand.sql` — to be written in T10
- **ENTITY:** `barbie/ax/ENTITY.md` v3.4 (§2 Four-Layer, §3 Multi-tenancy, §4 stack, §6 Tenant context Rust code, §7 Error model, §10 Observability, §11 Perf budget, §12.5 Interop, §12.6 Migration playbook, §18 Security)
- **Parent constitution:** `barbie/ENTITY.md`

---

*ADR complete. Awaiting RFC-001 + this ADR sign-off to unblock PLAN-001.*
