# PLAN-001 · cms_pages pilot — Implementation Plan

| Field | Value |
|-------|-------|
| **Status** | Draft (AVTONOM-generated 2026-05-24 19:59) |
| **Phase** | P3 Execution (`ENTITY.md §2.5`) |
| **Dependent on** | RFC-001 + ADR-001 sign-off |
| **Unblocks** | T07+ execution (crates/common onwards) + VAL-001 sign-off |
| **Cadence** | Atomic commits per step; spine markers per CLAUDE.md §M |

---

## Глоссарий

- **spine** = файлы, изменение которых требует explicit user-ok в любом режиме per `CLAUDE.md §M`
- **non-spine** = controllers/services/components/utils/tests/configs — могут быть изменены в SEMIAUTO/AVTONOM без paused остановок
- **Atomic commit-cell** = шаг, который commit'ится одним PR (или одним коммитом локально); содержит self-contained delta + tests + doc update

---

## Phase 1 — Audit & Planning (W1-W2)

### Step 1 · SITE1 audit cms_pages e2e

| Field | Value |
|-------|-------|
| **Status** | ✅ DONE (95% coverage) |
| **Artifact** | `barbie/ax/docs/audit/AUDIT-cms_pages-2026-05-24.md` |
| **Files touched** | `docs/audit/*.md` (non-spine) |
| **Duration actual** | 1 session (read-only) |
| **Gate** | FINAL GREEN |

### Step 2 · RFC-001 (P1 Strategic)

| Field | Value |
|-------|-------|
| **Status** | ✅ DONE (Draft pending user sign-off) |
| **Artifact** | `barbie/ax/docs/rfc/RFC-001-cms_pages-migration.md` |
| **Files touched** | `docs/rfc/*.md` (non-spine) |
| **Gate** | RFC sign-off blocks ADR finalization |

### Step 3 · ADR-001 (P2 Architectural)

| Field | Value |
|-------|-------|
| **Status** | ✅ DONE (Draft pending user sign-off) |
| **Artifact** | `barbie/ax/docs/adr/ADR-001-four-layer-rls.md` |
| **Files touched** | `docs/adr/*.md` (non-spine) |
| **Gate** | ADR sign-off blocks bootstrap |

### Step 3.5 · PLAN-001 (this document) + VAL-001 (Validation Spec)

| Field | Value |
|-------|-------|
| **Status** | 🟡 IN PROGRESS (PLAN this file, VAL-001 separate) |
| **Artifact** | `docs/plans/PLAN-001-cms_pages-pilot.md` + `docs/validations/VAL-001-cms_pages.md` |
| **Files touched** | `docs/plans/*.md`, `docs/validations/*.md` (non-spine) |
| **Gate** | P4 sign-off blocks crates/ touching |

---

## Phase 2 — Bootstrap (W3)

### Step 4 · Cargo workspace + manifests

| Field | Value |
|-------|-------|
| **Touched files (all non-spine)** | `Cargo.toml` (workspace root), `crates/{common,domain,application,infrastructure,presentation,runtime}/Cargo.toml`, `apps/server/Cargo.toml`, `xtask/Cargo.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml` |
| **Specs** | per ADR-001 D10 dependency choices + ENTITY.md §4 stack |
| **Verify** | `cargo build --workspace` green (даже с empty src/lib.rs stubs) |
| **Depends on** | ADR-001 sign-off |
| **Blocks** | Steps 7-13 |

### Step 5 · xtask crate — fitness functions

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `xtask/src/main.rs`, `xtask/src/architecture_check.rs`, `xtask/src/magic_check.rs`, `xtask/src/check_planning_refs.rs`, `xtask/src/alloc_budget.rs` (stub), `xtask/src/query_budget.rs` (stub) |
| **Specs** | per ENTITY.md §2.6 ban-list, §2.7 NO MAGIC, §2.5 RFC/ADR refs |
| **Verify** | `cargo xtask architecture-check` runs (хоть пустым результатом) |
| **Depends on** | Step 4 |

