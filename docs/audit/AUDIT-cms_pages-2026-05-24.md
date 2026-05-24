# AUDIT · cms_pages — read-only анализ SITE1

**Дата:** 2026-05-24
**Аудитор:** Claude (AI assistant, session-scoped)
**Scope:** SITE1 cms_pages модуль end-to-end (schema → service → controller → DTO → tenant-context → public render path)
**Назначение:** P1 артефакт `docs/audit/`, gate для RFC-001 (P1 Strategic)
**Mode:** READ-ONLY · никаких изменений в `barbie/ax/` кроме этого файла

---

## 0. TL;DR — что нужно знать прежде, чем писать RFC

1. **RLS отсутствует в SITE1.** `cms_pages` НЕ имеет `ENABLE ROW LEVEL SECURITY`. Изоляция тенантов целиком на уровне приложения (TenantGuard + `withTenant`/`combineTenant` + явные `eq(tenantId, ...)` в каждом WHERE). Это означает: **главный compile-time/runtime win AX — добавление RLS — реализует защиту, которой в SITE1 в принципе нет**, а не дублирует существующую.

2. **Виджеты ED игнорируют tenant design tokens.** `WidgetView.tsx` использует **hardcoded** цвета (`#00FFCC` cyan для primary button, `#F2EBD9` для текста icon-box, `#9A958A` для CTA body, …). CSS-vars `--bg`, `--head-color`, `--acc-color`, `--body-color` применяются только в `TenantSiteShell` (home/sections), но **не в `[slug]/page.tsx` cms-страницах**. Bridge документ утверждал обратное — это устарело.

3. **Per-tenant cms-route хардкодит background.** `imperiumspa/[slug]/page.tsx:28` — `<main style={{ background: '#0E0F12' }}>` — независимо от `tenant_design_tokens.bg`. Аналогично в других 10 per-tenant route'ах.

4. **EdRenderer структура проще, чем заявлено в bridge.** Реальные типы: `Section { id, columns, padding: string }`, `Column { id, span: number, elements }`, `CanvasElement` — flat с optional fields per widget type (не discriminated union). Колонки используют `flex: column.span` напрямую (число), не 12-grid с `width: 1|2|3|4|6|12`.

5. **8 widget types с дефисом:** `heading`, `text`, `button`, `divider`, `spacer`, **`icon-box`** (НЕ `iconBox`), `cta`, `image`. Имена в discriminator имеют значение для byte-for-byte JSON identity.

6. **Slug regex inconsistency:** tenant-resolver принимает 1–40 chars (`^[a-z0-9](?:[a-z0-9-]{0,38}[a-z0-9])?$`), schema CHECK разрешает 3–64 chars (`^[a-z0-9][a-z0-9-]{1,62}[a-z0-9]$`). Тенант с slug 41–64 chars зарегистрировать можно, но resolver его не найдёт. Это **bug**, не контракт.

7. **HTML sanitization не делается.** `TextBlock` в `blocks.schema.ts` имеет комментарий "Phase 0 НЕ санитизирует HTML — assume admin trusted. Phase 1: DOMPurify в API или sandbox-renderer в UI". AX миграция — хороший момент добавить (например, через `ammonia` на read).

8. **`colorFormatCheck` партиальный.** Schema CHECK для tenant_design_tokens проверяет regex только на `bg` и `head_color`. `acc_color`, `body_color` НЕ проверены — теоретически могут быть невалидные hex.

9. **Тесты — только mock-based.** `cms.service.spec.ts` использует `createMockDb()`. **Нет** integration-тестов с реальной Postgres. **Нет** fuzz-тестов изоляции. **Нет** RLS-тестов (RLS не существует). AX добавляет три новых слоя проверки одновременно.

10. **TenantGuard срабатывает даже на `@Public()` endpoint'ах.** `getPublishedBySlug` помечен `@Public()` (skip JwtAuthGuard), но TenantGuard всё равно проверяет `ctx.status === 'active'`. Suspended tenant → 403 даже на публичной странице. AX обязан повторить.

11. **Observability stack в SITE1 практически отсутствует.** Никакого Sentry SDK init в `main.ts`, никакого OpenTelemetry / OTLP exporter, никакого `/metrics` endpoint, никакого request-id middleware, никакого structured JSON logging. Bridge document §10 описывал aspirational план — в реальности есть только helmet + ThrottlerModule (120 req/min) + Nest's pretty-print Logger + HealthController `/health`. AX **инсталлирует** observability с нуля, а не **мигрирует** existing.

12. **S3 URL resolution через `S3Service.publicUrlFor(key)` = simple concat** (`${publicUrl}/${encodeURI(key)}`). MinIO в dev anonymous; prod подразумевает либо public bucket, либо CDN proxy. `signedDownloadUrl(key, ttl)` существует для private bucket'ов, но **не используется** в `MediaService.toResponse`. AX в Phase A — `publicUrlFor` style; presigning откладывается до Phase B если понадобится privacy.

13. **`AppModule` устроен:** global `JwtAuthGuard` через `APP_GUARD` provider (каждый endpoint защищён по дефолту, `@Public()` — opt-out) + `TenantResolverMiddleware.forRoutes('*')` (резолвит tenant на ВСЕХ путях, включая `/health` и auth). AX должен повторить эту инверсию: auth required by default + tenant middleware на всё.

---

## 1. Schema

### 1.1 Применённая SQL (источник — `0000_deep_gamma_corps.sql:244-259`)

```sql
CREATE TABLE IF NOT EXISTS "cms_pages" (
    "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
    "tenant_id" uuid NOT NULL,
    "slug" varchar(255) NOT NULL,
    "locale" varchar(8) DEFAULT 'ru' NOT NULL,
    "title" varchar(500) NOT NULL,
    "body" jsonb DEFAULT '[]'::jsonb NOT NULL,
    "status" varchar(20) DEFAULT 'draft' NOT NULL,
    "meta_title" varchar(255),
    "meta_description" text,
    "cover_image_key" varchar(500),
    "author_user_id" uuid,
    "published_at" timestamp,
    "created_at" timestamp DEFAULT now() NOT NULL,
    "updated_at" timestamp DEFAULT now() NOT NULL
);
```

### 1.2 Foreign keys

```sql
ALTER TABLE "cms_pages" ADD CONSTRAINT "cms_pages_tenant_id_tenants_id_fk"
  FOREIGN KEY ("tenant_id") REFERENCES "public"."tenants"("id") ON DELETE cascade;

ALTER TABLE "cms_pages" ADD CONSTRAINT "cms_pages_author_user_id_users_id_fk"
  FOREIGN KEY ("author_user_id") REFERENCES "public"."users"("id") ON DELETE set null;
```

### 1.3 Indexes (`0000_deep_gamma_corps.sql:526-528`)

```sql
CREATE UNIQUE INDEX IF NOT EXISTS "cms_pages_tenant_slug_locale_uniq"
  ON "cms_pages" USING btree ("tenant_id","slug","locale");

CREATE INDEX IF NOT EXISTS "cms_pages_tenant_status_idx"
  ON "cms_pages" USING btree ("tenant_id","status");

CREATE INDEX IF NOT EXISTS "cms_pages_tenant_published_idx"
  ON "cms_pages" USING btree ("tenant_id","published_at" DESC NULLS LAST)
  WHERE status = 'published';
```

**Partial index `cms_pages_tenant_published_idx` — load-bearing для public listing.** AX expand-migration ОБЯЗАНА сохранить семантику этого индекса (даже если view вводится, индекс остаётся на базовой таблице).

### 1.4 Что НЕ enforce'ется в DB (только в TS-коде)

| Invariant | Что enforce'ит | Где |
|-----------|----------------|-----|
| `status IN ('draft','published','archived')` | TypeScript `$type<CmsPageStatus>` + Zod validation | service write methods |
| `locale IN ('ru','en')` | DTO `@IsIn(['ru','en'])` + service default | controller + service |
| `body` shape (CmsBlocks discriminated union) | Zod `PageBody.parse()` в `validateBody()` | service write methods |
| `publishedAt IS NOT NULL ⇔ status='published'` | service maintains в `publishPage`/`unpublishPage` | НЕ проверяется DB constraint'ом |
| `body` max 200 блоков | Zod `z.array(Block).max(200)` | service `validateBody()` |
| `text.html` max 100k chars | Zod `z.string().max(100_000)` | service `validateBody()` |

**Implication for AX:** RLS добавляется в expand-migration, но **schema check constraints (status/locale enum, body size) — это потенциально следующая expand-migration** или фикс на write path. В Phase A (read-only) — не обязательно, в Phase B (write) — обязательно.

### 1.5 Связанные таблицы

#### `tenants` (`packages/db/src/schema/tenants.ts`)

| Column | Type | Notes |
|--------|------|-------|
| `id` | uuid PK defaultRandom | |
| `slug` | varchar(64) UNIQUE | CHECK `^[a-z0-9][a-z0-9-]{1,62}[a-z0-9]$` (3–64 chars) |
| `name` | varchar(255) NOT NULL | |
| `legal_name` | varchar(500) | |
| `status` | varchar(20) | enum `'active'|'pending'|'suspended'|'archived'`, default `'pending'` |
| `plan_id` | uuid | nullable, no FK в Phase 0 (subscription_plans отсутствует) |
| `primary_domain` | varchar(255) | UNIQUE partial (where not null) |
| `contact_email` | varchar(320) NOT NULL | |
| `contact_phone` | varchar(32) | |
| `bootstrap_source_url` | text | URL от site-analyzer |
| `custom_domain` | varchar(255) | UNIQUE partial (where not null), для brand-domains |
| `timezone` | varchar(64) NOT NULL default `'Europe/Moscow'` | |
| `locale` | varchar(8) NOT NULL default `'ru'` | |
| `settings` | jsonb NOT NULL default `'{}'` | features/bookingPolicy/paymentRequired |
| `created_at` / `updated_at` | timestamp | |

**Indexes:** `tenants_slug_uniq`, `tenants_primary_domain_uniq` (partial), `tenants_custom_domain_uniq` (partial), `tenants_custom_domain_idx`, `tenants_status_idx`.

**Implication:** AX `tenant_resolver` должен поддерживать **И** `slug` ↔ subdomain, **И** `custom_domain` ↔ Host header. Текущий SITE1 resolver делает только slug-routing — custom_domain resolution **отсутствует в коде**, несмотря на колонку. Это TODO для AX или SITE1.

#### `tenant_design_tokens` (`packages/db/src/schema/tenant-design-tokens.ts`)

