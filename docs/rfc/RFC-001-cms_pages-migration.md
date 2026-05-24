# RFC-001 · cms_pages read-path migration to AX (Rust)

| Field | Value |
|-------|-------|
| **Status** | Draft — pending sign-off |
| **Date** | 2026-05-24 |
| **Author** | Claude (AI assistant, session-scoped) |
| **Reviewers required** | user (business owner + tech lead in single-person ops) |
| **Phase** | P1 Strategic (`ENTITY.md §2.5`) |
| **Gate inputs** | `docs/audit/AUDIT-cms_pages-2026-05-24.md` (FINAL GREEN) |
| **Gate outputs** | unblocks P2 `ADR-001-four-layer-rls.md`, P3 `PLAN-001-cms_pages-pilot.md`, P4 `VAL-001-cms_pages.md` |
| **Target module** | `cms_pages` read-path · `getPublishedBySlug` only |
| **Migration phase** | A (Pilot, 1 tenant, 2 weeks observation) → B (Stage) → C (GA) → D (Retire) |

---

## Business reason

**Это первая vertical-slice миграция модуля NAS на Rust-стек AX. Решение НЕ про "Rust быстрее Node" в вакууме — оно мотивировано тремя конкретными gain'ами, которые audit подтвердил как делимые ровно на cms_pages read-path:**

### B1 — Tenant isolation upgrade: RLS добавляется впервые

Audit зафиксировал (C1): **`cms_pages` в SITE1 не имеет `ENABLE ROW LEVEL SECURITY`**. Изоляция полностью application-level (TenantGuard + `withTenant`/`combineTenant` + явные `eq(tenantId, ...)`). Если разработчик забыл WHERE — нет ничего на DB-уровне, что заметит.

AX миграция вводит **compile-time + DB-level** защиту одновременно:
- newtype `TenantId(Uuid)` в Rust — функция без `&TenantContext` не компилируется
- POLICY `rls_cms_pages_tenant_isolation` на `cms_pages` — даже если SQL забыл `WHERE tenant_id`, RLS отрежет

**Это hard wall vs SITE1 app-policy.** Read-path — оптимальный pilot: read SQL проще audit'ить, ниже риск регрессий чем write-side.

### B2 — Observability stack installation

Audit зафиксировал (C6): **SITE1 практически не имеет observability** — нет Sentry init (DSN закомментирован в .env.example), нет OpenTelemetry, нет `/metrics`, нет structured JSON logging, нет request-id. Bridge document §10 описывал aspirational план.

AX миграция инсталлирует full stack **впервые**:
- `tracing` + `tracing-subscriber` JSON output
- `tracing-opentelemetry` + OTLP exporter → Tempo / Honeycomb
- `metrics` + Prometheus exporter на `/metrics`
- `sentry-tracing` integration с request-id + tenant_id scope
- ULID `request_id` middleware

**Это inverts the migration framing:** не "повторяем существующее", а "Phase A AX-сервиса = первая инстанция платформы с реальной observability". RFC consciously принимает это repositioning — SITE1 backport observability — **отдельный track**, не блокирующий cms_pages migration.

### B3 — Performance headroom для read-heavy hot paths

Cold-data benchmark (предварительный, requires VAL-001 measurement):

| Metric | SITE1 estimate | AX target | Source |
|--------|----------------|-----------|--------|
| p95 latency (single page) | ~120ms (bridge §10) | ≤ 60ms | AX `ENTITY.md §11` |
| p99 latency | ~250ms estimate | ≤ 100ms | AX `ENTITY.md §11` |
| RSS @ 100 RPS | ~500MB Node | ≤ 200MB | AX `ENTITY.md §11` |
| Cold start | ~3s Node+Next | ≤ 200ms Rust release | AX `ENTITY.md §11.0` |
| Allocations/request | unmeasured | < 500 (dhat) | AX `ENTITY.md §11.5` |

**Disclaimer:** baseline SITE1 numbers — оценки. Audit (C7) зафиксировал: cross-stack alloc/span сравнение невозможно (dhat-rs только в Rust). VAL-001 validates через oha black-box HTTP timing + procfs RSS — единственное apples-to-apples сравнение.

### Why cms_pages specifically (не media, не appointments)

Per `ENTITY.md §12.7 Pilot Selection Matrix`:

- score **8.4** (1st place, vs media 7.8, appointments 6.2)
- **categorical exclusions** для clients (PII), tenants (cascade root), chat (state machine)
- read-side only — risk локализован; write-side остаётся в SITE1 как canonical writer
- `extractEdSections` — единственный реально используемый render path (audit §6.8, C4) — top-level CmsBlocks types почти не встречаются в production content. Это сужает Phase A render scope.

---

## Success criteria (measurable, MUST pass before Phase A → B)

