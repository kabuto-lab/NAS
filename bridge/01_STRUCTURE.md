# BRIDGE · 01 — STRUCTURE (SITE1 → AX mapping)

> Назначение: дать AI-кодеру **карту территории** — где что лежит в production-target SITE1, и куда это переезжает в Cargo workspace AX. Без этого документа агент будет переоткрывать монорепо grep-ом.
>
> Источник правды по решениям — `barbie/AX/ENTITY.md` (главное §2 Four-Layer, §5 структура проекта). Этот файл — навигационный indexer, не заменяет конституцию.

---

## 1. SITE1 — текущий production-target

```
F:\Users\a\Documents\_DEV\Tran\ES\barbie\SITE1\
├── apps/
│   ├── api/                          NestJS 10 backend (port 3010)
│   │   └── src/
│   │       ├── app.module.ts                       [SPINE — не трогать]
│   │       ├── main.ts                             bootstrap, helmet, CORS, swagger
│   │       ├── database/database.module.ts          DRIZZLE provider (postgres-js)
│   │       ├── auth/                                JWT, bcrypt, sessions
│   │       ├── tenant-context/                     ★ load-bearing для AX перевода
│   │       │   ├── tenant-context.service.ts        ALS (AsyncLocalStorage) wrapper
│   │       │   ├── tenant-resolver.middleware.ts   subdomain | X-Tenant-Slug | ?tenant=
│   │       │   ├── tenant.guard.ts                  Layer 1 защиты
│   │       │   ├── tenant.decorator.ts              @CurrentTenant
│   │       │   └── with-tenant.helper.ts           Layer 2 защиты (Drizzle WHERE injection)
│   │       ├── common/
│   │       │   ├── guards/roles.guard.ts            RBAC
│   │       │   └── decorators/require-role.decorator.ts
│   │       ├── cms/                                ★ pilot модуль для AX
│   │       │   ├── cms.module.ts
│   │       │   ├── cms.controller.ts                @Controller('cms/pages') v1
│   │       │   ├── cms.service.ts                   list / get / getPublishedBySlug / publish / ...
│   │       │   ├── cms.service.spec.ts              Stage 34 isolation tests (mock-based)
│   │       │   └── dto/
│   │       │       ├── blocks.schema.ts             Zod-схема CmsBlocks
│   │       │       ├── create-page.dto.ts
│   │       │       ├── update-page.dto.ts
│   │       │       ├── list-pages-query.dto.ts
│   │       │       └── page-response.dto.ts
│   │       ├── tenants/                            создание/обновление тенантов + design-tokens
│   │       ├── salons/ services/ staff/ clients/   CRUD модули (НЕ переезжают в Phase A)
│   │       ├── appointments/                       FSM + overlap (НЕ переезжает в Phase A)
│   │       ├── chat/                               SSE + channels (НЕ переезжает)
│   │       ├── media/                              S3/MinIO (рассматривается 2-м после cms)
│   │       ├── tools/                              site analyzer, wp-probe (НЕ переезжает)
│   │       └── test-utils/                         mock-db + sql-helpers (для unit-тестов)
│   │
│   └── web/                          Next.js 15 App Router (port 3011)
│       └── src/
│           ├── app/
│           │   ├── admin/                          /admin/* — UI staff'а (НЕ переезжает в AX)
│           │   │   ├── login | dashboard | projects | cms | tenants | settings | ...
│           │   │   └── AdminShell.tsx
│           │   └── (tenants)/                     ★ публичные tenant-сайты
│           │       ├── imperiumspa/
│           │       │   ├── page.tsx                 home (TenantSiteShell fallback)
│           │       │   └── [slug]/page.tsx          ED-render через EdRenderer  ← AX заменяет
│           │       ├── barbiespa/, dachaspa/, …    9 других тенантов с тем же шаблоном
│           │       └── ...
│           ├── components/
│           │   ├── tenant-site/                   ★ public render (то, что AX переписывает)
│           │   │   ├── TenantSiteShell.tsx          CSS-vars + Google Fonts + layout
│           │   │   ├── Navigation.tsx               3 nav templates
│           │   │   ├── TenantEditFab.tsx            FAB для админов (опц., НЕ переезжает)
│           │   │   └── sections/Hero|Programs|…    home-page sections (fallback пути)
│           │   ├── cms/ed-editor/                 ★ ED-renderer (read-side нужен AX)
│           │   │   ├── EdRenderer.tsx               рисует Section[] из block 'custom'
│           │   │   ├── extractEdSections()           normalize body → Section[]
│           │   │   └── SandboxEditor.tsx            edit-side (НЕ переезжает)
│           │   └── admin/                          /admin/* UI (НЕ переезжает)
│           └── lib/
│               ├── api-client.ts                   apiFetch helper
│               ├── tenants.ts                      fetchPublicTenant (SSR)
│               ├── cms-public.ts                  ★ fetchPublicCmsPage (SSR)
│               ├── cms-api.ts                      client CRUD (НЕ для AX)
│               ├── td-overrides.ts                ★ ?td=base64 декодер
│               └── tenants-design-tokens-api.ts    /admin/projects → API
│
└── packages/
    └── db/
        └── src/schema/                            ★ Drizzle schema — load-bearing для AX миграции
            ├── tenants.ts                          ★ корневая мульти-тенант таблица
            ├── tenant-design-tokens.ts             ★ 1:1 с tenants, дизайн-токены
            ├── tenant-menu-items.ts                hierarchical menu
            ├── cms-pages.ts                       ★ ПИЛОТ: эту таблицу читает AX
            ├── users.ts | sessions.ts | platform-admins.ts
            ├── salons.ts | services.ts | staff.ts | clients.ts
            ├── appointments.ts | media.ts
            ├── chat-*.ts (×4)
            └── audit-log-*.ts (×2)

    └── drizzle/                                   ★ applied миграции (read-only для AX)
        ├── 0000_deep_gamma_corps.sql               initial 17 tables
        ├── 0001_greedy_molten_man.sql              (нужно проверить — schema delta)
        ├── 0002_chat.sql
        └── 0003_tenant_bootstrap.sql
```