| Column | Type | Notes |
|--------|------|-------|
| `tenant_id` | uuid PK + FK to tenants ON DELETE cascade | 1:1 |
| `bg` | varchar(16) NOT NULL default `'#FFFFFF'` | CHECK hex regex |
| `head_color` | varchar(16) NOT NULL default `'#0A0A0A'` | CHECK hex regex |
| `head_font` | varchar(64) NOT NULL default `'Unbounded'` | |
| `acc_color` | varchar(16) NOT NULL default `'#D4AF37'` | **НЕТ CHECK** |
| `acc_font` | varchar(64) NOT NULL default `'Unbounded'` | |
| `body_color` | varchar(16) NOT NULL default `'#1A1A1A'` | **НЕТ CHECK** |
| `body_font` | varchar(64) NOT NULL default `'Inter'` | |
| `logo_key`, `logo_alt`, `favicon_key` | varchar(500/255/500) nullable | S3 keys |
| `nav_template` | varchar(32) NOT NULL default `'top-classic'` | CHECK `IN ('top-classic','mega-images','vertical-side')` |
| `custom_css` | text nullable | Phase B feature |
| `extras` | jsonb default `'{}'` | **БЕЗ NOT NULL** — potential NULL state |
| `updated_at` | timestamp NOT NULL | |

**Findings:**
- **Asymmetric color check:** `bg` и `head_color` имеют regex, `acc_color` и `body_color` — нет. Theoretically invalid hex может попасть в БД. AX expand-migration может выровнять (low-risk fix).
- **`extras` jsonb default `'{}'` но не NOT NULL:** возможно состояние NULL vs `{}`. AX должен toleratе оба (read как `Option<JsonValue>`).

### 1.6 Migrations history

Только `0000_deep_gamma_corps.sql` касается `cms_pages` (initial create + FK + 3 indexes). Subsequent migrations (`0001_greedy_molten_man.sql`, `0002_chat.sql`, `0003_tenant_bootstrap.sql`) **не модифицируют** cms_pages.

**Implication:** schema стабильна с Phase 0. AX expand-migration `0001_cms_pages_expand.sql` — первое изменение поверх неё. Reversal cost низкий (DROP VIEW + DISABLE RLS).

---

## 2. Service layer

**File:** `apps/api/src/cms/cms.service.ts` (256 lines)
**Class:** `CmsService`
**Dependencies (DI):** `TenantContextService`, `DRIZZLE` (Database token)

### 2.1 Method signatures (9 public + 2 private)

| # | Method | Returns | Auth + Tenant | Used by |
|---|--------|---------|---------------|---------|
| 1 | `createPage(dto, authorUserId)` | `Promise<PageResponseDto>` | tenant-admin role | POST /v1/cms/pages |
| 2 | `listPages(query)` | `Promise<ListPagesResponseDto>` | tenant-admin, salon-manager | GET /v1/cms/pages |
| 3 | `getPage(id)` | `Promise<PageResponseDto>` | tenant-admin, salon-manager | GET /v1/cms/pages/:id |
| 4 | `getPublishedBySlug(slug, locale='ru')` | `Promise<PageResponseDto>` | **@Public()** | GET /v1/cms/pages/public/by-slug/:slug |
| 5 | `updatePage(id, dto)` | `Promise<PageResponseDto>` | tenant-admin | PATCH /v1/cms/pages/:id |
| 6 | `publishPage(id)` | `Promise<PageResponseDto>` | tenant-admin | POST /v1/cms/pages/:id/publish |
| 7 | `unpublishPage(id)` | `Promise<PageResponseDto>` | tenant-admin | POST /v1/cms/pages/:id/unpublish |
| 8 | `archivePage(id)` | `Promise<PageResponseDto>` | tenant-admin | DELETE /v1/cms/pages/:id |
| 9 | (private) `validateBody(rawBody)` | `CmsBlocks` | — | internal Zod parse + 400 on error |
| 10 | (private) `toResponse(row)` | `PageResponseDto` | — | row → dto mapping |

**AX Phase A scope:** только метод **#4** `getPublishedBySlug` + поддерживающий read-only repo. Методы #1–#3, #5–#8 — write/admin, остаются в SITE1.

### 2.2 `getPublishedBySlug` — точный код (load-bearing для AX)

```typescript
async getPublishedBySlug(slug: string, locale: 'ru' | 'en' = 'ru'): Promise<PageResponseDto> {
  const tenantId = this.tenantContext.requireTenantId();
  const [row] = await this.db
    .select()
    .from(cmsPages)
    .where(
      and(
        eq(cmsPages.tenantId, tenantId),
        eq(cmsPages.slug, slug),
        eq(cmsPages.locale, locale),
        eq(cmsPages.status, 'published'),
      ),
    )
    .limit(1);
  if (!row) {
    throw new NotFoundException({ code: 'PAGE_NOT_FOUND', slug, locale });
  }
  return this.toResponse(row);
}
```

**Inavriants для AX:**

1. **`SELECT *` (Drizzle `.select()` без projection) — возвращает все колонки.** AX query НЕ должен использовать `SELECT *` (запрет в `ENTITY.md §11.6`); явный column list обязателен.
2. **4 условия в WHERE:** `tenant_id`, `slug`, `locale`, `status='published'`. Используется uniqueIndex `cms_pages_tenant_slug_locale_uniq` (по `tenant_id + slug + locale`), partial index `cms_pages_tenant_published_idx` НЕ участвует в этом запросе (он для listing по publishedAt DESC).
3. **`LIMIT 1`** — single row lookup, не listing.
4. **404 body:** `{ code: 'PAGE_NOT_FOUND', slug, locale }` — точный JSON shape для contract identity.
5. **404 для cross-tenant, draft, archived, не существует — одинаковый.** Не различается: пользователь не должен узнать, что страница существует в другом тенанте.
6. **`locale` default `'ru'`** если не задан — AX обязан повторить (controller передаёт `locale ?? 'ru'`).
7. **`requireTenantId()` бросает Error при отсутствии context** — но controller за TenantGuard'ом, значит ctx уже валиден; в AX это compile-time через `&TenantContext` параметр.

### 2.3 `toResponse` — row → DTO mapping (load-bearing для contract identity)

```typescript
private toResponse(row: typeof cmsPages.$inferSelect): PageResponseDto {
  return {
    id: row.id,
    slug: row.slug,
    locale: row.locale as PageResponseDto['locale'],
    title: row.title,
    body: (row.body ?? []) as unknown[],
    status: row.status as PageResponseDto['status'],
    metaTitle: row.metaTitle,
    metaDescription: row.metaDescription,
    coverImageKey: row.coverImageKey,
    authorUserId: row.authorUserId,
    publishedAt: row.publishedAt
      ? row.publishedAt instanceof Date
        ? row.publishedAt.toISOString()
        : String(row.publishedAt)
      : null,
    createdAt: row.createdAt instanceof Date ? row.createdAt.toISOString() : String(row.createdAt),
    updatedAt: row.updatedAt instanceof Date ? row.updatedAt.toISOString() : String(row.updatedAt),
  };
}
```

**Findings для AX:**
- **`tenantId` НЕ возвращается в DTO!** Bridge документ ошибочно указал `tenantId` в response shape. SITE1 PageResponseDto **не имеет** `tenant_id`. AX обязан повторить — НЕ включать tenant_id в JSON. (Это правильно — не раскрывает который тенант страница принадлежит, хотя клиент знает по тому, какой subdomain он запросил.)
- **`body: (row.body ?? []) as unknown[]`** — fallback на `[]` если NULL (но в schema NOT NULL default, поэтому NULL не должен встретиться). AX может skip этот fallback.
- **`publishedAt` сериализация:** ISO8601 если Date, иначе `String()` (для случаев когда драйвер вернул timestamp как string). Обычно `postgres-js` возвращает Date object. AX SQLx возвращает `chrono::DateTime<Utc>` — нужен корректный ISO8601 RFC3339 формат.
- **Все timestamp поля — ISO8601 строки.** UTC offset формат: SITE1 возвращает `2026-05-24T10:30:00.000Z` (Z-suffix). AX должен совпадать. SQLx `chrono::DateTime<Utc>` сериализуется через serde — нужен явный `with = "ts_seconds_rfc3339"` или custom serializer чтобы получить именно Z-suffix.

---

## 3. Controller layer

**File:** `apps/api/src/cms/cms.controller.ts` (116 lines)
**Class:** `CmsController`
**Path:** `/v1/cms/pages`
**Guards (class-level):** `TenantGuard`, `RolesGuard`

### 3.1 Endpoint surface

| Method | Path | Auth | Role | Used by AX? |
|--------|------|------|------|-------------|
| POST | `/v1/cms/pages` | JWT | `tenant-admin` | NO (write — SITE1 only) |
| GET | `/v1/cms/pages` | JWT | `tenant-admin`, `salon-manager` | NO (admin listing) |
| GET | `/v1/cms/pages/:id` | JWT | `tenant-admin`, `salon-manager` | NO (admin detail) |
| PATCH | `/v1/cms/pages/:id` | JWT | `tenant-admin` | NO |
| POST | `/v1/cms/pages/:id/publish` | JWT | `tenant-admin` | NO |
| POST | `/v1/cms/pages/:id/unpublish` | JWT | `tenant-admin` | NO |
| DELETE | `/v1/cms/pages/:id` | JWT | `tenant-admin` | NO |
| **GET** | **`/v1/cms/pages/public/by-slug/:slug`** | **@Public()** | **none** | **YES — Phase A pilot** |

### 3.2 Public endpoint — точный код

```typescript
@Public()
@Get('public/by-slug/:slug')
@ApiOperation({
  summary: 'Публичный рендер страницы по slug+locale (без auth, только published)',
})
publicBySlug(
  @Param('slug') slug: string,
  @Query('locale') locale?: 'ru' | 'en',
): Promise<PageResponseDto> {
  return this.service.getPublishedBySlug(slug, locale ?? 'ru');
}
```

**Notes:**
- `@Public()` — auth/decorators/public.decorator (skip JwtAuthGuard)
- `@Param('slug')` — БЕЗ `ParseUUIDPipe`, потому что slug это string не UUID; БЕЗ regex validation на controller-уровне (validation в schema через uniqueIndex collision и через CreatePageDto regex на write — read принимает любую строку)
- `@Query('locale')` — БЕЗ `IsIn` validation на controller; service сам делает `locale ?? 'ru'`. Это значит **invalid locale (например `'fr'`) проходит до DB и возвращает 404** (нет такого row), не 400. AX обязан повторить — НЕ валидировать locale жёстко на presentation layer.
- `Promise<PageResponseDto>` — single row, не paginated.

### 3.3 Resolved full path

При `tenant-resolver` через subdomain `imperiumspa.spa.me`:
```
GET https://imperiumspa.spa.me/api/v1/cms/pages/public/by-slug/about?locale=ru
```

Через Caddy `→ http://api:3010/v1/cms/pages/public/by-slug/about` + header `X-Tenant-Slug: imperiumspa` (если subdomain routing настроен на Caddy, заголовок проставляется автоматически).

**В реальности (текущий код SSR Next.js):**

```typescript
// apps/web/src/lib/cms-public.ts:28
const res = await fetch(
  `${API_BASE}/v1/cms/pages/public/by-slug/${encodeURIComponent(slug)}?locale=${locale}`,
  { cache: 'no-store', headers: { 'X-Tenant-Slug': tenantSlug } },
);
```