### Step 6 · cargo-deny + CI pipeline

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `deny.toml`, `.github/workflows/ci.yml` |
| **Specs** | ADR-001 D10 license allowlist + ban list; CI = fmt + clippy + build + test + deny + xtask + udeps (nightly) + audit (nightly) |
| **Verify** | CI workflow runs локально через `act` или GitHub Actions при push |
| **Depends on** | Step 4 |

---

## Phase 3 — Implementation (W4-W7, inside-out)

### Step 7 · crates/common — TenantId, AppError, IDs, Page

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `crates/common/src/{lib,tenant,ids,error,page}.rs` |
| **Specs** | per ADR-001 D6 (TenantContext shape), D7 (AppError enum + HTTP mapping) |
| **Verify** | `cargo build -p ax-common --release` green + unit tests `cargo test -p ax-common` |
| **Depends on** | Step 4 |
| **Blocks** | Steps 8, 9, 11, 12 |

### Step 8 · crates/domain — PublishedPage, Block, value objects

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `crates/domain/src/cms/{aggregate,value_objects,blocks}.rs`, `crates/domain/src/cms/mod.rs`, `crates/domain/src/lib.rs` |
| **Specs** | per audit §1.5 schema + ADR-001 D4 (enum + serde flat mapping) + §6.4 ED widget types |
| **Critical:** | invariants — `status='Published' ⇒ publishedAt: Some`, slug regex `^[a-z0-9](?:[a-z0-9/-]{1,78}[a-z0-9])?$` (page slug 3-80 chars, **allows slash**) |
| **Verify** | `cargo build -p ax-domain` green + unit tests с garde validate examples + serde round-trip tests с golden fixtures |
| **Depends on** | Step 7 |
| **Blocks** | Steps 9, 11 |

### Step 9 · crates/application — port trait + use case

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `crates/application/src/ports/cms.rs`, `crates/application/src/use_cases/cms/get_published_by_slug.rs`, `crates/application/src/lib.rs` |
| **Specs** | per ADR-001 D2 (CmsRepository signature) + Audit §2.2 invariants |
| **Trait signature** | `find_published_by_slug(&self, ctx: &TenantContext, slug: &PageSlug, locale: PageLocale) -> Result<PublishedPage, AppError>` |
| **Verify** | `cargo build -p ax-application` green + unit test с mock trait impl |
| **Depends on** | Steps 7, 8 |
| **Blocks** | Steps 11, 12 |

### Step 10 · Migration 0001_cms_pages_expand.sql + RLS + roles

| Field | Value |
|-------|-------|
| **Touched files** | `migrations/0001_cms_pages_expand.sql` (non-spine — new) |
| **Specs** | per ADR-001 D2 exact SQL |
| **CAUTION** | Applied migration → production schema change. **SPINE-TOUCH** — требует explicit user-ok before applying. **File creation = non-spine**; **applying via `psql` against prod = spine-action.** |
| **Verify (in dev)** | Apply against local Postgres; SITE1 SELECT cms_pages still works (BYPASSRLS preserved) |
| **Depends on** | ADR-001 sign-off |
| **Blocks** | Steps 11, 14 |

### Step 11 · crates/infrastructure — PgCmsRepository + with_tenant

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `crates/infrastructure/src/persistence/{pool,transaction,cms_pages_repo}.rs`, `crates/infrastructure/src/lib.rs` |
| **Specs** | per ADR-001 D3 (with_tenant helper SET LOCAL pattern) |
| **CRITICAL** | `cargo sqlx prepare --workspace --check` requires real Postgres connection to generate `.sqlx/` offline metadata. **First-run requires local DB up.** |
| **Verify** | `cargo build -p ax-infrastructure` green + `cargo sqlx prepare` runs successfully + integration test against testcontainers |
| **Depends on** | Steps 9, 10 |
| **Blocks** | Steps 12, 13, 14 |

### Step 12 · crates/presentation — Axum handler + middleware + router

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `crates/presentation/src/{lib,app_state}.rs`, `crates/presentation/src/middleware/{tenant_resolver,request_id,error_to_response,td_overrides}.rs`, `crates/presentation/src/api/{cms_handlers,version_handler}.rs` |
| **Specs** | per ADR-001 D6 (resolution priority), D7 (error mapping), Audit §3 controller surface |
| **Caveats** | feature-gated `leptos-ssr` for Phase B render code; Phase A — JSON only |
| **Verify** | `cargo build -p ax-presentation` green + `Router::oneshot` unit test для каждого endpoint |
| **Depends on** | Step 11 |
| **Blocks** | Step 13 |

