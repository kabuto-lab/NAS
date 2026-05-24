# VAL-001 · cms_pages pilot — Validation Spec

| Field | Value |
|-------|-------|
| **Status** | Draft (AVTONOM-generated 2026-05-24 19:59) |
| **Phase** | P4 Verification (`ENTITY.md §2.5`) |
| **Dependent on** | RFC-001 + ADR-001 + PLAN-001 sign-off |
| **Required before** | M-A gate (Phase A pilot exit) |

---

## Success criteria (measurable)

### Functional correctness

| # | Criterion | Threshold | Validation method | Owner |
|---|-----------|-----------|-------------------|-------|
| F1 | JSON response shape | byte-for-byte identical к SITE1 после нормализации `X-Request-Id`, `X-Stack`, и timestamps | `scripts/contract-diff.sh` — 100 random pilot pages | T19 |
| F2 | Field order preserved | `id, slug, locale, title, body, status, metaTitle, metaDescription, coverImageKey, authorUserId, publishedAt, createdAt, updatedAt` | golden file test `tests/integration/json_order_test.rs` | T14 |
| F3 | `tenantId` NOT in response | absent | golden file test | T14 |
| F4 | Optional fields serialize as `null` (not skipped) | `metaTitle`, `metaDescription`, `coverImageKey`, `authorUserId`, `publishedAt` always present | golden file test | T14 |
| F5 | 404 body shape | `{ code: 'PAGE_NOT_FOUND', slug, locale }` byte-for-byte | integration test | T14 |
| F6 | `locale` default fallback | unspecified → `'ru'` | integration test (query без `?locale=`) | T14 |
| F7 | Page slug regex | matches `^[a-z0-9](?:[a-z0-9/-]{1,78}[a-z0-9])?$` (3-80 chars, allows slash) | unit test в `crates/domain/src/cms/value_objects.rs` | T08 |
| F8 | Tenant slug regex | matches `^[a-z0-9][a-z0-9-]{1,62}[a-z0-9]$` (3-64 chars, **исправляет SITE1 H1 bug** до 64 chars) | unit test в tenant resolver | T12 |
| F9 | locale enum | `'ru' \| 'en'` exhaustive | unit test | T08 |
| F10 | status enum | `'draft' \| 'published' \| 'archived'` (read returns only published) | integration test | T14 |
| F11 | ISO8601 RFC3339 format с Z-suffix | `2026-05-24T19:59:00.000Z` style | golden file test | T14 |

### Tenant isolation (security)

| # | Criterion | Threshold | Validation method | Owner |
|---|-----------|-----------|-------------------|-------|
| I1 | RLS POLICY active | `cms_pages` has POLICY `rls_cms_pages_tenant_isolation` enabled | post-migration verify SQL: `SELECT polname FROM pg_policy WHERE polrelid = 'cms_pages'::regclass;` | T10 |
| I2 | `ax_app_role` HAS NO BYPASSRLS | `rolbypassrls = false` | post-migration: `SELECT rolname, rolbypassrls FROM pg_roles WHERE rolname='ax_app_role';` | T10 |
| I3 | `site1_admin_role` HAS BYPASSRLS | `rolbypassrls = true` (backward compat preserve) | same query | T10 |
| I4 | Cross-tenant read returns NotFound | tenant_b request with slug from tenant_a → 404 NOT_FOUND | integration test `cross_tenant_returns_not_found` | T14 |
| I5 | RLS blocks bypass-attempt | даже без `WHERE tenant_id` в query — POLICY режет | integration test `rls_blocks_missing_tenant_filter` | T14 |
| I6 | Suspended tenant 403 | `tenant.status='suspended'` → 403 TENANT_NOT_ACTIVE на public endpoint | integration test | T14 |
| I7 | Unresolved tenant 401 | request без X-Tenant-Slug + без subdomain → 401 TENANT_NOT_RESOLVED | integration test | T12 |
| I8 | Fuzz isolation 1M | 1M random cross-tenant attempts, 0 leaks | proptest nightly CI | T15 |
| I9 | Sentry TenantMismatch alert | any TenantMismatch event → high severity Sentry alert | manual trigger + verify Sentry receives event | T20 |
| I10 | `nas_tenant_mismatch_total = 0` | gauge metric stays at 0 for 2 weeks pilot | Grafana panel + alert | T20 |

### Performance