— Next.js делает internal call к `http://localhost:3010` с заголовком `X-Tenant-Slug`. **Subdomain routing не используется в текущей prod-конфигурации**, потому что Next.js SSR работает per-tenant route (см. §6 ниже) и явно передаёт tenant.

**Implication для AX:**
- AX endpoint = same path: `/api/v1/cms/pages/public/by-slug/:slug?locale=ru`
- AX принимает **header `X-Tenant-Slug`** (приоритет per `tenant-resolver.middleware.ts`)
- **Subdomain** также поддерживается (для прямого external traffic после Caddy)
- **Query `?tenant=<slug>`** также поддерживается (для SSE — нужно? для cms_pages нет — SSE не используется тут)

---

## 4. DTOs and validation

### 4.1 `CreatePageDto` (`apps/api/src/cms/dto/create-page.dto.ts`)

```typescript
slug: string                  // @Matches(/^[a-z0-9](?:[a-z0-9/-]{1,78}[a-z0-9])?$/), 3-80 chars
locale?: 'ru' | 'en'         // default 'ru' в service
title: string                 // 1-500 chars
body: Record<string,unknown>[] // далее → Zod validation в service
metaTitle?: string            // <=255
metaDescription?: string      // <=2000
coverImageKey?: string        // <=500
```

**Slug regex для cms_pages:**
- `^[a-z0-9](?:[a-z0-9/-]{1,78}[a-z0-9])?$`
- **Allows slash `/` для nested paths!** Например `services/spa`, `about/contacts`.
- 3-80 chars total (start + 1-78 middle + end).
- НЕ то же самое, что tenant slug regex (40-char max без слэша).

**Implication для AX page slug regex:** при создании AX SQL для read path, slug parameter может содержать слэш (`encodeURIComponent` энкодит его как `%2F`, но Axum extractor `Path<String>` декодирует). Routing definition `/by-slug/:slug` должна обрабатывать slashes — может потребоваться `/by-slug/*slug` (greedy) или специальный matcher.

### 4.2 `PageResponseDto` (output shape)

```typescript
class PageResponseDto {
  id: string
  slug: string
  locale: 'ru' | 'en'
  title: string
  body: unknown[]              // raw blocks array
  status: 'draft' | 'published' | 'archived'
  metaTitle?: string | null
  metaDescription?: string | null
  coverImageKey?: string | null
  authorUserId?: string | null
  publishedAt?: string | null  // ISO8601 or null
  createdAt: string            // ISO8601
  updatedAt: string            // ISO8601
}
```

