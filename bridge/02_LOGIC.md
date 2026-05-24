# BRIDGE · 02 — LOGIC (бизнес-логика и invariants с кодом-источником)

> Назначение: AI-кодеру дать **поведение в работающем виде**, не пересказ. Каждый блок — выдержка из SITE1 + комментарий «как это переезжает в AX».
>
> Конфликт с `barbie/AX/ENTITY.md` невозможен — этот файл вторичен; ENTITY определяет правила, LOGIC показывает текущую реализацию SITE1 как референс. Если SITE1 делает что-то, что ENTITY запрещает — AX делает по ENTITY, не копирует SITE1.

---

## 1. Multi-tenancy — 4 слоя защиты (SITE1) → 5 слоёв (AX)

SITE1 заявляет 4 слоя (`ROADMAP.md §2`):

1. **`TenantGuard`** — декларативный guard на контроллере (после `JwtAuthGuard`).
2. **`withTenant()`/`combineTenant()`** — Drizzle WHERE-injection helper в каждом запросе.
3. **DB constraint** — `tenant_id NOT NULL` + `ON DELETE CASCADE`.
4. **Postgres RLS** — opt-in в Phase 2 (пока **не включён** в SITE1).

AX добавляет 5-й слой по `AX/ENTITY.md §3`: **compile-time newtype `TenantId(Uuid)`** — `Uuid` не появляется в публичных сигнатурах функций, работающих с tenant-scoped данными.

### 1.1 Источник: middleware-резолвер тенанта (SITE1)

`apps/api/src/tenant-context/tenant-resolver.middleware.ts:41-103`:

```typescript
async use(req: Request, _res: Response, next: NextFunction): Promise<void> {
  const slug = this.extractSlug(req);
  if (!slug) return next();   // ← НЕ бросает; guard на endpoint'е сам решит

  const rows = await this.db
    .select({ id: tenants.id, slug: tenants.slug, status: tenants.status })
    .from(tenants)
    .where(and(eq(tenants.slug, slug), isNotNull(tenants.id)))
    .limit(1);

  const tenant = rows[0];
  if (!tenant) return next();

  const ctx: TenantContext = { tenantId: tenant.id, tenantSlug: tenant.slug, status: tenant.status };
  this.ctxService.run(ctx, () => next());   // ALS-propagation
}

private extractSlug(req: Request): string | null {
  // 1) header fallback (приоритет — для тестов и admin tools)
  const headerVal = req.headers[this.fallbackHeader];   // 'x-tenant-slug'
  if (typeof headerVal === 'string' && headerVal.trim().length > 0) {
    return this.normalizeSlug(headerVal);
  }
  // 2) subdomain: {slug}.lvh.me / {slug}.spa.me
  const host = (req.hostname || req.headers.host || '').toString().toLowerCase();
  const cleanHost = host.split(':')[0];
  const suffix = '.' + this.rootDomain;
  if (cleanHost && cleanHost !== this.rootDomain && cleanHost.endsWith(suffix)) {
    const candidate = cleanHost.slice(0, -suffix.length);
    if (candidate && !candidate.includes('.')) return this.normalizeSlug(candidate);
  }
  // 3) ?tenant=<slug> для SSE (EventSource не шлёт custom headers)
  const queryVal = req.query?.tenant;
  if (typeof queryVal === 'string' && queryVal.trim().length > 0) {
    return this.normalizeSlug(queryVal);
  }
  return null;
}
```

**Что повторить в AX (`crates/presentation/src/middleware/tenant_resolver.rs`):**