**Маркеры:**
- `[SPINE — не трогать]` — конституция запрещает AVTONOM-правки (см. `CLAUDE.md §M`)
- `★ load-bearing для AX перевода` — обязательное чтение перед написанием соответствующего Rust-файла
- `НЕ переезжает в Phase A` — остаётся в SITE1; cross-stack контракт через §12.5 Interop

---

## 2. AX — целевой Cargo workspace (после bootstrap + cms_pages pilot)

Дословно из `barbie/AX/ENTITY.md §5` + дополнения из §12.6.2 L3 порядка файлов:

```
F:\Users\a\Documents\_DEV\Tran\ES\barbie\AX\
├── ENTITY.md                          [SPINE] конституция, v3.2
├── PROMPT_FOR_AI_CODER.md             промпт-входная точка
├── STATUS.md                          честный assessment Phase 3 pre-conditions
├── Cargo.toml                         workspace manifest (resolver = "2")
├── rust-toolchain.toml                channel = "1.84", components rustfmt+clippy
├── .gitignore                         target/, .sqlx/, *.profraw, *.profdata
├── deny.toml                          cargo-deny config (§19)
├── rustfmt.toml | clippy.toml         editor + lint config
│
├── crates/
│   ├── common/                        TenantId, UserId, RequestId (newtype), AppError, Page<T>
│   │   ├── src/lib.rs
│   │   ├── src/tenant.rs              ★ TenantId(Uuid) — Copy + Hash, см. §6
│   │   ├── src/ids.rs                  UserId, RequestId (ULID)
│   │   ├── src/error.rs                AppError enum + IntoResponse
│   │   └── src/page.rs                 Page<T> { items, next_cursor, total }
│   │
│   ├── domain/                        value objects + aggregates, ZERO внешних deps кроме serde/chrono/garde/uuid
│   │   ├── src/lib.rs
│   │   └── src/cms/
│   │       ├── mod.rs                  pub mod aggregate; pub use ...;
│   │       ├── value_objects.rs        PageId, PageSlug (validated), PageLocale, PublishedAt
│   │       └── aggregate.rs            PublishedPage { ... } + invariants методы
│   │
│   ├── application/                   use cases + port traits, depends on domain + common
│   │   ├── src/lib.rs
│   │   ├── src/ports/
│   │   │   ├── mod.rs
│   │   │   └── cms.rs                  trait CmsRepository (read-only); trait CmsRenderer
│   │   └── src/use_cases/cms/
│   │       ├── mod.rs
│   │       └── get_published_by_slug.rs
│   │
│   ├── infrastructure/                SQLx impls, S3 client adapters, depends on application + domain
│   │   ├── src/lib.rs
│   │   ├── src/persistence/
│   │   │   ├── pool.rs                 PgPool factory, sqlx::PgPoolOptions
│   │   │   ├── transaction.rs          with_tenant(pool, ctx, |tx| async { ... })  ← §6
│   │   │   ├── cms_pages_repo.rs       impl CmsRepository → sqlx::query_as! на view
│   │   │   └── views/
│   │   │       ├── mod.rs
│   │   │       └── cms_views.rs        typed wrappers для cms_pages_v_active
│   │   └── src/observability/         tracing-subscriber init, OTLP exporter
│   │
│   └── presentation/                  Axum router + Leptos SSR + OpenAPI, depends on application + common
│       ├── src/lib.rs
│       ├── src/app_state.rs            AppState { repos: Arc<dyn ...>, renderer: Arc<dyn ...> }
│       ├── src/middleware/
│       │   ├── tenant_resolver.rs      analog SITE1 middleware из §6
│       │   ├── request_id.rs           ULID + tracing::Span injection
│       │   └── error_to_response.rs
│       ├── src/api/
│       │   └── cms_handlers.rs         GET /api/v1/cms/pages/public/by-slug/{slug}
│       └── src/leptos/
│           ├── cms_page_component.rs   Section[] → IntoView (SSR)
│           └── tenant_shell.rs         CSS-vars equivalent of TenantSiteShell
│
├── migrations/
│   └── 0001_cms_pages_expand.sql      CREATE VIEW cms_pages_v_active + RLS POLICY
│
├── tests/
│   ├── integration/
│   │   └── cms_pages_test.rs          testcontainers + real Postgres + RLS verify
│   └── fuzz/
│       └── cms_tenant_isolation.rs    proptest 1M cross-tenant attempts
│
├── ops/
│   ├── caddy/Caddyfile.snippets/cms-ax-pilots.caddy
│   └── observability/dashboards/cms-cutover-watch.json
│
├── docs/
│   ├── audit/                         Phase 1 SITE1 audit (AUDIT-cms_pages-*.md + baseline)
│   ├── rfc/                            RFC-001 (P1 Strategic)
│   ├── adr/                            ADR-001 (P2 Architectural)
│   ├── plans/                          PLAN-001 (P3 — 16-шаговый порядок)
│   ├── validations/                    VAL-001 (P4 Verification)
│   ├── perf/
│   │   ├── baseline-site1-<date>.txt   oha/wrk output
│   │   └── explain/                   per-query EXPLAIN ANALYZE snapshots
│   ├── interop/
│   │   ├── OWNERSHIP.md                кто canonical для какой таблицы
│   │   ├── CONTRACTS/cms_pages.md      schema + invariants + writer/reader
│   │   ├── EVENTS/cms.page.published.v1.json
│   │   └── ROLLBACK/cms_pages.md      < 5 min recipe
│   ├── releases/v0.1.0/
│   │   ├── ROLLBACK.md
│   │   └── ROLLBACK_DRILL.md           timestamps подтверждающие < 5 min
│   ├── risks/
│   │   └── RISK-bus-factor.md          fixed pre-condition failure
│   └── STATUS.md                       Phase 3 honest assessment
│
├── xtask/                              custom build automation
│   └── src/main.rs                     architecture-check, check-planning-refs (§2.6)
│
└── bridge/                             этот пакет документов (для AI-кодера)
    ├── 01_STRUCTURE.md                этот файл
    ├── 02_LOGIC.md
    ├── 03_TECHNICAL_SPEC.md
    └── 04_DESIGN.md
```