| # | Criterion | Threshold | Validation method | Owner |
|---|-----------|-----------|-------------------|-------|
| P1 | p50 latency | ≤ 20ms | `oha -n 10000 -c 100 -z 60s` against `/api/v1/cms/pages/public/by-slug/about` | T16 |
| P2 | p95 latency | ≤ 60ms | same | T16 |
| P3 | p99 latency | ≤ 100ms | same | T16 |
| P4 | Throughput | ≥ 5000 req/s на core | `oha -c 200` + measure | T16 |
| P5 | RSS @ 100 RPS | ≤ 200MB | `procfs` polling 5s interval during 60s benchmark | T16 |
| P6 | Cold start | ≤ 200ms | `time ./target/release/ax-server` до первого `READY` log | T13 |
| P7 | DB acquire p95 | ≤ 5ms | `tracing` span `db.acquire` percentile | T13 |
| P8 | EXPLAIN cost regression | ≤ 30% increase vs baseline | `cargo xtask query-budget` nightly | T11 |
| P9 | Query budget | ≤ 5 joins per query (cms_pages get use case = 0 joins) | xtask AST scan | T05 |
| P10 | No `SELECT *` | absent | xtask regex check | T05 |
| P11 | No `OFFSET` pagination | absent (cms_pages get не использует pagination) | xtask | T05 |
| P12 | Sequential scan на > 1000 rows | absent | EXPLAIN ANALYZE snapshot | T16 |
| P13 | Response payload | ≤ 1 MB | `tower-http::limit::ResponseBodyLimitLayer` | T12 |

### Allocation budget

| # | Criterion | Threshold | Validation method | Owner |
|---|-----------|-----------|-------------------|-------|
| A1 | Heap allocations per CRUD read | < 500 | `dhat-rs` bench + baseline file | T16 (bench) |
| A2 | Total bytes per request (read) | < 64 KB | dhat bench | T16 |
| A3 | Async tasks per request | < 20 | tracing instrument count | T16 |
| A4 | Large allocs (> 16 KB) | traced + tagged | dhat | T16 |
| A5 | `TenantContext.clone()` calls | 0 — TenantContext is Copy | clippy custom lint | T05 |
| A6 | Regression vs baseline | ≤ 20% deviation | `cargo xtask alloc-budget` nightly | T05 |

### Code quality

| # | Criterion | Threshold | Validation method | Owner |
|---|-----------|-----------|-------------------|-------|
| Q1 | `cargo build --workspace --release` | green | CI build | T04 |
| Q2 | `cargo clippy --workspace --all-targets -- -D warnings` | green | CI lint | T05 |
| Q3 | `cargo fmt --check` | green | CI format | T05 |
| Q4 | `cargo test --workspace` | green | CI test | T14 |
| Q5 | `cargo sqlx prepare --workspace --check` | green (offline meta committed) | CI | T11 |
| Q6 | `cargo deny check` | green | CI | T06 |
| Q7 | `cargo udeps --workspace` | 0 unused deps | nightly CI | T06 |
| Q8 | `cargo audit` | 0 vulnerabilities | nightly CI + before each release | T06 |
| Q9 | `cargo xtask architecture-check` | 4-layer boundary не нарушен | CI | T05 |
| Q10 | `cargo xtask magic-check` | no forbidden patterns | CI | T05 |
| Q11 | `cargo xtask check-planning-refs` | PR содержит RFC/ADR/PLAN/VAL refs | CI per ENTITY §2.5 | T05 |
| Q12 | Function length | < 80 LOC | clippy `too_many_lines` | T05 |
| Q13 | Cyclomatic complexity | < 15 | clippy `cognitive_complexity` | T05 |

### Operational readiness