- Тот же приоритет: header → subdomain → query
- Тот же regex slug нормализации: `^[a-z0-9](?:[a-z0-9-]{0,38}[a-z0-9])?$`
- Тот же subdomain pattern: `{slug}.{ROOT_DOMAIN}`, NOT multi-level (`foo.bar.spa.me` отвергается)
- Кладёт `TenantContext { tenant_id: TenantId, slug: String, status: TenantStatus }` в `req.extensions()` (не в ALS — Tokio task-local если нужно глобально, но extensions достаточно для большинства handler'ов)

**Что AX делает иначе:**

- `TenantContext` — `Clone + Copy`-friendly (см. `AX/ENTITY.md §6`); `slug` — `Arc<str>` если хочется без аллокаций
- Резолвер кэширует tenant в-memory (LRU с TTL), не ходит в БД на каждый запрос — slug→id mapping меняется редко

### 1.2 Источник: withTenant helper (SITE1, Layer 2)

`apps/api/src/tenant-context/with-tenant.helper.ts:34-55`:

```typescript
export function withTenant<T extends SelectQueryBuilder<T>>(
  query: T,
  tenantId: string,
  tenantIdColumn: AnyColumn,
): T {
  return query.where(eq(tenantIdColumn, tenantId));
}

export function combineTenant(
  tenantId: string,
  tenantIdColumn: AnyColumn,
  ...extraConditions: (SQL | undefined)[]
): SQL {
  const conds = [eq(tenantIdColumn, tenantId), ...extraConditions.filter((c): c is SQL => !!c)];
  return and(...conds) as SQL;
}
```

**AX-эквивалент** (`crates/infrastructure/src/persistence/transaction.rs`, дословно из `AX/ENTITY.md §6`):

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

**Ключевое отличие AX:** SITE1 добавляет `WHERE tenant_id = $1` приложением → защита от багов, но не от прямого SQL. AX использует `SET LOCAL app.current_tenant_id` + **Postgres RLS** → даже если разработчик забыл WHERE, RLS политика отрежет чужие row'ы. Это «hard wall» вместо «полиции на дороге».

### 1.3 Источник: применение в сервисе (SITE1)

`apps/api/src/cms/cms.service.ts:127-144` (метод `getPublishedBySlug`):

```typescript
async getPublishedBySlug(slug: string, locale: 'ru' | 'en' = 'ru'): Promise<PageResponseDto> {
  const tenantId = this.tenantContext.requireTenantId();
  const [row] = await this.db
    .select()
    .from(cmsPages)
    .where(
      and(
        eq(cmsPages.tenantId, tenantId),     // ← Layer 2 явно
        eq(cmsPages.slug, slug),
        eq(cmsPages.locale, locale),
        eq(cmsPages.status, 'published'),
      ),
    )
    .limit(1);
  if (!row) {
    // 404 даже если страница есть в другом тенанте — НЕ раскрываем существование
    throw new NotFoundException({ code: 'PAGE_NOT_FOUND', slug, locale });
  }
  return this.toResponse(row);
}
```

**Инварианты, которые AX обязан сохранить (caller-visible):**

1. **Только `published`** — `draft` и `archived` возвращают 404, идентично SITE1.
2. **404 на cross-tenant** — не 403, не «found in other tenant». Чужой tenant_id → как будто страницы вообще нет.
3. **Пара (slug, locale, tenant_id)** уникальна — гарантировано schema (uniqueIndex `cms_pages_tenant_slug_locale_uniq`), AX полагается на этот invariant.
4. **`locale` default `'ru'`** — если не задан, fallback на `ru`. AX делает то же.
5. **Error code `PAGE_NOT_FOUND`** — JSON body `{ "code": "PAGE_NOT_FOUND", "slug": "...", "locale": "..." }`. AX byte-for-byte.

### 1.4 Источник: isolation specs (SITE1, Stage 34)

`apps/api/src/cms/cms.service.spec.ts:69-77`:

```typescript
it('getPublishedBySlug — публичный рендер тенант-filtered', async () => {
  const db = createMockDb();
  db.queueResult([mockPageRow({ status: 'published' })]);
  const service = makeService(db);

  await service.getPublishedBySlug('home', 'ru');

  expectTenantFilter(whereArgsOf(db), cmsPages.tenantId, TENANT_A);
});
```

**AX эквивалент** — `tests/integration/cms_pages_test.rs`:

```rust
#[tokio::test]
async fn published_by_slug_is_tenant_filtered() {
    let ctx = TestContext::new().await;   // testcontainers Postgres + RLS
    let tenant_a = ctx.seed_tenant("a", &[("home", PageStatus::Published)]).await;
    let _tenant_b = ctx.seed_tenant("b", &[("home", PageStatus::Published)]).await;

    let result = ctx.repo()
        .get_published_by_slug(&tenant_a, "home", PageLocale::Ru)
        .await
        .expect("found");

    assert_eq!(result.tenant_id, tenant_a);
    assert_ne!(result.tenant_id, /* tenant_b id */ ctx.tenant_b_id);
}

#[tokio::test]
async fn cross_tenant_attempts_return_not_found() {
    // 1M fuzz из tests/fuzz/cms_tenant_isolation.rs
}
```

**Усиление AX vs SITE1:** mock-based unit-тест проверяет, что `WHERE tenant_id = ?` присутствует. Integration-тест с testcontainers проверяет, что **даже если бы программист забыл** про `tenant_id`, RLS-политика не пустит чужие row'ы. Это два независимых слоя проверки.

---

## 2. CMS lifecycle и invariants

`packages/db/src/schema/cms-pages.ts:33-90`:

```typescript
export type CmsPageStatus = 'draft' | 'published' | 'archived';

export type CmsBlocks = Array<
  | { type: 'hero'; data: { title: string; subtitle?: string; imageKey?: string } }
  | { type: 'text'; data: { html: string } }
  | { type: 'gallery'; data: { mediaIds: string[] } }
  | { type: 'services'; data: { categoryFilter?: string; limit?: number } }
  | { type: 'cta'; data: { label: string; href: string; style?: 'primary' | 'secondary' } }
  | { type: 'custom'; data: Record<string, unknown> }
>;

export const cmsPages = pgTable('cms_pages', {
  id: uuid('id').defaultRandom().primaryKey(),
  tenantId: uuid('tenant_id').references(() => tenants.id, { onDelete: 'cascade' }).notNull(),

  slug: varchar('slug', { length: 255 }).notNull(),
  locale: varchar('locale', { length: 8 }).notNull().default('ru'),
  title: varchar('title', { length: 500 }).notNull(),
  body: jsonb('body').$type<CmsBlocks>().notNull().default(sql`'[]'::jsonb`),
  status: varchar('status', { length: 20 }).$type<CmsPageStatus>().notNull().default('draft'),
  metaTitle: varchar('meta_title', { length: 255 }),
  metaDescription: text('meta_description'),
  coverImageKey: varchar('cover_image_key', { length: 500 }),
  authorUserId: uuid('author_user_id').references(() => users.id, { onDelete: 'set null' }),
  publishedAt: timestamp('published_at'),
  createdAt: timestamp('created_at').defaultNow().notNull(),
  updatedAt: timestamp('updated_at').defaultNow().notNull(),
}, (t) => ({
  tenantSlugLocaleUniq: uniqueIndex('cms_pages_tenant_slug_locale_uniq').on(t.tenantId, t.slug, t.locale),
  tenantStatusIdx: index('cms_pages_tenant_status_idx').on(t.tenantId, t.status),
  tenantPublishedIdx: index('cms_pages_tenant_published_idx')
    .on(t.tenantId, t.publishedAt.desc())
    .where(sql`status = 'published'`),  // ← partial index, load-bearing для public listing
}));
```

**Invariants, переезжающие в AX domain layer (`crates/domain/src/cms/aggregate.rs`):**

1. **Lifecycle FSM:** `draft → published → archived`. Прямые переходы из любого в любое запрещены (только draft↔published, archived terminal). Изначальный статус — `draft` (schema default).
2. **`publishedAt` имеет значение тогда и только тогда, когда `status = 'published'`.** При unpublish — `publishedAt` сбрасывается в `NULL`. При archive — `publishedAt` остаётся последним значением (audit), но page всё равно недоступна.
3. **`slug` immutable** после создания. SITE1 не имеет PATCH endpoint для slug; rename = создать новую страницу + archive старой.
4. **`body` валидируется Zod-схемой** на write (см. `dto/blocks.schema.ts`). AX повторяет через `garde` + `serde`.
5. **Каждый block имеет фиксированный shape по `type`** (discriminated union). `'custom'` — escape hatch, `data: Record<string, unknown>`, используется ED-editor'ом (Stage 28).
6. **Partial index** для `status='published'` — load-bearing для производительности public render. AX **обязан** сохранить семантику; expand migration не должна ронять этот индекс.

### 2.1 Block ED special — `custom` с `data.ed`

`apps/web/src/components/cms/ed-editor/EdRenderer.tsx::extractEdSections()` (упрощённо):

```typescript
export function extractEdSections(body: CmsBlocks): Section[] {
  // M1 формат: один блок { type: 'custom', data: { ed: Section[] } }
  const customBlock = body.find((b) => b.type === 'custom' && b.data?.ed);
  if (!customBlock) return [];
  const ed = (customBlock as any).data.ed;
  if (!Array.isArray(ed)) return [];
  return ed;  // Section[] (sections → columns → widgets tree)
}
```

**Что AX обязан:**

- Понять, что 90%+ страниц в production используют именно `custom + data.ed`-формат (M1 ED-editor pipeline, Stage 28).
- Уметь рендерить `Section[]` через Leptos component (`crates/presentation/src/leptos/cms_page_component.rs`).
- Fallback: если `body` содержит другие блоки (`hero`, `text`, etc.) — рендерить их тоже; не падать.

Полный тип `Section`:

```typescript
// SITE1 SandboxEditor.tsx (упрощённый)
type Section = {
  id: string;
  columns: Column[];
  style?: { padding?: string; bg?: string; };
};
type Column = {
  id: string;
  width: 1 | 2 | 3 | 4 | 6 | 12;   // grid units из 12
  widgets: Widget[];
};
type Widget =
  | { type: 'heading'; props: { level: 1 | 2 | 3; text: string; color?: string; font?: string; align?: 'left'|'center'|'right' } }
  | { type: 'text';    props: { html: string; color?: string; font?: string } }
  | { type: 'button';  props: { label: string; href: string; variant?: 'primary'|'secondary'; size?: 'sm'|'md'|'lg' } }
  | { type: 'divider'; props: { thickness?: number; color?: string } }
  | { type: 'spacer';  props: { height: number } }
  | { type: 'iconBox'; props: { icon: string; title: string; description?: string } }
  | { type: 'cta';     props: { title: string; subtitle?: string; cta: { label: string; href: string } } }
  | { type: 'image';   props: { src: string; alt?: string; width?: number; height?: number } };
```

8 типов виджетов. AX повторяет точно — каждый widget маппится в Leptos `view!` шаблон с теми же inline-style правилами.

---

## 3. Read-path end-to-end (как страница доходит до пользователя)

Сейчас (SITE1):

```
Browser GET https://imperiumspa.spa.me/about
       ↓
Caddy → Next.js (port 3011)
       ↓
Next.js route: apps/web/src/app/(tenants)/imperiumspa/[slug]/page.tsx
       ↓
fetchPublicCmsPage('about', 'imperiumspa') →
   fetch http://api:3010/v1/cms/pages/public/by-slug/about?locale=ru
   headers: { 'X-Tenant-Slug': 'imperiumspa' }
       ↓
NestJS: TenantResolverMiddleware → CmsController.publicBySlug → CmsService.getPublishedBySlug
       ↓
SELECT FROM cms_pages WHERE tenant_id=? AND slug=? AND locale=? AND status='published' LIMIT 1
       ↓
Response: CmsPageDTO { id, slug, title, body, ... }
       ↓
Next.js: extractEdSections(page.body) → EdRenderer → SSR HTML
       ↓
Browser получает финальный HTML
```

После cms_pages migration (AX Phase A):

```
Browser GET https://imperiumspa-pilot.spa.me/about
       ↓
Caddy → AX (port 7000) — для whitelisted tenant + path
       ↓
Axum router: GET /{slug}
       ↓
tenant_resolver middleware → req.extensions::<TenantContext>()
       ↓
cms_handlers::get_published_by_slug(ctx, slug, locale) →
  use_cases::cms::get_published_by_slug::execute(repo, ctx, slug, locale) →
  CmsRepository::find_published_by_slug(tx, ctx, slug, locale) →
  with_tenant(pool, ctx, |tx| async {
    SET LOCAL app.current_tenant_id = $1;
    SELECT FROM cms_pages_v_active WHERE slug=$2 AND locale=$3 LIMIT 1;  -- RLS отрежет чужой
  })
       ↓
PublishedPage { id, slug, title, body, ... }
       ↓
CmsRenderer::render_html(page, tenant_design_tokens) → Leptos SSR
       ↓
Response: HTML byte-stream + headers
       ↓
Browser получает финальный HTML
```

Ключевые отличия:
- **AX рендерит HTML сам** (Leptos SSR) — нет round-trip Next.js ↔ API. Это главный perf win.
- **RLS на view'е** — даже если запрос забудет slug-фильтр, чужие tenant'ы не утекут.
- **Один процесс** вместо двух (Next.js + API).

---

## 4. Что **не должно ломаться** при cutover (contract identity)

Конкретно для GET `/{slug}` от внешнего пользователя:

| Аспект | SITE1 | AX (обязан) |
|--------|-------|-------------|
| **HTTP status** для существующей published page | 200 | 200 |
| **HTTP status** для draft/archived/missing/cross-tenant | 404 | 404 |
| **HTTP status** для wrong tenant slug | 404 (no leak) | 404 |
| **Content-Type** | `text/html; charset=utf-8` | `text/html; charset=utf-8` |
| **Cache-Control** | `no-store` (SSR) или `public, s-maxage=...` если задано | identical headers |
| **Vary** | `Accept-Encoding` | identical |
| **HTML structure** | `<html>...<head><link rel="stylesheet" Google Fonts><style>:root{--bg:...}</style></head><body><div class="tenant-site">...</div></body></html>` | byte-for-byte where possible; class names, CSS-vars names — same |
| **`?td=base64`** preview overrides работают | ✓ | ✓ (через extractor в presentation middleware) |
| **Open Graph / meta tags** | `metaTitle`, `metaDescription`, `<meta name="...">` | identical |
| **Time to first byte** (TTFB) | ~120ms p95 (baseline) | ≤ 60ms p95 (success criteria) |

Любое отклонение — это либо bug, либо нужен **deliberate** ADR с обоснованием. Без ADR — не отклоняться.

---

## 5. Cross-module dependencies (что cms_pages читает)

| Зависимость | Откуда читает | AX подход |
|-------------|---------------|-----------|
| `tenants` row | для design tokens + locale + customDomain | read-only view `tenants_v_active` через `infrastructure/persistence/views/` |
| `tenant_design_tokens` | CSS-vars + Google Fonts URL | read-only view `tenant_design_tokens_v` |
| `media` row (через `imageKey` в block) | для S3 URL построения | пока не нужно в pilot — AX рендерит относительный `/media/<key>` URL, Next/Caddy резолвит CDN |
| `tenant_menu_items` (для Navigation в TenantSiteShell) | для верхнего меню | в pilot scope: **только если** AX рендерит home / catch-all `[slug]` — да, нужен. Phase A: ОК через read-only view |

**AX правило:** ВСЕ cross-module reads — через **read-only views** в `infrastructure/persistence/views/`, никогда напрямую через `sqlx::query("SELECT * FROM tenant_design_tokens")` в `cms_pages_repo.rs`. Это compile-time принуждается через module visibility (`pub(crate)` на views), runtime — через `cargo xtask architecture-check` (см. `AX/ENTITY.md §2.6`).

---

## 6. Error handling — что AX обязан повторить

`apps/api/src/cms/cms.service.ts` бросает:

- `NotFoundException({ code: 'PAGE_NOT_FOUND', id|slug, locale? })` → HTTP 404
- `ConflictException({ code: 'PAGE_SLUG_LOCALE_TAKEN', ... })` → HTTP 409 (write path — НЕ в Phase A)
- `BadRequestException` через Zod validation failure (write path)

AX (`crates/common/src/error.rs`):

```rust
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("not found: {0}")]   NotFound(NotFoundDetail),
    #[error("conflict: {0}")]    Conflict(String),
    #[error("validation: {0}")]  Validation(#[from] garde::Report),
    #[error("unauthorized")]     Unauthorized,
    #[error("forbidden: {0}")]   Forbidden(String),
    #[error("tenant mismatch")]  TenantMismatch,            // ← security event, Sentry capture
    #[error("bad request: {0}")] BadRequest(String),
    #[error("internal: {0}")]    Internal(#[from] eyre::Report),
    #[error("database")]         Database(#[from] sqlx::Error),
}

#[derive(Debug, serde::Serialize)]
pub struct NotFoundDetail {
    pub code: &'static str,   // e.g. "PAGE_NOT_FOUND"
    #[serde(flatten)]
    pub fields: serde_json::Value,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound(d) => (StatusCode::NOT_FOUND, Json(d)).into_response(),
            // ... остальное по таблице из AX §7
        }
    }
}
```

**Invariants:**
- `code` поле всегда `SCREAMING_SNAKE_CASE`
- 404 body — `{ "code": "PAGE_NOT_FOUND", "slug": "...", "locale": "..." }` идентично SITE1
- `TenantMismatch` — **отдельный вариант**, Sentry capture с severity high, не общий 403. Это security event.

---

## 7. Чек-лист «что AX обязан сохранить» (бизнес-уровень)

- [ ] 4 (→5) слоя tenant-изоляции по `AX/ENTITY.md §3`
- [ ] Resolver приоритет: header → subdomain → query
- [ ] 404 на cross-tenant запросы (никогда 403, не утечка существования)
- [ ] `slug` immutable; pair `(tenant_id, slug, locale)` UNIQUE
- [ ] Partial index `WHERE status='published'` сохранён в expand migration
- [ ] CmsBlocks discriminated union — 6 типов, AX поддерживает все
- [ ] `custom + data.ed` — основной формат после Stage 28 (90%+ страниц); AX рендерит `Section[] → HTML`
- [ ] 8 widget types в Section/Column/Widget tree
- [ ] HTTP contract identity (status / Content-Type / Cache-Control / HTML structure / OG meta tags / ?td= override)
- [ ] Error code naming — `SCREAMING_SNAKE_CASE`, тот же JSON shape
- [ ] `TenantMismatch` — отдельный security event