---

## 3. Translation table — SITE1 файл → AX-эквивалент (для cms_pages)

| SITE1 файл | Содержание | AX-эквивалент | AX слой |
|------------|------------|---------------|---------|
| `packages/db/src/schema/cms-pages.ts` | Drizzle schema (jsonb body, indexes) | `migrations/0001_cms_pages_expand.sql` + `crates/domain/src/cms/aggregate.rs` | Schema + Domain |
| `packages/db/src/schema/tenants.ts` | Tenants table | (read-only через views) `crates/infrastructure/src/persistence/views/tenants_views.rs` | Infrastructure |
| `packages/db/src/schema/tenant-design-tokens.ts` | 1:1 design tokens | `crates/infrastructure/src/persistence/views/design_tokens_views.rs` + tenant aggregate | Infrastructure + Domain |
| `apps/api/src/cms/cms.service.ts` (метод `getPublishedBySlug`) | Сервисный метод | `crates/application/src/use_cases/cms/get_published_by_slug.rs` | Application |
| `apps/api/src/cms/cms.controller.ts` (route `public/by-slug/:slug`) | HTTP endpoint | `crates/presentation/src/api/cms_handlers.rs` | Presentation |
| `apps/api/src/cms/dto/blocks.schema.ts` | Zod CmsBlocks discriminated union | `crates/domain/src/cms/value_objects.rs` (enum Block) | Domain |
| `apps/api/src/tenant-context/tenant-resolver.middleware.ts` | NestJS middleware | `crates/presentation/src/middleware/tenant_resolver.rs` | Presentation |
| `apps/api/src/tenant-context/with-tenant.helper.ts` | Drizzle WHERE injection (Layer 2) | `crates/infrastructure/src/persistence/transaction.rs::with_tenant` (но через SET LOCAL для RLS) | Infrastructure |
| `apps/web/src/lib/cms-public.ts::fetchPublicCmsPage` | SSR client | (не нужен — AX сам обслуживает запрос) | — |
| `apps/web/src/components/tenant-site/TenantSiteShell.tsx` | CSS-vars + Google Fonts | `crates/presentation/src/leptos/tenant_shell.rs` | Presentation |
| `apps/web/src/components/cms/ed-editor/EdRenderer.tsx` | Section → HTML | `crates/presentation/src/leptos/cms_page_component.rs` | Presentation |
| `apps/web/src/lib/td-overrides.ts::decodeTdParam` | base64 → tokens | `crates/presentation/src/middleware/td_overrides.rs` (extractor) | Presentation |
| `apps/web/src/app/(tenants)/imperiumspa/[slug]/page.tsx` | Next route | (заменяется Caddy-route на AX `/api/...` + Leptos route) | — |
| `apps/api/src/cms/cms.service.spec.ts` | mock-based isolation specs | `tests/integration/cms_pages_test.rs` (real Postgres) + `tests/fuzz/cms_tenant_isolation.rs` | Tests |