| # | Criterion | Threshold | Validation method | Owner |
|---|-----------|-----------|-------------------|-------|
| O1 | Health endpoint | `GET /health` returns 200 with `{ok, db, uptime, version, git_sha}` | curl | T13 |
| O2 | Readiness endpoint | `GET /health/ready` returns 200 if DB+S3 ok, 503 otherwise | curl | T13 |
| O3 | Graceful shutdown | SIGTERM → drain ≤ 30s, no dropped requests | integration test | T13 |
| O4 | Sentry capture works | trigger 500 → Sentry receives event with tenant_id+request_id tags | manual test | T20 |
| O5 | OTLP export | spans arrive в Tempo/Honeycomb (dev может быть Jaeger) | manual verify | T20 |
| O6 | Prometheus `/metrics` | exposition format valid; `nas_*` metrics present | `curl /metrics` + parse | T20 |
| O7 | JSON log output | stdout = valid JSON, one event per line | `cat log \| jq .` | T13 |
| O8 | Request ID propagation | `X-Request-Id` in response = ULID; tracing span uses same | curl + check headers | T12 |
| O9 | Caddy reverse-proxy reload | graceful, 0 5xx during reload | drill in T17 | T17 |
| O10 | Rollback < 5 min | drill timestamps | T17 ROLLBACK_DRILL.md | T17 |
| O11 | PgBouncer transaction mode | `SHOW POOLS` confirms `pool_mode = transaction` | manual pre-deploy | T18 |
| O12 | `site1_admin_role` configured в SITE1 PgBouncer | confirmed | manual pre-deploy | T18 |

---

## Tests required (checklist with file paths)

### Unit tests

- [ ] **`crates/common/src/tenant.rs::tests`** — TenantId roundtrip, TenantContext Copy semantics
- [ ] **`crates/common/src/error.rs::tests`** — AppError → HTTP status code mapping (всех variants)
- [ ] **`crates/common/src/ids.rs::tests`** — UserId, RequestId Ulid construction
- [ ] **`crates/domain/src/cms/value_objects.rs::tests`**:
  - PageSlug accepts valid (`'about'`, `'services/spa'`, `'a'.repeat(80)`)
  - PageSlug rejects invalid (`'/'`, `''`, `'A'`, `'a'.repeat(81)`, `'-bad'`)
  - PageLocale parses `'ru'`/`'en'`, rejects others
  - PageStatus enum variants
- [ ] **`crates/domain/src/cms/blocks.rs::tests`**:
  - Block enum serde roundtrip — `hero`, `text`, `gallery`, `services`, `cta`, `custom`
  - Widget enum serde — все 8 types incl. `icon-box` (тип с дефисом!)
  - Section/Column/CanvasElement roundtrip
  - ElStyle defaults
- [ ] **`crates/domain/src/cms/aggregate.rs::tests`**:
  - PublishedPage invariants — status='Published' ⇒ publishedAt Some
  - reconstitute() rejects invalid combinations
- [ ] **`crates/application/src/use_cases/cms/get_published_by_slug.rs::tests`** — mock repo, delegates correctly

### Integration tests (testcontainers + real Postgres)

- [ ] **`tests/integration/cms_pages_test.rs`** (T14):
  - `published_page_returns_200_for_owning_tenant` (F1, I4)
  - `cross_tenant_returns_not_found` (I4, F5)
  - `draft_status_returns_not_found` (F10)
  - `archived_status_returns_not_found` (F10)
  - `locale_default_ru_fallback` (F6)
  - `invalid_locale_returns_404` (audit behavior)
  - `suspended_tenant_returns_403` (I6)
  - `rls_blocks_missing_tenant_filter` (I5) — query без `SET LOCAL`, expect 0 rows
  - `site1_admin_role_bypass_works` (I3)
- [ ] **`tests/integration/json_order_test.rs`** (F2, F3, F4, F11):
  - Golden file fixture: `tests/fixtures/cms_page_response.golden.json`
  - Compare serialization output ↔ golden file byte-for-byte
- [ ] **`tests/integration/test_page_design_variance.rs`** (T23, Phase B):
  - 3 tenants (pentagon, barbiespa, roxy-spa) — same content, different tokens
  - HTML diff ограничен CSS-vars + Google Fonts URL

### Fuzz tests

- [ ] **`tests/fuzz/cms_tenant_isolation.rs`** (T15, I8):
  - proptest 1M iterations (full nightly), 10k smoke (regular CI)
  - random attacker_tenant ≠ victim_tenant, random slug
  - assertion: AppError::NotFound OR absent — никогда не utечка victim_tenant's data

### E2E / smoke

- [ ] **`tests/e2e/health_smoke.rs`** — `cargo run` + curl /health → 200 (O1)
- [ ] **`tests/e2e/graceful_shutdown.rs`** — SIGTERM + drain ≤ 30s (O3)

### Performance benchmarks

- [ ] **`benches/cms_pages_get_published.rs`** (criterion) — local timing
- [ ] **External oha runs** — documented в `docs/perf/` (T16)