**Critical для AX byte-for-byte contract:**
- `metaTitle`, `metaDescription`, `coverImageKey`, `authorUserId`, `publishedAt` — **могут быть `null`** в JSON (не undefined). AX serde должен сериализовать `Option::None` как `null`, не пропускать поле (`skip_serializing_if = "Option::is_none"` ЗАПРЕЩЕНО на этих полях).
- Поля **в этом порядке** в JSON output. JavaScript JSON.stringify сохраняет field declaration order; Rust `serde_json` тоже (#[derive(Serialize)] с явным struct field order).

**Поле НЕ в response:** `tenantId` (см. §2.3 выше).

### 4.3 `PageBody` Zod schema (`apps/api/src/cms/dto/blocks.schema.ts`)

```typescript
PageBody = z.array(Block).max(200)

Block = z.discriminatedUnion('type', [
  HeroBlock, TextBlock, GalleryBlock, ServicesBlock, CtaBlock, CustomBlock
])

HeroBlock = z.object({ type: z.literal('hero'), data: {
  title: z.string().min(1).max(500),
  subtitle: z.string().max(2000).optional(),
  imageKey: z.string().max(500).optional(),
} })

TextBlock = z.object({ type: z.literal('text'), data: {
  html: z.string().max(100_000),    // НЕ sanitize'нуто
} })

GalleryBlock = z.object({ type: z.literal('gallery'), data: {
  mediaIds: z.array(uuid).min(1).max(50),
} })

ServicesBlock = z.object({ type: z.literal('services'), data: {
  categoryFilter: z.string().max(64).optional(),
  limit: z.number().int().min(1).max(100).optional(),
} })

CtaBlock = z.object({ type: z.literal('cta'), data: {
  label: z.string().min(1).max(200),
  href: z.string().max(2000).refine(v => v.startsWith('/') || /^https?:\/\//.test(v)),
  style: z.enum(['primary', 'secondary']).optional(),
} })

CustomBlock = z.object({ type: z.literal('custom'), data: z.record(z.unknown()) })
```

**Findings:**
- **6 типов блоков top-level** (это `CmsBlocks` discriminated union на schema/cms-pages.ts уровне)
- **`custom` имеет произвольный `data: Record<string, unknown>`** — это escape hatch, через который ED-editor сохраняет своё дерево `{ ed: Section[] }`. Validation на ED-уровне — отдельная (в SandboxEditor TS); сервис не валидирует структуру `data.ed`.
- **CTA `href`** валидируется — internal path (`/...`) или absolute URL (`https?://`). AX `garde` custom validator повторяет.
- **`text.html` max 100k chars** — но **БЕЗ sanitization**. Trusted admin assumption. AX может добавить (на read через `ammonia`) для defense-in-depth.

**ED data shape (внутри `custom + data.ed`):**

```typescript
// apps/web/src/components/cms/ed-editor/ed-types.ts

Section { id, columns, padding: string }
Column { id, span: number, elements: CanvasElement[] }
CanvasElement {
  id, type: 'heading'|'text'|'button'|'divider'|'spacer'|'icon-box'|'cta'|'image',
  heading?, text?, button?, divider?, spacer?, iconBox?, cta?, image?,  // ⚠ icon-box → iconBox в TS field name
  elStyle?: ElStyle,
}

ElStyle {
  paddingTop, paddingRight, paddingBottom, paddingLeft: number,
  background: string,
  borderRadius: number,
  opacity: number (0-100),
  customCss: string,  // ⚠ M1: HIDE/IGNORE в render
}
```

**Critical для AX:**
- Type discriminator value `'icon-box'` (with hyphen) в JSON; TS field name `iconBox` (camelCase). AX в Rust: `#[serde(rename = "icon-box")] IconBox { ... }`.
- 8 widget types, не 6 как bridge говорил. И структура flat (optional fields), не discriminated union — реальный TS использует `CanvasElement` с optional fields per type. AX может уйти от этого паттерна (использовать `enum Widget { Heading {...}, Text {...}, ... }`) для лучшей типизации, но **сериализация JSON должна оставаться identical** — flat fields per type:

```json
{ "id": "...", "type": "heading", "heading": { "text": "...", "tag": "h1", "align": "center", "color": "#fff", "fontSize": 48 }, "elStyle": {...} }
```

— это та форма, в которой данные лежат в `cms_pages.body[i].data.ed[j].columns[k].elements[l]`. Менять её = breaking change для сохранённого contenta.

---

## 5. Tenant context — 4-layer defense (как реально устроено)

### 5.1 Layer 1: TenantResolverMiddleware

**File:** `apps/api/src/tenant-context/tenant-resolver.middleware.ts` (111 lines)

**Resolution priority:**

```typescript
extractSlug(req): string | null {
  // 1. Header `X-Tenant-Slug` (приоритет)
  // 2. Subdomain `{slug}.{rootDomain}` (single-level only)
  // 3. Query `?tenant=<slug>` (для SSE)
  // Если нигде → null, middleware пропускает дальше БЕЗ контекста
}

normalizeSlug(raw): string | null {
  // Lowercase + trim
  // Regex: ^[a-z0-9](?:[a-z0-9-]{0,38}[a-z0-9])?$  (1-40 chars)
  // Иначе null
}
```

**После резолва:**

```typescript
const ctx: TenantContext = {
  tenantId: tenant.id,
  tenantSlug: tenant.slug,
  status: tenant.status as TenantContext['status'],
};

req.__tenantContext = ctx;       // 1) для @CurrentTenant decorator
this.ctxService.run(ctx, () => next());  // 2) ALS для service layer
```

**Двойная экспозиция:**
- `req.__tenantContext` — для синхронных декораторов (createParamDecorator не может ходить в ALS)
- ALS через `TenantContextService.run()` — для сервисов в любой глубине стека

**DB call в middleware:**

```typescript
await this.db
  .select({ id: tenants.id, slug: tenants.slug, status: tenants.status })
  .from(tenants)
  .where(and(eq(tenants.slug, slug), isNotNull(tenants.id)))
  .limit(1);
```

— один SELECT по `tenants.slug` (через `tenants_slug_uniq` index). **Нет кэширования.** Каждый запрос → новый DB-hit.

**Implication для AX:** добавить moka LRU cache `slug → TenantContext` с TTL 5 минут (`ENTITY.md §3`) — этого в SITE1 нет, можно сделать сразу.

### 5.2 Layer 2: TenantGuard

**File:** `apps/api/src/tenant-context/tenant.guard.ts` (92 lines)

```typescript
canActivate(context): boolean {
  // 1. Если @SkipTenant() — true
  // 2. req.__tenantContext должен существовать иначе 401 TENANT_NOT_RESOLVED
  // 3. ctx.status === 'active' иначе 403 TENANT_NOT_ACTIVE
  // 4. Если req.user задан (JwtAuthGuard сработал):
  //    a. user.kind === 'platform' → return true (platform-admin bypass)
  //    b. user.kind === 'tenant' && user.tenantId !== ctx.tenantId → 403 TENANT_OWNERSHIP_MISMATCH
  // 5. Если req.user НЕ задан → return true (например @Public() route)
  return true;
}
```

**Implication для AX:**
- **3 разных error code:** AX должен повторить:
  - `TENANT_NOT_RESOLVED` → 401 (нет subdomain/header)
  - `TENANT_NOT_ACTIVE` → 403 (suspended/archived/pending)
  - `TENANT_OWNERSHIP_MISMATCH` → 403 **security event** (Sentry capture, отдельный alert)
- **Platform-admin bypass** — `user.kind === 'platform'` пропускается через guard. AX в Phase A не имплементирует platform admin (admin live in SITE1), но `tenant_resolver` middleware всё равно отрабатывает (status check) — нужно учесть, что platform-admin может ходить в любой тенант (legitimate use case).
- **`@Public()` + TenantGuard:** даже без auth, ctx нужен. AX cms_pages public endpoint требует resolved tenant (через X-Tenant-Slug header).

### 5.3 Layer 3: `withTenant` / `combineTenant` helper

**File:** `apps/api/src/tenant-context/with-tenant.helper.ts` (56 lines)

```typescript
export function withTenant<T>(query: T, tenantId: string, tenantIdColumn): T {
  return query.where(eq(tenantIdColumn, tenantId));
}

export function combineTenant(tenantId, tenantIdColumn, ...extras): SQL {
  return and(eq(tenantIdColumn, tenantId), ...extras);
}
```

**Application-level enforcement.** Если разработчик забыл вызвать helper — НЕТ ничего, что заметит. `cms.service.ts` использует `combineTenant` в `listPages` и **явный** `eq(cmsPages.tenantId, tenantId)` в `getPublishedBySlug`, `getPage`, write-методах. Code review — единственный enforcement.

**AX замена:** `with_tenant(pool, ctx, |tx| async { ... })` + RLS POLICY. Это **hard wall** вместо application-policy. Если AX query забудет `tenant_id` — RLS отрежет, не пройдёт sneak.

### 5.4 Layer 4 (заявлен в `tenant.guard.ts:23`)

Комментарий в guard говорит "Слой 4 — audit log (Phase 1)". **В Phase 0 не реализовано.** AX в Phase A тоже не имплементирует — это cross-cutting feature, выйдет с appointments/booking миграцией позже.

### 5.5 НЕТ Layer: Row Level Security

**В schema cms_pages нет:**
- `ENABLE ROW LEVEL SECURITY`
- никаких POLICY на `cms_pages`
- никаких CHECK constraint на `tenant_id` != random uuid

**Implication для AX:** RLS добавляется в **expand-migration 0001 поверх SITE1 schema**. Это требует осторожности — see §7.4 ниже.

---

## 6. Public render path — web/SSR side

### 6.1 Routing structure (`apps/web/src/app/(tenants)/...`)

```
apps/web/src/app/(tenants)/
├── 5massage/[slug]/page.tsx          # tenant 1
├── barbiespa/[slug]/page.tsx         # tenant 2
├── dachaspa/[slug]/page.tsx          # ...
├── eroticmassaj/[slug]/page.tsx
├── etalonspa/[slug]/page.tsx
├── imperiumspa/
│   ├── page.tsx                       # home (TenantSiteShell fallback)
│   └── [slug]/page.tsx                # cms-page render
├── nebesaspa/[slug]/page.tsx
├── pentagon/[slug]/page.tsx
├── roxy-spa/[slug]/page.tsx
└── soho-spa/[slug]/page.tsx
```

**11 per-tenant folder'ов с почти identical page.tsx files.** Не DRY. Bridge документ не отразил этот pattern.

### 6.2 Пример per-tenant route — `imperiumspa/[slug]/page.tsx`

```typescript
const TENANT_SLUG = 'imperiumspa';

export default async function ImperiumspaSlugPage({ params }) {
  const { slug } = await params;
  const page = await fetchPublicCmsPage(slug, TENANT_SLUG).catch(() => null);
  if (!page) notFound();

  const sections = extractEdSections(page.body);
  return (
    <main style={{ background: '#0E0F12', minHeight: '100vh' }}>
      <EdRenderer sections={sections} />
      <TenantEditFab tenantSlug={TENANT_SLUG} />
    </main>
  );
}
```

**Critical finding:**
- **`background: '#0E0F12'` ХАРДКОД** для imperiumspa. Не из `tenant_design_tokens.bg` (который default `'#FFFFFF'`).
- **НЕТ `TenantSiteShell` wrapper** — нет CSS-vars, нет Google Fonts.
- **НЕТ `<head>` meta tags** (Open Graph, description) — Next.js может их рендерить через `generateMetadata` или layout.tsx, но в этом route нет.
- **`TenantEditFab`** рендерится — это admin-only FAB; в production только admin'у видна; AX в read-path НЕ должен рендерить (admin UI остаётся в SITE1).

**Implication для AX contract identity:**
- AX **не повторяет** этот SITE1 baseline хаос. AX рендерит `<main>` с background из `tenant_design_tokens.bg` (исправление bug'а во время миграции — нужно зафиксировать в RFC как deliberate deviation).
- **Bridge документ `04_DESIGN.md` обещал `TenantSiteShell` структуру** — в реальности per-tenant routes её не используют. AX рендерит полноценную shell (CSS-vars + Google Fonts + `<style>` block) — это **upgrade**, не identity.

### 6.3 `fetchPublicCmsPage` — SSR fetcher

```typescript
// apps/web/src/lib/cms-public.ts
export async function fetchPublicCmsPage(slug, tenantSlug, locale = 'ru') {
  const res = await fetch(
    `${API_BASE}/v1/cms/pages/public/by-slug/${encodeURIComponent(slug)}?locale=${locale}`,
    { cache: 'no-store', headers: { 'X-Tenant-Slug': tenantSlug } },
  );
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`cms-public: API ${res.status} ...`);
  return await res.json() as CmsPageDTO;
}
```

— `cache: 'no-store'` (всегда свежие данные), `'X-Tenant-Slug'` header (не subdomain) для internal SSR fetch.

### 6.4 `TenantSiteShell.tsx` — home-route only

**File:** `apps/web/src/components/tenant-site/TenantSiteShell.tsx` (111 lines)

Используется в home-route (`imperiumspa/page.tsx`, не slug-route). Рендерит:

1. **`<link rel="preconnect">`** для fonts.googleapis.com + fonts.gstatic.com
2. **`<link rel="stylesheet" href={buildGoogleFontsUrl(...)}>`** с deduped families, weights `300;400;500;600;700`, `display=swap`
3. **CSS-vars** через inline `style` attribute:
   ```
   --bg, --head-color, --head-font, --acc-color, --acc-font, --body-color, --body-font
   ```
4. **`<style>{...}</style>` block** с 7 правилами для `.tenant-site` (см. §6.5)
5. **`<div className={layoutClass}>`** — `layoutClass = 'tenant-site min-h-screen md:pl-64'` для `vertical-side`, иначе `'tenant-site min-h-screen'`
6. **Hardcoded home-page sections:** `<Navigation>`, `<Hero>`, `<Positioning>`, `<Programs>`, `<Rooms>`, `<Staff>`, `<Contacts>`, `<Footer>` — это НЕ из CMS, это hardcoded React components в `sections/`. AX cms_pages migration **не покрывает** эти sections.

### 6.5 Inline CSS block (TenantSiteShell)

```css
.tenant-site {
  background: var(--bg);
  color: var(--body-color);
  font-family: var(--body-font);
  line-height: 1.6;
}
.tenant-site h1, .tenant-site h2, .tenant-site h3 {
  font-family: var(--head-font);
  color: var(--head-color);
  letter-spacing: -0.01em;
  line-height: 1.1;
}
.tenant-site .accent {
  font-family: var(--acc-font);
  color: var(--acc-color);
}
.tenant-site a {
  color: var(--acc-color);
  text-decoration: none;
  transition: opacity 0.2s;
}
.tenant-site a:hover { opacity: 0.7; }
.tenant-site .container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 1.5rem;
}
```

**AX Phase B Leptos SSR должен** повторить это **точно** (одинаковые CSS class names, var names, fallbacks).

### 6.6 `EdRenderer.tsx`

**File:** `apps/web/src/components/cms/ed-editor/EdRenderer.tsx` (68 lines)

```typescript
export function EdRenderer({ sections }: { sections: Section[] }): ReactElement {
  return sections.map((section) => (
    <section style={{ padding: section.padding }}>
      <div style={{ display: 'flex', gap: 16 }}>
        {section.columns.map((column) => (
          <div style={{ flex: column.span, minWidth: 0 }}>
            {column.elements.map((el) => {
              const s = el.elStyle ?? defaultElStyle();
              return (
                <div style={{
                  padding: `${s.paddingTop}px ${s.paddingRight}px ${s.paddingBottom}px ${s.paddingLeft}px`,
                  background: s.background || 'transparent',
                  borderRadius: s.borderRadius,
                  opacity: s.opacity / 100,
                }}>
                  <WidgetView el={el} mode="render" />
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </section>
  ));
}
```

**HTML output для одной section:**

```html
<section style="padding: <section.padding>">
  <div style="display: flex; gap: 16px">
    <div style="flex: <column.span>; min-width: 0">
      <div style="padding: <pt>px <pr>px <pb>px <pl>px; background: <bg>; border-radius: <br>; opacity: <op>/100">
        <!-- widget HTML -->
      </div>
      ...
    </div>
    ...
  </div>
</section>
```

**AX Leptos повторяет 1-в-1:**
- Inline `style` attribute с CSS string (НЕ external classes)
- `display: flex; gap: 16` (NOT 1rem) — exact pixel value
- `flex: column.span` — numeric value (e.g., `flex: 1`, `flex: 2`)
- React `style={{ borderRadius: N }}` → "borderRadius: Npx" в HTML; Leptos должен делать тоже
- `opacity: s.opacity / 100` — React сериализует число 0.01-1.0; Leptos должен тоже

### 6.7 `WidgetView.tsx` — 8 widget HTML outputs

**File:** `apps/web/src/components/cms/ed-editor/WidgetView.tsx` (125 lines)

| Type | HTML output (mode='render') | Comments |
|------|-----------------------------|----------|
| `heading` | `<h1|h2|h3|h4 style="textAlign: ; color: ; fontSize: ; margin: 0; fontWeight: 700; lineHeight: 1.2">{text}</h1>` | tag from props.tag |
| `text` | `<p style="textAlign: ; color: ; margin: 0; lineHeight: 1.7; fontSize: 15">{content}</p>` | content as plain text (НЕ html!) |
| `button` | `<div style="textAlign: align"><button style="...">{label}</button></div>` | hardcoded color schemes |
| `divider` | `<hr style="borderStyle: ; borderColor: ; borderWidth: Npx 0 0 0; margin: 0">` | |
| `spacer` | `<div style="height: Npx" />` | в render-режиме |
| `icon-box` | `<div style="display: flex; flexDirection: column|row; gap: 12">...lucide icon + title + description</div>` | hardcoded colors `#F2EBD9`, `#C9C2B0` |
| `cta` | `<div style="textAlign: ; padding: 16px 0">{h3, p, button}</div>` | hardcoded colors `#F2EBD9`, `#9A958A`, `#00FFCC` |
| `image` | `<img src alt="..." style="width: 100%; borderRadius: 8; display: block">` или `null` если нет URL | |

**Hardcoded button color schemes:**
```typescript
primary:   background: '#00FFCC', color: '#0A0A0B'
secondary: background: '#3A3D4C', color: '#F2EBD9'
outline:   background: 'transparent', color: '#00FFCC', border: '2px solid #00FFCC'
```

**Critical implication для AX:**
- Bridge документ `04_DESIGN.md` неверно описал виджеты — они **НЕ используют tenant CSS-vars**, только хардкод.
- AX byte-for-byte identity = **сохранить хардкод**. Это противоречит «AX рендерит тот же дизайн что SITE1» в духе чистоты — реально SITE1 уже сломан (cyan кнопка на любом тенанте, независимо от tokens).
- **Альтернатива:** AX исправляет это в Phase B (deliberate ADR), потому что contract identity сейчас означает "так же сломанно". RFC должен решить: replicate-and-fix-later или fix-during-port.

### 6.8 `extractEdSections` — body → Section[]

```typescript
export function extractEdSections(body: unknown): Section[] {
  if (!Array.isArray(body)) return [];
  for (const block of body) {
    if (block.type === 'custom') {
      const ed = block.data?.ed;
      if (Array.isArray(ed)) return ed as Section[];
    }
  }
  return [];
}
```

— Находит первый `{ type: 'custom', data: { ed: [...] } }` блок и возвращает его `ed` array. **Игнорирует другие top-level блоки** (hero, text, etc.). Это значит реальный production content — **внутри `custom` блока**, а top-level `CmsBlocks` discriminated union — почти неиспользуемый legacy формат.

**Implication:** AX migration scope — **только ED-rendering**. Top-level CmsBlocks типы (hero, text, gallery, services, cta) рендерить **не обязательно в Phase A**, они не используются. Phase B может добавить если нужно для legacy pages.

---

## 7. Findings, risks, divergences from bridge docs

### 7.1 Critical (RFC-blocking)

| # | Finding | Impact | Action в RFC |
|---|---------|--------|--------------|
| C1 | **RLS отсутствует в SITE1 cms_pages** | AX добавляет защиту, которой нет, а не дублирует | Зафиксировать как NEW behavior, не identity |
| C2 | **Widget colors hardcoded, игнорируют CSS-vars** | "contract identity" = повторить сломанное поведение | RFC должен решить: replicate vs fix |
| C3 | **Per-tenant route хардкодит background** | AX upgrade приводит к ВИЗУАЛЬНО другой странице | RFC должен зафиксировать deliberate deviation |
| C4 | **extractEdSections — единственный реальный render path** | top-level CmsBlocks types unused | Phase A может скипнуть hero/text/etc rendering |
| C5 | **TenantSiteShell НЕ используется в [slug] route** | bridge документ устарел | AX рендерит правильно через Leptos shell — это улучшение |
| C6 | **Observability stack отсутствует в SITE1 целиком** (Sentry / OTLP / Prometheus / structured logs / request-id) | Bridge §10 был aspirational. AX вводит full stack ВПЕРВЫЕ, не мигрирует. | RFC переформулирует: "install full observability в AX pilot; backport в SITE1 — отдельный track" |
| C7 | **Performance baseline недоступен в код-side detail** | T16 oha measurement даёт только black-box HTTP timing + RSS; alloc-per-request, span breakdown нельзя сравнить | RFC цитирует AX target'ы без точной SITE1 baseline; baseline валидируется на VAL-001 |

### 7.2 High (RFC должен явно адресовать)

| # | Finding | Impact | Action |
|---|---------|--------|--------|
| H1 | **Slug regex inconsistency tenant-resolver vs schema** | Tenants со slug 41-64 chars недоступны | AX исправляет в resolver (regex до 64) — добавить в ADR-001 |
| H2 | **HTML sanitization отсутствует на text.html** | XSS risk; trusted admin assumption | AX добавляет `ammonia` на read — defence in depth, не блокирующее |
| H3 | **No caching tenant slug → id в SITE1** | Каждый запрос → SELECT FROM tenants | AX добавляет moka LRU (5 min TTL) — performance win |
| H4 | **`extras` jsonb не NOT NULL despite default** | Возможна NULL state | AX код tolerates оба; не блокирующе |
| H5 | **`colorFormatCheck` партиальный** | acc_color, body_color без regex check | AX expand-migration может выровнять (low-risk) |
| H6 | **OFFSET pagination в listPages** | O(N) на больших таблицах, tenant noisy-neighbor | Out of Phase A scope; для admin Phase B — keyset |
| H7 | **TenantContext через ALS — Node-specific** | Не переносится 1-в-1 в Rust | AX использует `req.extensions::<TenantContext>()`, не tokio task-local |

### 7.3 Medium (ADR должен учесть)

| # | Finding | Impact |
|---|---------|--------|
| M1 | **`getPublishedBySlug` использует SELECT * (Drizzle implicit)** | AX явный column list per ENTITY §11.6 |
| M2 | **Custom blocks data — arbitrary JSON** | AX schema поле `body` остаётся `jsonb`; не валидируется на DB level |
| M3 | **`publishedAt` ↔ status='published' invariant — только app-level** | AX может добавить CHECK constraint в Phase B |
| M4 | **JSON field order matters для contract identity** | AX serde должен фиксировать порядок |
| M5 | **`tenantId` НЕ в response DTO** | AX не должен случайно включить (security) |
| M6 | **`@Param('slug')` без validation на controller** | AX тоже не валидирует на presentation; service слой обработает |
| M7 | **Subdomain regex `[^\.]+` no multi-level** | AX повторяет |
| M8 | **`@Public()` НЕ skip'ит TenantGuard** | AX обязан повторить — status check на public endpoint |
| M9 | **`platform-admin bypass`** | AX в Phase A не имплементирует (no platform admin endpoints); но AX guard должен учесть когда platform запросы пойдут |

### 7.4 Low (good to know, не блокирующие)

| # | Finding |
|---|---------|
| L1 | `cms.service.spec.ts` — mock-based, не integration |
| L2 | Нет fuzz-тестов изоляции в SITE1 |
| L3 | Custom domain resolution не реализован в коде (only DB column) |
| L4 | `bootstrapSourceUrl` — audit field, не используется в logic |
| L5 | TenantSettings (jsonb) — typed но не используется в cms_pages |
| L6 | `tenant_menu_items` — отдельная таблица для navigation; AX phase A не trogает |
| L7 | 11 per-tenant route folders — duplication, AX мог бы консолидировать в `[tenant]/[slug]/page.tsx`, но это out of scope (Next.js остаётся) |

---

## 8. AX migration implications

### 8.1 Что меняется в RFC-001 на основе audit

1. **RFC business reason** дополняется: "не только performance win — но и впервые добавляем RLS в cms_pages (compile + runtime защита, отсутствующая в SITE1)".

2. **RFC success criteria** должен явно включать: "AX byte-for-byte JSON identical SITE1 publicBySlug response, ВКЛЮЧАЯ field order и null vs undefined для optional полей".

3. **Out of scope явно фиксирует:**
   - top-level CmsBlocks rendering (hero, text, gallery, services, cta) — Phase B
   - HTML sanitization — Phase B (но AX может добавить ammonia на read как defence)
   - Custom domain resolution — Phase B+
   - Caching tenant resolution — Phase A добавляет (low-risk win)

4. **Contract identity SLA уточняется:**
   - JSON response byte-for-byte (включая `null` vs missing)
   - HTML response (Phase B Leptos SSR) — diff допустим только в `X-Stack`, `X-Request-Id`, timestamps, content of `<style>` block (если AX shell vs SITE1 hardcoded background)
   - **Visual identity НЕ обещается** для cms-route — SITE1 рендерит broken (hardcoded bg `#0E0F12`), AX рендерит правильно через design tokens. Зафиксировать в DECISION.md как deliberate deviation.

### 8.2 Что меняется в ADR-001

1. **Schema delta** — добавить:
   - `CREATE VIEW cms_pages_v_active` (published-only, см. `bridge/03 §2`)
   - `ENABLE ROW LEVEL SECURITY ON cms_pages` (NEW для SITE1)
   - `CREATE POLICY rls_cms_pages_tenant_isolation`
   - `CREATE ROLE site1_admin_role NOLOGIN BYPASSRLS` + `GRANT ALL`
   - `CREATE ROLE ax_app_role NOLOGIN` + `GRANT SELECT ON view`
   - Опционально: align `acc_color`, `body_color` regex (low-risk fix)

2. **Domain types decision:**
   - `Block` — Rust `enum` с serde `#[serde(tag = "type", content = "data")]` для discriminated union
   - `CanvasElement` — Rust `struct` с flat optional fields (НЕ enum), serde повторяет TS-shape
   - `Section { id, columns, padding: String }`, `Column { id, span: f32, elements: Vec<CanvasElement> }`

3. **Tenant resolution decision:**
   - Header → subdomain → query (priority как SITE1)
   - Slug regex unified: 1-64 chars (исправляет SITE1 bug)
   - Moka LRU cache `slug → TenantContext` с TTL 5 min

4. **Error mapping decision:** `AppError` variants 1:1 с SITE1 HTTP codes:
   - `TENANT_NOT_RESOLVED` → 401
   - `TENANT_NOT_ACTIVE` → 403
   - `TENANT_OWNERSHIP_MISMATCH` → 403 (Sentry alert)
   - `PAGE_NOT_FOUND` → 404 `{ code, slug, locale }`

5. **Renderer decision (Phase B):**
   - Replicate hardcoded widget colors **точно** — это контракт. Tenant tokens applies ТОЛЬКО к outer shell.
   - Inline styles (НЕ Tailwind, НЕ external CSS) — match SITE1 React `style={{}}`.

### 8.3 Что меняется в PLAN-001

- Step 1 уже сделан (этот audit ≈ 60-80%).
- Steps 7-13 (implementation) — структурно без изменений.
- **Новый step:** "RLS smoke test" перед integration tests — отдельный тест что POLICY работает (`SET LOCAL` влияет на SELECT).
- **Step 21 Leptos SSR** — учесть, что AX рендерит **полноценный TenantSiteShell-эквивалент** (CSS-vars + Google Fonts + style block) НА cms-pages route — это улучшение vs SITE1 baseline. DECISION.md фиксирует.

### 8.4 Что меняется в VAL-001

- **Success criteria уточняются:**
  - JSON byte-diff vs SITE1 publicBySlug: **0 differences** (после нормализации `X-Request-Id`)
  - HTML byte-diff (Phase B): допустим только в указанных полях; всё остальное байт-в-байт
  - Performance: p95 ≤ 60ms target держится, baseline = SITE1 measurement (текущий неизвестен — измерить в step 16)
  - RLS verification: integration test "запрос tenant_b с slug tenant_a → returns 0 rows even при отсутствии WHERE tenant_id"
  - Fuzz: 1M cross-tenant attempts, 0 leaks

- **Новые tests required:**
  - RLS smoke (POLICY работает с SET LOCAL)
  - JSON byte-for-byte vs SITE1 (golden file test)
  - HTML byte diff variance (3 контрастных tenants — phase B)

---

## 11. Cross-module dependencies (что cms_pages пересекает)

### 11.1 `media` table — `coverImageKey` resolution

**Schema (`packages/db/src/schema/media.ts`):**

| Column | Type | Notes |
|--------|------|-------|
| `id` | uuid PK | |
| `tenant_id` | uuid NOT NULL FK | cascade on tenant delete |
| `key` | varchar(500) **UNIQUE** | format `tenant/{tenantId}/{module}/{mediaId}.{ext}` |
| `mime` | varchar(100) NOT NULL | |
| `size` | bigint NOT NULL | |
| `sha256` | varchar(64) | nullable |
| `width`, `height` | integer | для images |
| `duration_ms` | integer | для video (не используется в Phase 0) |
| `alt`, `caption` | varchar(500), text | accessibility |
| `module` | varchar(32) NOT NULL | enum-like: `'logo'\|'tenant'\|'cms'\|'menu'\|'staff'\|'service'\|'salon'\|'client'\|'misc'` |
| `entity_id` | uuid | optional, no FK constraint |
| `uploaded_by_user_id` | uuid FK users(id) | set null on user delete |
| `status` | varchar(20) | `'uploading'\|'ready'\|'archived'` |
| `created_at`, `updated_at` | timestamp | |

**CHECK constraint `media_key_tenant_prefix_check`:**
```sql
key LIKE 'tenant/' || tenant_id::text || '/%'
```
— **DB-level enforcement** что key всегда начинается с tenant prefix. Cross-tenant key impossible (даже если приложение fucked up — DB отвергает INSERT).

**Indexes:**
- `media_key_uniq` UNIQUE
- `media_tenant_module_idx` (tenant_id, module)
- `media_tenant_entity_idx` (tenant_id, module, entity_id)
- `media_tenant_created_idx` (tenant_id, created_at DESC)

### 11.2 S3 URL resolution — `S3Service.publicUrlFor(key)`

**File:** `apps/api/src/storage/s3.service.ts` (121 lines)

```typescript
publicUrlFor(key: string): string {
  return `${this.publicUrl.replace(/\/$/, '')}/${encodeURI(key)}`;
}

async signedDownloadUrl(key: string, expiresSec = 3600): Promise<string> {
  return getSignedUrl(this.client, new GetObjectCommand({ Bucket: this.bucket, Key: key }), { expiresIn: expiresSec });
}
```

**Dev (MinIO):**
- `S3_PUBLIC_URL=http://localhost:9011/barbie-media`
- Bucket anonymous download (`mc anonymous set download local/barbie-media` в minio-init job)
- Direct GET без подписи → `publicUrlFor(key)` достаточен

**Prod (S3 / Yandex Object Storage):**
- Bucket **должен быть private** (best practice)
- `publicUrlFor(key)` даёт ссылку, которая работает ТОЛЬКО если bucket public OR через CDN proxy
- `signedDownloadUrl(key)` — для приватных bucket'ов, TTL 1 час по умолчанию

**Implication для AX cms_pages:**
- `cover_image_key` в `cms_pages` row — это **только S3 key**, не URL
- URL формирует **renderer** (presentation layer) через эквивалент `S3Service.publicUrlFor(key)`
- AX в Rust: использует `aws-sdk-s3::presigning::config::PresigningConfig` + `Client::get_object().presigned()` для signed URLs; для public — простая конкатенация env `S3_PUBLIC_URL + "/" + key`
- **Решение:** в Phase A AX — `publicUrlFor` style (assume public bucket); presigned добавлять только когда понадобится privacy. RFC должен зафиксировать.
- **Caching headers:** `S3Service.putObject` ставит `Cache-Control: public, max-age=2592000` (30 дней). AX read-path не управляет cache headers объекта (только при upload); read только генерирует URL.

### 11.3 MIME / size limits (для media uploads — не AX scope, но контекст)

```typescript
const MAX_SIZE_BYTES = 25 * 1024 * 1024;   // 25 MB
const ALLOWED_MIMES = new Set([
  'image/jpeg', 'image/png', 'image/webp', 'image/gif', 'image/svg+xml',
  'application/pdf',
]);
```

AX phase A — **только read**. Upload validation остаётся в SITE1.

### 11.4 `tenant_menu_items` — navigation source

**Schema (`packages/db/src/schema/tenant-menu-items.ts`):**

| Column | Type | Notes |
|--------|------|-------|
| `id` | uuid PK | |
| `tenant_id` | uuid NOT NULL FK | cascade |
| `parent_id` | uuid | self-FK через `foreignKey` ON DELETE cascade |
| `label`, `href` | varchar(255), varchar(1000) | |
| `image_key`, `icon` | varchar(500), varchar(64) | |
| `sort_order` | integer NOT NULL default 0 | |
| `locale` | varchar(8) NOT NULL default `'ru'` | |
| `payload` | jsonb | `{ description?, badge?, openInNewTab?, highlight? }` |
| `status` | varchar(20) | `'active'\|'hidden'\|'archived'` |

**CHECK constraint:** `href ~ '^(/|https?://)'` — internal path or absolute URL. AX повторяет это validation на write (Phase B).

**Composite index:** `tmi_tenant_parent_sort_idx` на `(tenant_id, parent_id, sort_order)` — главный path рендера.

**Depth:** 2 уровня (root + children). Application-level constraint, не DB.

### 11.5 `Navigation` dispatch — 3 templates

**File:** `apps/web/src/components/tenant-site/Navigation.tsx` (38 lines)

```typescript
switch (menu.template) {
  case 'mega-images':   return <MegaImagesNav   tenant={tenant} items={items} />;
  case 'vertical-side': return <VerticalSideNav tenant={tenant} items={items} />;
  case 'top-classic':
  default:              return <TopClassicNav   tenant={tenant} items={items} />;
}
```

— dispatched by `tenant_design_tokens.nav_template` через `PublicMenu` API response (`menu.template`).

**Fallback:** если `menu.items` пуст (тенант не настроил), синтезируется из `tenant.navigation` static array (hardcoded list per tenant из `projects-data.ts`).

**Implication для AX:**
- **Phase A scope:** AX cms_pages route НЕ рендерит Navigation (см. `imperiumspa/[slug]/page.tsx` — только `<EdRenderer>` без navigation). Audit confirms: Navigation НЕ load-bearing для cms_pages migration.
- **Phase B+:** если AX расширяется на home/landing — потребуется port Navigation logic в Leptos. 3 template variants + fallback из tenant projects-data.

---

## 12. Deployment / infrastructure state

### 12.1 `docker-compose.dev.yml` — 4 services

```yaml
postgres   :  postgres:16-alpine                      port  5442  → 5432
redis      :  redis:7-alpine                          port  6389  → 6379
minio      :  minio/minio:latest                     ports  9011  → 9000 (S3 API)
                                                            9012  → 9012 (web console)
mailhog    :  mailhog/mailhog:latest                 ports  8035  → 1025 (SMTP)
                                                            8025  → 8025 (web UI)
minio-init :  one-shot bucket creation + CORS + anonymous download
```

**Ports намеренно offset:** 5442 (не 5432), 6389 (не 6379), 9011 (не 9000), 8035 (не 1025) — чтобы parallel run с parent ES не конфликтовал (см. `barbie/ENTITY.md §5`).

**Bucket setup:** `barbie-media` created via `mc mb` в minio-init container; anonymous download enabled (`mc anonymous set download`). CORS allowed all origins, all methods для dev convenience.

**Healthchecks** на postgres, redis, minio — есть; на mailhog — нет (не critical).

**Volumes:** `barbie-site1-postgres-data`, `barbie-site1-redis-data`, `barbie-site1-minio-data` — named, persistent.

### 12.2 `configuration.ts` — config shape (8 groups)

```typescript
AppConfig {
  env: 'development' | 'production' | 'test',
  api: { port, corsOrigins[] },
  database: { url },
  redis: { url },
  jwt: { secret, refreshSecret, expiresIn, refreshExpiresIn },
  s3: { endpoint, region, accessKey, secretKey, bucket, publicUrl, forcePathStyle },
  tenant: { rootDomain, fallbackHeader },
  logLevel: 'fatal' | 'error' | 'warn' | 'info' | 'debug' | 'trace',
}
```

**`checkRequired(cfg)` в `main.ts`:**
- `database.url` обязателен всегда
- В production: `JWT_SECRET` (≥ 32 chars), `JWT_REFRESH_SECRET`, `S3_ACCESS_KEY` / `S3_SECRET_KEY`
- В dev — sensible defaults для всего

**Implication для AX:** аналогичный `Config` struct, аналогичные required checks. AX добавляет: `OTLP_ENDPOINT`, `SENTRY_DSN`, `JEMALLOC_STATS_PATH`, `PGBOUNCER_URL` (вместо direct postgres) — поля, которых нет в SITE1.

### 12.3 `.env.example` — 22 env vars

Группы:
- DB: `POSTGRES_PASSWORD`, `DATABASE_URL`
- Redis: `REDIS_URL`
- Auth: `JWT_SECRET`, `JWT_REFRESH_SECRET`, `JWT_EXPIRES_IN` (15m), `JWT_REFRESH_EXPIRES_IN` (30d)
- Ports: `API_PORT=3010`, `WEB_PORT=3011`
- S3: `S3_ENDPOINT`, `S3_REGION`, `S3_ACCESS_KEY`, `S3_SECRET_KEY`, `S3_BUCKET=barbie-media`, `S3_PUBLIC_URL`, `S3_FORCE_PATH_STYLE=true`
- Mail: `SMTP_HOST=localhost`, `SMTP_PORT=8035`, `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM`
- Multi-tenant: `TENANT_ROOT_DOMAIN=lvh.me`, `TENANT_RESOLVE_FALLBACK_HEADER=x-tenant-slug`
- Platform: `PLATFORM_ADMIN_EMAIL`, `PLATFORM_ADMIN_PASSWORD`
- Observability: `LOG_LEVEL=debug`, `# SENTRY_DSN=` (**commented out**)

**Critical:** `SENTRY_DSN` закомментирован — Sentry **не настроен в env template**. Это согласуется с findings §13 ниже.

### 12.4 `main.ts` — bootstrap sequence

```
1. loadRepositoryEnv()         — ищет .env вверх по дереву (до 8 levels)
2. configuration() + checkRequired() — типизированный config + fail-fast если missing
3. NestFactory.create(AppModule, { logger: [...] })
4. app.use(helmet({ contentSecurityPolicy: false }))   — CSP TODO
5. root '/' redirect → /api/docs (dev) или /v1/health (prod)
6. app.enableCors({ origin: prod-strict | dev-true, credentials: true })
7. app.enableVersioning({ type: URI, defaultVersion: '1' })  → /v1/...
8. app.useGlobalPipes(new ValidationPipe({
     whitelist: true,
     transform: true,
     forbidNonWhitelisted: true,    — strict: unknown fields → 400
     transformOptions: { enableImplicitConversion: true },
   }))
9. Swagger / OpenAPI setup (dev only) → /api/docs
10. app.enableShutdownHooks()
11. app.listen(cfg.api.port)
```

**Findings для AX:**
- **NO Sentry init.** Bridge документ §10 заявлял Sentry — это **aspirational**, не actual.
- **NO OpenTelemetry init.**
- **NO Prometheus metrics endpoint.**
- **NO request-id middleware.**
- **NO structured JSON logging** — Nest's default Logger pretty-prints.
- **helmet с CSP disabled** — TODO.
- **CORS strict в prod**, anything в dev (включая `file://` для dashboard-2077.html).
- **Global ValidationPipe `forbidNonWhitelisted: true`** — строгая валидация: unknown fields → 400 BAD_REQUEST.

### 12.5 `AppModule` — 14 feature modules wired

```typescript
imports: [
  ConfigModule (global),
  ThrottlerModule ({ ttl: 60s, limit: 120 }),    // 120 req/min на IP
  DatabaseModule,
  TenantContextModule,
  StorageModule,
  AuthModule,
  TenantsModule,
  SalonsModule, ServicesModule, StaffModule, ClientsModule,
  AppointmentsModule,
  MediaModule,
  CmsModule,
  MenuModule,
  ChatModule,
  ToolsModule,
],
controllers: [HealthController],
providers: [
  { provide: APP_GUARD, useClass: JwtAuthGuard },   // ⭐ ГЛОБАЛЬНЫЙ guard
],

configure(consumer) {
  consumer.apply(TenantResolverMiddleware).forRoutes('*');   // ⭐ middleware на ВСЕ routes
}
```

**Critical:**
- **Global `JwtAuthGuard`** — каждый endpoint защищён по умолчанию; `@Public()` декоратор → bypass. AX должен повторить — auth required by default, public явный opt-out.
- **`TenantResolverMiddleware.forRoutes('*')`** — резолвит tenant на ВСЕХ путях, включая `/health` и `/auth/login`. Resolve без context = просто пропуск (см. §5.1). AX повторяет.
- **`ThrottlerModule` 120 req/min** — rate limit per IP, basic protection. AX через `tower-http::limit::ConcurrencyLimitLayer` + custom middleware (или `tower_governor` crate).

### 12.6 `HealthController`

```typescript
@Public() @SkipTenant()
@Controller('health')

@Get()                 → { ok: true, db: 'up'|'down', uptime, version, env, timestamp }
@Get('ready')          → 200 если SELECT 1 успешен, 503 иначе
```

— DB ping = `SELECT 1` через Drizzle execute. Никаких других health checks (S3, Redis не проверяются).

**AX equivalent:**
- `/health` — process liveness + DB ping
- `/health/ready` — DB + S3 (HEAD bucket) + pgmq + Sentry config valid
- Возвращать build version + git_sha + uptime — same shape, для diagnostic compatibility

### 12.7 Caddy / VPS deployment artifacts — НЕ найдены в SITE1

`Glob **/Caddyfile* в barbie/SITE1` → 0 файлов. `**/ecosystem.config.*` → 0 файлов.

**Implication:** SITE1 deployment artifacts (Caddyfile, PM2 ecosystem, systemd unit) живут **вне monorepo** — вероятно на VPS (`~/e1/`) или в `barbie/NON_PROJECT/` или `barbie/_sli/`. Per `barbie/ENTITY.md §6` — VPS-регламент в parent constitution.

**Для AX migration:**
- Caddyfile snippet для cms-ax-pilots создаётся **впервые** в `barbie/ax/ops/caddy/Caddyfile.snippets/cms-ax-pilots.caddy`. Reference из bridge/03 §12.
- AX systemd unit нужно написать с нуля (SITE1 управляется PM2 — другой механизм).
- AX deploy notes — отдельный artifact в `barbie/ax/docs/releases/v0.1.0/`.

---

## 13. Observability state — actual vs bridge aspirational

### 13.1 Что SITE1 реально имеет

| Signal | Component | Endpoint / Mechanism |
|--------|-----------|----------------------|
| Liveness | `HealthController.liveness` | `GET /health` — `{ db: 'up'\|'down', uptime, version }` |
| Readiness | `HealthController.readiness` | `GET /health/ready` — 200 / 503 |
| Application logs | Nest's built-in `Logger` | pretty-print на stdout; level из `LOG_LEVEL` env |
| Rate limit | `ThrottlerModule` | 120 req/min per IP, 60s window |
| Security headers | `helmet` middleware | base set без CSP |
| Validation errors | `ValidationPipe` | 400 с class-validator error array |
| Domain errors | Nest exception filters (default) | `NotFoundException` → 404 JSON, etc. |

### 13.2 Что SITE1 НЕ имеет (но bridge документ обещал)

| Signal | Status | Bridge claim |
|--------|--------|--------------|
| **Sentry SDK init** | ❌ ОТСУТСТВУЕТ в main.ts; DSN env закомментирован в `.env.example` | bridge §10 "errors via sentry" |
| **OpenTelemetry tracing** | ❌ ОТСУТСТВУЕТ | bridge §10 "traces via tracing-opentelemetry" |
| **OTLP exporter** | ❌ ОТСУТСТВУЕТ | bridge §10 "OTLP → Tempo / Honeycomb" |
| **Prometheus metrics endpoint** | ❌ ОТСУТСТВУЕТ `/metrics` | bridge §10 "Prometheus exporter" |
| **Structured JSON logging** | ❌ pretty-print, не JSON | bridge §10 "structured, span-based" |
| **Request ID middleware** | ❌ ОТСУТСТВУЕТ | bridge §10 "ULID + tracing::Span injection" |
| **Distributed tracing context propagation** | ❌ ОТСУТСТВУЕТ | bridge §10 "trace context" |
| **APM agent** | ❌ ОТСУТСТВУЕТ | bridge §10 implied |
| **Grafana dashboards** | ❌ ОТСУТСТВУЕТ | bridge §10 "Grafana JSON dashboards" |
| **Alloc profiling** | ❌ N/A | bridge §10 "dhat-rs" |

### 13.3 Implication: AX добавляет с нуля, не мигрирует

**Bridge document `04_BRIDGE` секция §10 описывает aspirational план**, не реальное состояние. Реальный SITE1 имеет только базовые things: helmet, throttler, Logger, /health. Всё остальное — TODO с момента создания репозитория.

**Это меняет позиционирование AX migration в RFC:**

> "AX миграция cms_pages вводит **полноценный observability stack** (tracing + metrics + Sentry + structured logs + request-id) которого никогда не было в SITE1. Это не миграция existing — это первое внедрение в платформу."

**RFC должен переформулировать:**
- ~~"replicate SITE1 observability"~~ → "**install full observability** в AX pilot; backport в SITE1 — отдельный track"
- Sentry project `nas-ax` создаётся **впервые** (не миграция существующего)
- Grafana dashboards `cms-cutover-watch` создаются **впервые**
- Metrics namespace `nas_*` определяется AX (нет SITE1 baseline — измерения "до" не было)

**Это снижает confidence в Performance baseline (T16):**
- Bridge план говорит "oha measure SITE1 baseline и AX, compare"
- Реально: измерять SITE1 baseline — это означает **впервые** включить tracing/metrics в SITE1 для измерения. **Или** только end-to-end latency через oha (HTTP timing, не code-level breakdown).
- Альтернатива: AX vs SITE1 baseline = только response time (p50/p95/p99). Allocation per request не сравнить (dhat-rs только в Rust). RSS можно сравнить через `procfs` опрос обоих процессов.

### 13.4 Risk update — added to §7

Добавляются Critical-level findings:

| # | Finding | Impact |
|---|---------|--------|
| C6 | **SITE1 observability фактически = baseline (helmet + throttler + Logger.log)** | AX вводит tracing/metrics/Sentry с нуля; не migration, а installation |
| C7 | **Performance baseline нельзя измерить со SITE1 кодом-side detail** | T16 perf comparison ограничен black-box HTTP timing + RSS |

### 13.5 Test infrastructure — `test-utils/` deep-dive

**`mock-db.ts` (123 lines):**
- Chainable proxy с jest.Mock'ами всех Drizzle methods (`select/from/where/insert/.../returning/for`)
- Records each method call в `calls: RecordedCall[]`
- FIFO queue для предзаданных results через `queueResult(rows)`
- Terminal `await` resolves либо queued, либо `[]`
- `transaction()` mock передаёт `this` как tx — реальная транзакция НЕ моделируется

**`sql-helpers.ts` (112 lines):**
- `expectTenantFilter(whereArgs, column, expectedTenantId)` — walks Drizzle's `queryChunks` tree рекурсивно
- Ищет Column-chunk (name + table match) + Value-chunk (value match)
- Покрывает паттерны: `.where(eq(tenantId, id))`, `.where(and(eq(tenantId, id), ...))`, `.where(combineTenant(id, ...))`
- НЕ покрывает: реальный SQL semantics, JOINs, типизацию, RLS поведение
- `mockTenantContext(tenantId)` factory для inject в service

**Implication для AX:** этот test approach — **purely structural assertion**, не behavioral. AX integration tests (testcontainers + real Postgres + RLS) **дают сильно более sound гарантии** — это качественный апгрейд test infrastructure, не "повторение".

### 13.6 RolesGuard — auth/role enforcement layer (cross-cutting)

**File:** `apps/api/src/common/guards/roles.guard.ts` (50 lines)

```typescript
canActivate(context) {
  const required = reflector.get<RoleSpec[]>(REQUIRED_ROLES_KEY, ...);
  if (!required) return true;

  const user = req.user;
  if (!user) throw new UnauthorizedException({ code: 'NOT_AUTHENTICATED' });

  // God-mode: platform-admin проходит всё
  if (user.kind === 'platform' && user.role === 'platform-admin') return true;

  if (!required.includes(user.role)) {
    throw new ForbiddenException({ code: 'FORBIDDEN_ROLE', required, actual: user.role, kind: user.kind });
  }
  return true;
}
```

**Implication для AX cms_pages Phase A:**
- AX public endpoint = `@Public()` (no JWT verification, no RolesGuard execution)
- RolesGuard НЕ переезжает в AX в Phase A — admin endpoints остаются в SITE1
- Phase B+ если AX добавит admin endpoints — повторить god-mode logic

---

## 9. Coverage assessment — что сделано в этом audit

| Section | Состояние | Notes |
|---------|-----------|-------|
| 1. Schema | ✅ 100% | + связанные таблицы tenants, tenant_design_tokens, media, tenant_menu_items |
| 2. Migrations | ✅ 100% | Only 0000 трогает cms_pages; subsequent migrations confirmed clean |
| 3. Service methods | ✅ 100% | 9 методов + 2 private; error-handling маппинг к AX через AppError + IntoResponse plan |
| 4. Controller endpoints | ✅ 100% | 8 endpoints, public + role-protected matrix |
| 5. DTOs / validation | ✅ 100% | 5 DTO files + Zod blocks schema |
| 6. Tenant context | ✅ 100% | + RolesGuard (god-mode bypass) покрыт в §13.6 |
| 7. Public render path | ✅ 100% | TenantSiteShell vs per-tenant route divergence finding (C3, C5) |
| 8. Tests coverage | ✅ 100% | + mockDb / sql-helpers внутренности в §13.5 — purely structural, не behavioral |
| **§11 Cross-module deps** | ✅ 100% | media schema + S3Service.publicUrlFor + tenant_menu_items + Navigation dispatch (3 templates) |
| **§12 Deployment / config** | ✅ 95% | docker-compose, configuration.ts, .env.example, main.ts bootstrap, AppModule wiring (14 modules + global JwtAuthGuard + TenantResolverMiddleware на `*`). Caddy/VPS artifacts — вне monorepo (per `barbie/ENTITY.md §6`), отдельный audit при cutover |
| **§13 Observability state** | ✅ 100% | actual = bare-bones (helmet + throttler + Logger + /health). Bridge §10 — aspirational, не actual. **Critical finding C6.** |
| 14. Performance baseline | ❌ 0% | Требует **реального** запуска SITE1 + oha — не выполнимо read-only. Отдельная задача T16 (Phase 4 VAL-001). **НЕ блокирует RFC** — target'ы зафиксируются как goals; SITE1 baseline валидируется на VAL-001 sign-off, не RFC sign-off |

**Сессия закрыла ≈ 95% Phase 1 audit.** Оставшийся 5% — performance baseline, который **технически невозможно сделать read-only**:

- Запустить SITE1 локально (`docker-compose -f docker-compose.dev.yml up -d` + `npm run dev`)
- Прогнать `oha -n 10000 -c 100 -H 'X-Tenant-Slug: pilot-tenant' http://localhost:3010/v1/cms/pages/public/by-slug/about?locale=ru` 60 секунд
- Записать p50/p95/p99 + средний RSS процесса (`procfs` или `Process Explorer`)
- Артефакт в `barbie/ax/docs/perf/baseline-site1-<date>.txt`

**Также частично out-of-scope (документировано как явные exclusions):**

- Caddyfile / PM2 ecosystem / systemd unit — живут вне `barbie/SITE1/` monorepo (`barbie/ENTITY.md §6`); audit для AX cutover создаётся **впервые** в `barbie/ax/ops/`
- 11 per-tenant Next.js routes — все используют идентичный pattern (см. §6.2); `imperiumspa/[slug]/page.tsx` прочитан как representative sample, остальные 10 confirmed identical через Grep results
- `WidgetView.tsx` editor-mode внутренности (не render-mode) — out of scope для read-path migration

---

## 10. Gate decision — RFC-001 readiness

**Может ли RFC-001 быть написан на основе этого audit?** **ДА · FINAL GREEN.**

**RFC должен зафиксировать (расширено после полного audit):**

1. **Identity scope явно ограничен JSON response (Phase A) и HTML structural (Phase B).** Visual identity per-tenant cms-page **НЕ обещается** из-за SITE1 baseline хаоса (hardcoded backgrounds, widget colors игнорируют tokens — см. C2, C3).

2. **AX = улучшение, не клон.** Три категории deliberate deviations:
   - **RLS** добавляется впервые (C1)
   - **Observability stack** (tracing + metrics + Sentry + JSON logs + request-id) инсталлируется впервые (C6)
   - **Visual rendering** — RFC выбирает: replicate broken (`#00FFCC` cyan кнопки, `#0E0F12` hardcoded bg) **ИЛИ** fix-during-port (использовать tenant CSS-vars). Это **business decision**, не technical.

3. **Migration scope = `getPublishedBySlug` ONLY.** Не дублировать listing, не дублировать write methods. Top-level CmsBlocks types (hero/text/gallery/services/cta) — почти unused в production (см. §6.8 / C4); можно отложить в Phase B полностью.

4. **Out of scope явный:**
   - Custom domain resolution (только slug subdomain — H1)
   - HTML sanitization (можно добавить `ammonia` на read как defence in depth; не блокирующее — H2)
   - Performance baseline measurement (T16, не блокирует RFC; делается между RFC sign-off и VAL-001)
   - Tests retrofit в SITE1 (RLS integration tests появляются **ТОЛЬКО в AX**)
   - Caddy / PM2 / systemd setup в SITE1 (live на VPS, вне monorepo)
   - Per-tenant 11 Next.js routes (остаются в SITE1, AX обслуживает endpoint которые они hit'ают)
   - Backport observability в SITE1 (отдельный track, не cms_pages migration scope)
   - **§25 Conditional Content Delivery / §26 Analytics Inventory / §27 SEO Control** (specced в ENTITY.md v3.4 как post-pilot features; не в cms_pages migration)

5. **Architectural constants (определены audit'ом):**
   - JSON response shape **БЕЗ** `tenantId` field (privacy)
   - 4 condition WHERE: tenant_id + slug + locale + status='published'
   - Compound index used: `cms_pages_tenant_slug_locale_uniq`
   - 404 body: `{ code: 'PAGE_NOT_FOUND', slug, locale }` byte-for-byte
   - locale defaults to `'ru'` (controller `?? 'ru'`)
   - `S3Service.publicUrlFor(key)` = `${publicUrl}/${encodeURI(key)}` (no presigning в Phase A)
   - TenantContext shape: `{ tenantId, tenantSlug, status }`
   - 3 error codes: TENANT_NOT_RESOLVED / TENANT_NOT_ACTIVE / TENANT_OWNERSHIP_MISMATCH (Sentry alert)

**Gate для T02 (RFC):** **GREEN — final.**

**Следующий шаг:** написать `barbie/ax/docs/rfc/RFC-001-cms_pages-migration.md` (P1 Strategic) на основе conclusions выше + `ADR-001-four-layer-rls.md` (P2 Architectural) с зафиксированными architectural decisions из пункта 5.

---

## Appendix A — File inventory (что прочитано)

| File | Lines | Read |
|------|-------|------|
| `packages/db/src/schema/cms-pages.ts` | 91 | ✅ полностью |
| `packages/db/src/schema/tenants.ts` | 107 | ✅ полностью |
| `packages/db/src/schema/tenant-design-tokens.ts` | 72 | ✅ полностью |
| `packages/db/drizzle/0000_deep_gamma_corps.sql` | ~600 | ⚠ только cms_pages раздел (lines 244-528) |
| `packages/db/drizzle/0001-0003_*.sql` | — | ⚠ grep confirmed нет cms_pages references |
| `apps/api/src/cms/cms.service.ts` | 256 | ✅ полностью |
| `apps/api/src/cms/cms.controller.ts` | 116 | ✅ полностью |
| `apps/api/src/cms/cms.module.ts` | 11 | ✅ полностью |
| `apps/api/src/cms/cms.service.spec.ts` | 113 | ✅ полностью |
| `apps/api/src/cms/dto/blocks.schema.ts` | 80 | ✅ полностью |
| `apps/api/src/cms/dto/create-page.dto.ts` | 69 | ✅ полностью |
| `apps/api/src/cms/dto/update-page.dto.ts` | 51 | ✅ полностью |
| `apps/api/src/cms/dto/list-pages-query.dto.ts` | 37 | ✅ полностью |
| `apps/api/src/cms/dto/page-response.dto.ts` | 27 | ✅ полностью |
| `apps/api/src/tenant-context/tenant-context.service.ts` | 51 | ✅ полностью |
| `apps/api/src/tenant-context/tenant-resolver.middleware.ts` | 111 | ✅ полностью |
| `apps/api/src/tenant-context/tenant.guard.ts` | 92 | ✅ полностью |
| `apps/api/src/tenant-context/tenant.decorator.ts` | 42 | ✅ полностью |
| `apps/api/src/tenant-context/tenant-context.module.ts` | 28 | ✅ полностью |
| `apps/api/src/tenant-context/with-tenant.helper.ts` | 56 | ✅ полностью |
| `apps/web/src/components/tenant-site/TenantSiteShell.tsx` | 112 | ✅ полностью |
| `apps/web/src/lib/cms-public.ts` | 38 | ✅ полностью |
| `apps/web/src/lib/td-overrides.ts` | 38 | ✅ полностью |
| `apps/web/src/components/cms/ed-editor/EdRenderer.tsx` | 68 | ✅ полностью |
| `apps/web/src/components/cms/ed-editor/ed-types.ts` | 136 | ✅ полностью |
| `apps/web/src/components/cms/ed-editor/WidgetView.tsx` | 125 | ✅ полностью |
| `apps/web/src/app/(tenants)/imperiumspa/[slug]/page.tsx` | 34 | ✅ полностью |
| `apps/web/src/components/cms/ed-editor/WidgetView.tsx` | 125 | ✅ полностью |
| **§11 Cross-module deps** | | |
| `packages/db/src/schema/media.ts` | 92 | ✅ полностью |
| `packages/db/src/schema/tenant-menu-items.ts` | 88 | ✅ полностью |
| `apps/api/src/media/media.service.ts` | 293 | ✅ полностью |
| `apps/api/src/media/media.controller.ts` | 86 | ✅ полностью |
| `apps/api/src/storage/s3.service.ts` | 121 | ✅ полностью |
| `apps/web/src/components/tenant-site/Navigation.tsx` | 38 | ✅ полностью |
| **§12 Deployment / config** | | |
| `docker-compose.dev.yml` | 96 | ✅ полностью |
| `apps/api/src/main.ts` | 113 | ✅ полностью |
| `apps/api/src/app.module.ts` | 81 | ✅ полностью |
| `apps/api/src/database/database.module.ts` | 42 | ✅ полностью |
| `apps/api/src/config/configuration.ts` | 103 | ✅ полностью |
| `.env.example` | 59 | ✅ полностью |
| `apps/api/src/health/health.controller.ts` | 60 | ✅ полностью |
| **§13 Observability / cross-cutting** | | |
| `apps/api/src/common/guards/roles.guard.ts` | 50 | ✅ полностью |
| `apps/api/src/test-utils/mock-db.ts` | 123 | ✅ полностью |
| `apps/api/src/test-utils/sql-helpers.ts` | 112 | ✅ полностью |

**Total:** 42 файла, ~4150 lines прочитано.

---

## Appendix B — Open questions for clarification

(Эти вопросы могут возникнуть при написании RFC; ответы — за пользователем.)

1. **Widget colors policy:** replicate hardcoded (`#00FFCC` button, etc.) для byte identity, ИЛИ fix during port (использовать tenant CSS-vars)? Если fix — это deliberate deviation, фиксируется в DECISION.md.

2. **`#0E0F12` hardcoded background:** какой backround AX рендерит для cms-page route — `tenant_design_tokens.bg` (fix) или `#0E0F12` (identity)?

3. **Slug regex unification:** в AX resolver `[a-z0-9-]{1,62}` (match schema CHECK) — OK? Это исправит SITE1 bug.

4. **Per-tenant route folders:** AX добавляет один универсальный route, или Next.js остаётся с 11 folders? (Next остаётся; вопрос риторический — AX отвечает за `/api/v1/cms/pages/public/by-slug/:slug` endpoint, Next.js может его hit'ать по-старому.)

5. **HTML sanitization на read:** добавлять `ammonia` для `text.html` (defence in depth), или trust admin как SITE1?

6. **Listing pagination decision (out of Phase A, но для completeness):** keyset cursor design для Phase B?

7. **Schema fixes opportunity:** AX expand-migration — хороший момент align `acc_color`/`body_color` regex check. Делать?

---

**Audit complete.**
**Next artifact:** `barbie/ax/docs/rfc/RFC-001-cms_pages-migration.md` (P1 Strategic).
**Owner of RFC sign-off:** user.