**Принцип отображения:** один SITE1-источник → один AX-целевой файл; никаких «давайте объединим cms + tenants в один модуль для DRY». Бoundary `domain ← application ← infrastructure ← presentation` строже Nest'овской организации модулей.

---

## 4. Что AX **категорически** не дублирует из SITE1

| Поведение SITE1 | Почему не в AX Phase A |
|-----------------|----------------------|
| Write/Update/Delete cms_pages из admin UI | SITE1 — canonical writer (§12.5 Ownership Table); AX в Phase A только читает |
| WP-importer (`wp-import.service.ts`, `wp-sanitize.ts`) | Out of scope — отдельный модуль, не блокирует pilot |
| ED-editor edit-side (`SandboxEditor.tsx`, `EditorHost.tsx`) | UI редактирования остаётся в Next.js админке |
| /admin/* любые роуты | UI staff'а остаётся в SITE1 целиком |
| AdminShell + Rail + RailFooter | UI инфраструктура SITE1 |
| Любые CRUD модули (services, salons, staff, clients) | Не в pilot scope |
| TenantEditFab (плавающая кнопка для админов) | Tied to /admin session; не в AX read-path |
| `?td=` сохранение overrides (write) | Read-only декодер достаточен; запись tokens — через SITE1 admin |
| OpenAPI client generation для /admin | AX будет иметь свой OpenAPI через utoipa, но клиент не нужен (Leptos рендерит сам) |
| Email / SMTP / Telegram bot | Phase 1 functionality — не в Phase 0 pilot |

---

## 5. Конвенции именования (AX → внешний мир)

| Слой SITE1 | Имя в AX | Различия |
|-----------|----------|----------|
| DB column `tenant_id` | Same — но в Rust типе `TenantId(Uuid)` (newtype) | `Uuid` не утекает в публичные сигнатуры |
| `cmsPages.body` (jsonb `CmsBlocks[]`) | `Vec<Block>` в domain, `Json<Vec<Block>>` в SQLx layer | Mapping в `infrastructure/persistence/cms_pages_repo.rs` |
| `cmsPages.status` (`'draft'\|'published'\|'archived'`) | `enum PageStatus { Draft, Published, Archived }` | Serde rename_all = "snake_case" |
| HTTP path `/v1/cms/pages/public/by-slug/:slug` | AX: `/api/v1/cms/pages/public/by-slug/:slug` | `/api/` префикс — Caddy легче маршрутизирует |
| Header `X-Tenant-Slug` | Same — для compat; в идеале subdomain | См. `tenant_resolver.rs` |
| ResponseDto `PageResponseDto` | `CmsPageResponse` (struct serde) | Naming AX-style, но контракт identical |
| Error `NotFoundException({ code: 'PAGE_NOT_FOUND' })` | `AppError::NotFound(...)` → 404 JSON `{ "code": "PAGE_NOT_FOUND" }` | Маппинг в `crates/common/src/error.rs::IntoResponse` |

**Goal contract identity:** клиент Next.js (или Caddy-routed user) не должен заметить разницы между SITE1 и AX response payload. JSON shape **byte-for-byte** идентичен. Поведение 404 идентично. Кэширование заголовков идентично.

---

## 6. Чек-лист для AI-кодера, прежде чем что-то писать

- [ ] Прочитал `barbie/ENTITY.md` целиком
- [ ] Прочитал `barbie/AX/ENTITY.md` целиком (особенно §2, §3, §5, §6, §12.6, §12.7)
- [ ] Открыл и просмотрел все «★»-помеченные SITE1 файлы из §1
- [ ] Понял translation table из §3 — знаешь куда какой кусок поведения переезжает
- [ ] Зафиксировал в `docs/audit/AUDIT-cms_pages-<date>.md`, что именно из SITE1 read-path переносится, что — нет
- [ ] Знаешь, что contract identity (§5) — главный SLA первой фазы