| # | Criterion | Validation | Owner |
|---|-----------|------------|-------|
| S1 | JSON response **byte-for-byte identical** к SITE1 `publicBySlug` (после нормализации `X-Request-Id`, `X-Stack`, timestamps) | `scripts/contract-diff.sh` — diff 100 pilot pages | T19 |
| S2 | **0 cross-tenant leaks** в 1M proptest fuzz (`tests/fuzz/cms_tenant_isolation.rs`) | nightly CI | T15 |
| S3 | **0 RLS bypass** в integration tests — POLICY работает с `SET LOCAL` | `tests/integration/cms_pages_test.rs` | T14 |
| S4 | p95 latency ≤ 60ms под 200 RPS на pilot tenant, 60s warm-up | `oha` measurement, `docs/perf/ax-result.txt` | T16 |
| S5 | Rollback drill **< 5 минут** от detect до Caddy revert | `docs/releases/v0.1.0/ROLLBACK_DRILL.md` timestamps | T17 |
| S6 | `nas_tenant_mismatch_total = 0` за 2 недели Phase A observation | Grafana `cms-cutover-watch` | T20 |
| S7 | `cargo xtask architecture-check` green — 4-layer boundary не нарушен | CI на каждом PR | T05 |
| S8 | `cargo deny check` green — supply-chain hygiene | CI | T05 |

**Gate condition:** все 8 criteria GREEN → DECISION.md GO для Phase B. Любой RED → extend Phase A, не переходить.

---

## Constraints

### C1 — Shared database, expand-only schema

AX и SITE1 ходят в **тот же** Postgres через PgBouncer. Никакой отдельной БД, никакой репликации, никакого CDC.

Schema delta — **expand-only**:
- CREATE VIEW `cms_pages_v_active` (published-only filter) — additive
- ALTER TABLE `cms_pages` ENABLE ROW LEVEL SECURITY + CREATE POLICY — additive (BYPASSRLS на site1_admin_role сохраняет SITE1 behavior)
- CREATE ROLE `site1_admin_role` (BYPASSRLS) + `ax_app_role` (no bypass) — additive
- НЕТ DROP COLUMN, НЕТ ALTER TYPE, НЕТ rename'ов

**N-1 compat verified в gate:** SITE1 на старой схеме читает cms_pages без изменений (см. ADR-001 §Schema delta).

### C2 — PgBouncer в `transaction` mode — load-bearing

RLS через `SET LOCAL app.current_tenant_id` **требует** PgBouncer в `transaction` pool mode (`ENTITY.md §3`):
- `session` mode → SET LOCAL держится между запросами одного коннекта → pool exhaustion под нагрузкой
- `statement` mode → SET LOCAL вообще не работает (no transaction scope)

**Pre-condition:** AX deploy на VPS требует PgBouncer config audit. **Не deploy в prod без подтверждения transaction mode.**

### C3 — Contract identity SLA (JSON, Phase A)

JSON response должен быть **byte-for-byte identical** к SITE1 — поле order, null vs missing, ISO8601 формат timestamps.

Audit §4.2 зафиксировал architectural constants:
- **`tenantId` НЕ в response** (privacy / unchanged SITE1 behavior)
- `body: unknown[]` — raw blocks array
- Optional поля сериализуются как `null`, не пропускаются (`metaTitle`, `metaDescription`, `coverImageKey`, `authorUserId`, `publishedAt`)
- `publishedAt`: ISO8601 RFC3339 с Z-suffix (`2026-05-24T10:30:00.000Z`)
- Поле order: `id, slug, locale, title, body, status, metaTitle, metaDescription, coverImageKey, authorUserId, publishedAt, createdAt, updatedAt`
- 404 body: `{ code: 'PAGE_NOT_FOUND', slug, locale }` byte-for-byte

### C4 — Visual identity SLA (HTML, Phase B) — НЕ обещается полностью

Audit зафиксировал (C2, C3, C5):

- **Widget colors hardcoded в SITE1** (`#00FFCC` cyan кнопки, `#F2EBD9` icon-box, etc.) игнорируют tenant CSS-vars
- **Per-tenant cms-route хардкодит** `background: '#0E0F12'` независимо от `tenant_design_tokens.bg`
- **`TenantSiteShell` НЕ используется в `[slug]` route** — нет CSS-vars, нет Google Fonts, нет `<style>` block

**RFC явно зафиксирует deliberate deviation:**

AX Phase B рендерит полноценный shell (CSS-vars + Google Fonts + style block) **через design tokens**. Это означает:
- `pentagon` тенант рендерит на тёмном фоне (его `bg=#0A0A0C`), не на `#0E0F12`
- `barbiespa` рендерит на pink (`bg=#FFB6D9`), не на `#0E0F12`
- кнопки наследуют `--acc-color` тенанта, не hardcoded cyan