### Static checks (xtask)

- [ ] `cargo xtask architecture-check` (Q9) — boundary verification
- [ ] `cargo xtask magic-check` (Q10) — NO MAGIC compliance
- [ ] `cargo xtask check-planning-refs` (Q11) — PR has RFC/ADR/PLAN/VAL links
- [ ] `cargo xtask alloc-budget` (A6) — dhat snapshot regression
- [ ] `cargo xtask query-budget` (P8) — EXPLAIN regression

---

## Observability hooks

### Tracing spans (mandatory)

```rust
#[tracing::instrument(
    skip(state),
    fields(
        tenant_id = %ctx.tenant_id.0,
        tenant_slug = %ctx.tenant_slug,
        request_id = %ctx.request_id,
        cms.page.slug = %slug.as_str(),
        cms.page.locale = %locale.as_str(),
    ),
)]
async fn get_published_by_slug(...) -> Result<Json<CmsPageResponse>, AppError>
```

Span attributes verified в T20 dashboard creation.

### Metrics (Prometheus exposition)

```
# HELP nas_request_duration_seconds Latency histogram per route and status_class
# TYPE nas_request_duration_seconds histogram
nas_request_duration_seconds{stack="ax", route="/api/v1/cms/pages/public/by-slug/:slug", status_class="2xx"} ...

nas_request_total{stack="ax", route, status_class} counter
nas_alloc_bytes_per_request{stack="ax", route} histogram
nas_db_pool_active{stack="ax"} gauge
nas_tenant_mismatch_total{stack="ax"} counter    -- ALERT > 0
nas_cms_page_not_found_total{stack="ax", reason="cross_tenant|draft|missing"} counter
```

### Sentry tags

- `stack: ax`
- `migration_phase: pilot|stage|ga|retire`
- `tenant_id: <uuid>` (per request scope)
- `tenant_slug: <string>`
- `request_id: <ulid>`
- `user_id: <uuid>` (when known)

### Alert rules

| Alert | Condition | Severity | Page |
|-------|-----------|----------|------|
| `nas_tenant_mismatch_total > 0` | any cross-tenant attempt | High | On-call |
| `p95_latency_ms{stack="ax"} > 100` for 5m | perf regression | High | On-call |
| `error_rate{stack="ax", status_class="5xx"} > 1%` | service degraded | Critical | Auto-revert Caddy |
| `nas_alloc_bytes_per_request > baseline × 1.2` | alloc regression | Low | Issue auto-assigned |
| `cms-cutover-watch dashboard panel red` | composite | High | On-call review |

### Dashboards

- **`cms-cutover-watch`** (`ops/observability/dashboards/cms-cutover-watch.json`) — Phase A live observation
  - Side-by-side SITE1 vs AX: p50/p95/p99, error rate, RPS, RSS, alloc/req
  - Tenant mismatch counter
  - Pilot tenant traffic share

---

## Gate sign-off

**Required для Phase 4 → Phase A:**

- [ ] All F1-F11 criteria green
- [ ] All I1-I10 criteria green (RLS POLICY active + 1M fuzz green)
- [ ] All P1-P13 criteria measured (P5-P6 may have estimates; firm в Phase A)
- [ ] All A1-A6 criteria measured (baseline in `docs/perf/`)
- [ ] All Q1-Q13 CI gates green
- [ ] All O1-O12 ops checks green
- [ ] T16 oha baseline captured (SITE1 + AX)
- [ ] T17 rollback drill < 5 min documented

**Sign-off record:**

| Reviewer | Date | Decision | Notes |
|----------|------|----------|-------|
| _user_ | _pending_ | _pending_ | — |

---

## Related artifacts

- **Audit:** `docs/audit/AUDIT-cms_pages-2026-05-24.md`
- **RFC:** `docs/rfc/RFC-001-cms_pages-migration.md`
- **ADR:** `docs/adr/ADR-001-four-layer-rls.md`
- **PLAN:** `docs/plans/PLAN-001-cms_pages-pilot.md`
- **ENTITY:** `barbie/ax/ENTITY.md` v3.4 (§2.5 P4 template, §11 perf budget, §11.5 alloc budget, §11.6 query budget, §17 tests)

---

*Validation spec complete. Awaiting RFC + ADR + PLAN + this VAL sign-off to unblock Step 4 (workspace bootstrap).*