### Step 13 · apps/server — bootstrap + DI + tracing init

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `apps/server/src/{main,router}.rs` |
| **Specs** | per ADR-001 D8 (observability stack init) + ENTITY.md §16 локальный dev + §21 release engineering |
| **Verify** | `cargo run --bin ax-server` + `curl http://localhost:7000/health` → `{ok: true, ...}` |
| **Depends on** | Step 12 |
| **Blocks** | Step 14 (integration testing requires running server OR via Router::oneshot — second preferred) |

---

## Phase 4 — Validation (W8)

### Step 14 · Integration tests (testcontainers + real Postgres + RLS)

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `tests/integration/cms_pages_test.rs` |
| **Specs** | per ADR-001 RLS validation + bridge/03 §9.1 + VAL-001 success criteria |
| **Critical tests** | (1) published page returns 200 для owning tenant; (2) cross-tenant returns NotFound; (3) draft/archived return NotFound; (4) tenant_status='suspended' returns 403; (5) RLS POLICY actively blocks даже без `WHERE tenant_id` |
| **Verify** | `cargo test --test cms_pages_test` green + testcontainers Postgres container starts < 30s |
| **Depends on** | Steps 11, 13 |
| **Blocks** | M-A gate |

### Step 15 · Fuzz test — 1M cross-tenant attempts

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `tests/fuzz/cms_tenant_isolation.rs` |
| **Specs** | proptest 1M iterations per ENTITY.md §12.6.6 G5 + bridge/03 §9.2 |
| **Budget** | 10 min in nightly CI; 10k smoke в обычном CI |
| **Verify** | 0 leaks reported |
| **Depends on** | Step 14 |
| **Blocks** | M-A gate |

### Step 16 · Performance baseline — oha + EXPLAIN snapshots

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `docs/perf/baseline-site1-<date>.txt`, `docs/perf/ax-result-<date>.txt`, `docs/perf/explain/cms_pages_get_published.txt` |
| **Specs** | per ENTITY.md §11.6 + RFC-001 success criteria S4 |
| **Steps** | (1) start SITE1 локально; (2) `oha -n 10000 -c 100 -H 'X-Tenant-Slug: pilot' http://localhost:3010/...`; (3) start AX `cargo run --release -p ax-server`; (4) repeat oha против `localhost:7000`; (5) compare p50/p95/p99/RSS |
| **Verify** | AX p95 ≤ SITE1 p95 (target ≤ 60ms) |
| **Depends on** | Step 13 + SITE1 running |
| **Blocks** | M-A gate (S4 criterion) |
| **AVTONOM-OOS** | этой сессии не выполнимо (требует server runs) |

### Step 17 · Caddy snippet + rollback drill

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `ops/caddy/Caddyfile.snippets/cms-ax-pilots.caddy`, `docs/releases/v0.1.0/{ROLLBACK.md,ROLLBACK_DRILL.md}` |
| **Specs** | per ADR-001 D9 + ENTITY.md §12.6.5 |
| **Drill protocol** | (1) tail logs `caddy` + `ax-server`; (2) trigger fake regression в pilot tenant; (3) remove `import cms_ax_pilots` from main Caddyfile; (4) `caddy reload`; (5) timestamp начала + конца |
| **Pass criterion** | drill < 5 min, no 5xx errors during reload |
| **Depends on** | Steps 12, 13 |
| **Blocks** | M-A gate (S5 criterion) |

---

## Phase A — Pilot (W9-W10) — 1 tenant production cutover

### Step 18 · AX deploy + Caddy routing для 1 pilot tenant