**Это визуально другая страница.** Acceptance: фиксируется в `docs/releases/v0.1.0/DECISION.md` как deliberate upgrade. RFC sign-off acknowledges.

**Alternative considered (rejected):** replicate broken behavior pixel-for-pixel. Rejected because:
1. SITE1 baseline outright игнорирует tenant'овский bg — это **bug в SITE1**, не feature
2. Tenant'ы платят за design tokens; обещание identity = обещание сохранить bug
3. Цена альтернативы: maintain hardcoded color map в AX Leptos рендере **навсегда**; deviation от ENTITY.md §3 "compile-time enforcement многих принципов"

### C5 — Read-only Phase A

AX в Phase A обслуживает **только** `GET /api/v1/cms/pages/public/by-slug/:slug?locale=ru`. Никаких write endpoints:
- POST/PATCH/DELETE/publish/unpublish/archive — **остаются в SITE1**
- SITE1 = **canonical writer** в `docs/interop/OWNERSHIP.md`
- Если AX обнаружит write request — 405 METHOD_NOT_ALLOWED, не proxy в SITE1 (избегаем хождения по кругу)

### C6 — Single-tenant pilot scope

Phase A = **1 pilot tenant** (выбор tenant'а — в P3 PLAN-001 §Caddy snippet, default `imperiumspa`). Phase B = 3 contrasting tenants (`pentagon`, `barbiespa`, `roxy-spa`). Phase C = full whitelist.

Никаких parallel module migrations (per `ENTITY.md §12.6.8 anti-pattern №1`). `media` migration — отдельный RFC после cms_pages GA.

### C7 — Search engine cloaking ОТКЛЮЧЁН в Phase A

ENTITY.md §25 specs Conditional Content Delivery — universal request-aware variant primitive. Для pilot tenant'а **CCD не активируется**. Если бы вариант с `classification=cloaking` присутствовал — это меняло бы response shape и нарушало бы S1 contract identity.

Pilot tenant config обязан иметь:
- 0 active `tenant_content_variants` rows
- `tenant_settings.ccd_cloaking_opt_in = false` (default)

CCD интеграция — отдельный RFC в Phase B+.

### C8 — Analytics tags и SEO control — passthrough в Phase A

ENTITY.md §26 (analytics inventory) и §27 (SEO control) — **specced**, но **не имплементированы** в Phase A. AX рендерит JSON response без `<head>` (Phase A = JSON only); HTML rendering начинается в Phase B (Leptos SSR) — там и должны заходить:
- analytics tag injection (§26)
- SEO meta tags + canonical + hreflang (§27)

Эти features ставятся отдельным track'ом, не блокируя cms_pages migration.

---

## Out of scope (explicit exclusions — каждый — отдельный RFC если понадобится)

| Out of scope | Reason |
|--------------|--------|
| Write endpoints (create/update/publish/unpublish/archive) | SITE1 canonical writer; Phase B+ AX рассмотрит |
| Admin UI / `/admin/*` routes | UI staff'а остаётся в Next.js целиком |
| Custom domain resolution (`tenant.customDomain` → routing) | SITE1 не реализовал в коде (только DB column); AX H1 finding — фикс отложен |
| HTML sanitization (text.html через ammonia/DOMPurify) | SITE1 не делает; AX H2 — defence in depth, не блокирующее, Phase B optional |
| Listing pagination keyset upgrade | listPages — admin endpoint; не в Phase A scope |
| Top-level CmsBlocks rendering (hero, text, gallery, services, cta) | Audit C4: почти не используются в production (90%+ страниц = `custom + data.ed`); Phase B fallback |
| Per-tenant 11 Next.js routes (`/(tenants)/<slug>/[slug]/page.tsx`) | Остаются в SITE1; AX обслуживает endpoint, Next.js hit'ает по-старому |
| Caddy routing setup в SITE1 (modify existing Caddyfile) | Только AX добавляет свой snippet; SITE1 Caddy config не трогается |
| Observability backport в SITE1 | Отдельный track |
| §25 CCD / §26 Analytics / §27 SEO infrastructure | Specced в ENTITY.md v3.4 как post-pilot features; SITE1 first imp |
| Schema deltas помимо expand (DROP, ALTER TYPE) | Forward-only migration discipline |
| TenantSettings table changes | jsonb column в `tenants`, не трогается |
| Cross-tenant analytics queries | Out of pilot; Phase B+ если понадобится |
| Auth (JWT) AX-side implementation | AX endpoint = @Public() в Phase A; JWT verification — Phase B если admin endpoints добавятся |
| Performance baseline для SITE1 в код-side detail | C7: not feasible cross-stack; black-box HTTP timing достаточен |
| Backfill RLS на другие таблицы | Только `cms_pages` в Phase A migration scope |

---

## Risks & mitigations

### R1 — PgBouncer не в transaction mode на pilot VPS

**Severity:** Critical. **Probability:** Medium (current VPS config unknown).

**Mitigation:** перед deploy на VPS — audit `pgbouncer.ini` `pool_mode = transaction`. Если session mode — change config, rolling reload PgBouncer, verify (`SHOW POOLS;` should show transaction). Документация: `docs/releases/v0.1.0/PRE-DEPLOY-CHECKLIST.md`.

### R2 — SITE1 canonical writer ломает schema invariants

**Scenario:** SITE1 admin создаёт `cms_page` где `status='published'` но `published_at IS NULL`. AX view filter (`WHERE status='published'`) пропускает row, но `publishedAt` сериализуется как `null` → JSON byte-diff vs SITE1.

**Mitigation:** Phase A integration test verifies invariant `status='published' ⇔ published_at IS NOT NULL`. Если SITE1 row нарушает invariant — лог warning, продолжаем рендер. Phase B AX expand-migration **MAY** добавить DB CHECK constraint (после consensus с SITE1).

### R3 — Hardcoded background `#0E0F12` ломает contract identity

**Scenario:** pilot tenant имеет `tenant_design_tokens.bg='#FFB6D9'` (pink). SITE1 рендерит `#0E0F12` (hardcoded). AX (Phase B) рендерит `#FFB6D9`. HTML diff не "byte-for-byte" — это **визуально другой сайт**.

**Mitigation:** C4 deliberate deviation в DECISION.md. RFC sign-off acknowledges. Если pilot tenant возражает — fallback в `tenant_settings.ax_visual_legacy_bg = true` (hardcoded `#0E0F12`) с миграционным таймером (revoke в Phase C).

### R4 — Caddy routing race (request landing на старом SITE1 во время Phase A switch)

**Scenario:** Caddy reload занимает ~200ms; запросы в этом окне попадают на старый upstream.

**Mitigation:** Caddy `graceful reload` (SIGUSR1) — drain'ит in-flight requests; новый upstream — только для new connections. Verify Phase A через `oha` 100 RPS continuous во время reload — 0 5xx errors expected.

### R5 — RLS POLICY ломает SITE1 read-paths

**Scenario:** SITE1 admin endpoint читает `cms_pages` через `site1_admin_role` который должен BYPASSRLS. Если roles configuration криво — SITE1 admin падает.

**Mitigation:** Pre-deploy integration test от **SITE1 роли** — `SELECT 1 FROM cms_pages LIMIT 1` должен return row без `SET LOCAL`. Если падает — POLICY misconfigured.

### R6 — Sentry init fails в production → ослепляем себя

**Scenario:** `SENTRY_DSN` invalid в prod env, Sentry SDK silently fails. Crash в AX не captured.

**Mitigation:** AX startup validates Sentry DSN (test-event на boot, fail-fast если DSN invalid AND `env != development`). Per ENTITY.md §18.6 Secret handling.

### R7 — moka cache stale на `slug → TenantId`

**Scenario:** Tenant slug меняется в SITE1 admin (rare, но legitimate). AX cache 5min TTL может рендерить контент для wrong tenant.

**Mitigation:** TTL 5min = acceptable. **MAY** добавить `pgmq` event `tenant.slug.changed` → AX invalidates cache (Phase B). Phase A — принимаем eventual consistency.

---

## Sign-off

**Required before P2 (ADR-001) start:**
- [ ] User reviews this RFC, all Constraints (C1-C8) acknowledged
- [ ] C4 visual deviation (hardcoded bg fix) — explicit GO/NO-GO
- [ ] Pilot tenant selection — default `imperiumspa`, alternative confirmed if different
- [ ] PgBouncer mode audit scheduled (R1 mitigation)

**Sign-off record:**

| Reviewer | Date | Decision | Notes |
|----------|------|----------|-------|
| _user_ | _pending_ | _pending_ | — |

---

## Related artifacts

- **Audit:** `docs/audit/AUDIT-cms_pages-2026-05-24.md` (FINAL GREEN — input gate)
- **ADR (P2):** `docs/adr/ADR-001-four-layer-rls.md` — unblocked by this RFC sign-off
- **Implementation Plan (P3):** `docs/plans/PLAN-001-cms_pages-pilot.md` — to be written after ADR-001
- **Validation Spec (P4):** `docs/validations/VAL-001-cms_pages.md` — to be written before Phase 4 (T14)
- **ENTITY constitution:** `barbie/ax/ENTITY.md` v3.4 (§2.5 RFC pipeline, §3 multi-tenancy, §11 perf budget, §12.6 migration playbook, §12.7 pilot selection)
- **Bridge context:** `barbie/ax/bridge/01-04` (SITE1↔AX mapping reference, не источник правды)
- **Parent constitution:** `barbie/ENTITY.md` (workspace canonical)

---

*RFC complete. Awaiting user sign-off to unblock ADR-001.*