| Field | Value |
|-------|-------|
| **Touched files** | `ops/systemd/ax-server.service` (non-spine new), VPS Caddyfile (production state — outside repo; manually edited — **NOT in this commit's diff**) |
| **Specs** | per ADR-001 D9 + ENTITY.md §6 VPS regulation + §21.0 baseline |
| **Pre-deploy checklist** | (1) PgBouncer transaction mode confirmed (R1 mitigation); (2) `ax_app_role` exists в prod DB; (3) Sentry DSN valid (R6); (4) systemd unit syntax verified |
| **Verify** | `curl https://pilot-tenant.spa.me/api/v1/cms/pages/public/by-slug/about?locale=ru` → 200 от AX (`X-Stack: ax` header) |
| **Depends on** | Step 17 + user explicit ok (SPINE-touch: production VPS) |

### Step 19 · Contract diff — byte-for-byte JSON vs SITE1

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `scripts/contract-diff.sh` (new), `docs/audit/contract-diff-results-<date>.txt` |
| **Specs** | per RFC-001 S1 criterion + Audit §4.2 architectural constants |
| **Steps** | (1) list all published pages for pilot tenant; (2) for each: fetch SITE1 + AX in parallel; (3) normalize `X-Request-Id`, `X-Stack`, timestamps; (4) diff |
| **Pass criterion** | 0 differences после нормализации |
| **Depends on** | Step 18 |
| **Blocks** | M-A gate (S1 criterion) |

### Step 20 · Live observability — Sentry + Grafana cutover-watch

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `ops/observability/dashboards/cms-cutover-watch.json` |
| **Specs** | per ENTITY.md §12.6.7 + ADR-001 D8 |
| **Sentry project** | `nas-ax` with tags `stack=ax`, `migration_phase=pilot`; alert `nas_tenant_mismatch_total > 0` |
| **Grafana dashboard** | side-by-side SITE1 vs AX p50/p95/p99/error rate/RPS/RSS/alloc/req |
| **Depends on** | Step 18 |

### M-A Gate · Phase A → Phase B

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `docs/releases/v0.1.0/DECISION.md` |
| **Conditions** | All 8 RFC success criteria (S1-S8) green + 2 weeks pilot observation + 0 incidents |
| **Reviewers** | minimum 2 (user + ?) |
| **Depends on** | Steps 18-20 + 2-week observation |

---

## Phase B — Stage (W11-W12)

### Step 21 · Leptos SSR adapter

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `crates/presentation/src/leptos/{tenant_shell,cms_page_component}.rs`, `crates/presentation/Cargo.toml` (add feature `leptos-ssr = default`) |
| **Specs** | per ADR-001 D5 (deliberate visual deviation) + Audit §6.6 EdRenderer pattern |
| **Verify** | HTML byte-diff vs SITE1 - допустимы только X-Stack / X-Request-Id / timestamps / per ADR-001 C4 visual deviation |
| **Depends on** | M-A gate |

### Step 22 · Expand pilot → 3 tenants (varied design tokens)

| Field | Value |
|-------|-------|
| **Touched files** | Update `ops/caddy/Caddyfile.snippets/cms-ax-pilots.caddy` (add 2 more hosts) |
| **Tenants** | pentagon (тёмный strict), barbiespa (luxury pink), roxy-spa (cyberpunk neon) — для variance test |
| **Depends on** | Step 21 + 2-week observation Phase A |

### Step 23 · HTML diff variance test — 3 contrasting tenants

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `tests/integration/test_page_design_variance.rs` |
| **Specs** | per Audit §6.7 + bridge/04 §7 expectation |
| **Pass criterion** | HTML diff между 3 темами отличается **только** в CSS-vars + Google Fonts URL |
| **Depends on** | Step 22 |

### M-B Gate · Phase B → Phase C

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `docs/releases/v0.2.0/DECISION.md` |
| **Conditions** | perf p95 ≤ 60ms confirmed под 3× load + design variance test green + 0 incidents |
| **Depends on** | Step 23 + 2-week observation |

---

## Phase C — GA (W13-W16) — all whitelisted tenants

### Step 24 · Rollout — all pilot tenants

### Step 25 · Continuous monitoring — perf + isolation + alloc budget

| Field | Value |
|-------|-------|
| **Cadence** | Daily Grafana review (5 min); weekly Sentry review; nightly `cargo xtask alloc-budget` regression check |
| **Escalation** | Any regression → auto-pause rollout + ADR на возобновление |

---

## Phase D — Retire (W17)

### Step 26 · Deprecate SITE1 `GET /v1/cms/pages/public/by-slug`

| Field | Value |
|-------|-------|
| **Touched files (SITE1 — outside this repo!)** | `barbie/SITE1/apps/api/src/cms/cms.controller.ts` — mark `@Deprecated` + warning log |
| **Caveat** | DROP route только когда 7 дней 0 RPS на endpoint'е |
| **Reversible** | Trivial revert |

### M-D Gate · Migration complete

| Field | Value |
|-------|-------|
| **Touched files (non-spine)** | `docs/releases/v1.0.0/DECISION.md` + `docs/interop/OWNERSHIP.md` finalized |
| **Outcome** | AX = canonical read для cms_pages; SITE1 = canonical write |

---

## Dependency graph (simplified)

```
[Audit ✓] → [RFC ✓] → [ADR ✓] → [PLAN ✓] → [VAL]
                                              │
                                              ▼
                              [Step 4 workspace] ─┬─ [Step 5 xtask]
                                                  ├─ [Step 6 CI + deny]
                                                  │
                                                  ▼
                                          [Step 7 common]
                                                  │
                                                  ▼
                                          [Step 8 domain]
                                                  │
                                                  ▼
                                          [Step 9 application]
                                                  │   ┌──────────────────────┐
                                                  ▼   ▼                      │
                                          [Step 10 migration]                │
                                                  │                          │
                                                  ▼                          │
                                          [Step 11 infrastructure]           │
                                                  │                          │
                                                  ▼                          │
                                          [Step 12 presentation]             │
                                                  │                          │
                                                  ▼                          │
                                          [Step 13 server]                   │
                                                  │                          │
                                                  ▼                          │
                                          [Steps 14-17 validation]           │
                                                  │                          │
                                                  ▼                          │
                                          [M-A gate]                         │
                                                  │                          │
                                                  ▼                          │
                                          [Steps 18-20 pilot]                │
                                                  │                          │
                                                  ▼                          │
                                          [Step 21 Leptos] [Step 22 expand]  │
                                                  │           │              │
                                                  ▼           ▼              │
                                          [Step 23 variance]                 │
                                                  │                          │
                                                  ▼                          │
                                          [M-B gate]                         │
                                                  │                          │
                                                  ▼                          │
                                          [Step 24 GA rollout]              │
                                                  │                          │
                                                  ▼                          │
                                          [Step 25 monitoring]              │
                                                  │                          │
                                                  ▼                          │
                                          [Step 26 retire]                  │
                                                  │                          │
                                                  ▼                          │
                                          [M-D gate complete]               │
```

---

## Spine-touch summary (для CLAUDE.md §M references)

| File / Action | Spine? | Touched in which step? |
|---------------|--------|------------------------|
| `barbie/ax/ENTITY.md` | ✅ SPINE | None (frozen for this PLAN) |
| `barbie/ENTITY.md` | ✅ SPINE | None |
| Applied production migration | ✅ SPINE-action | Step 10 application (creation = non-spine; apply via psql = spine) |
| New crate `Cargo.toml` files | non-spine | Step 4 |
| `crates/*/src/*.rs` (Rust code) | non-spine | Steps 7-13 |
| `.github/workflows/ci.yml` | non-spine | Step 6 |
| `docs/audit/`, `docs/rfc/`, `docs/adr/`, `docs/plans/`, `docs/validations/`, `docs/perf/`, `docs/interop/`, `docs/releases/` | non-spine | Steps 1-3, 16-17, 19, M-gates |
| `migrations/*.sql` (creation) | non-spine | Step 10 |
| Production VPS Caddyfile | ✅ SPINE | Step 18 (manual user-driven action) |
| `ops/caddy/` snippet (local to repo) | non-spine | Steps 17, 22 |
| `ops/systemd/` unit | non-spine | Step 18 (creation; deploy on VPS = spine-action) |

---

*Plan complete. Next: write VAL-001 → bootstrap workspace.*
