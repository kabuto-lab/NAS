# ROADMAP — Month 2 · 2026-06-22 → 2026-07-17 (20 working days)

> Derived from `AUDIT-2026-06-22.md` (forward) + `MASTER-ROADMAP-2026-2027.md`
> M2 section. Reconcile on 2026-06-22 against actual state.

## B1 · Goals

### G1 — RLS policy SQL + tenant-scoped migrations

- **Why:** ENTITY §3.3 + §15 + GAP-ANALYSIS #9. Without RLS at the DB
  layer, app-level tenant checks are the only defence — single-bug
  away from cross-tenant data leak.
- **Success criterion:** 5 migrations applied
  (`0002..0006`); proptest (1000 rounds) inserts random posts under
  random tenants and asserts SELECT returns zero cross-tenant rows
  via every code path. `xtask architecture-check` green.
- **Estimated days:** 5 (W5)

### G2 — `PgPostRepository` + `with_tenant` single-RTT pattern

- **Why:** ENTITY §3.5 — `set_config('app.tenant_id', $1, true)` MUST
  share the transaction with the business statement; two-RTT is wasted
  latency and breaks RLS scope.
- **Success criterion:** all 5 `PostRepository` methods (`find_by_slug,
  find_by_id, list_published, insert, update`) implemented; each makes
  ONE roundtrip per call (verified by Postgres log inspection in
  integration test); 5 integration tests against testcontainers.
- **Estimated days:** 5 (W6)

### G3 — `JwtVerifier` + capability extraction from JWT

- **Why:** Replace the W4 D1 thread-local stub with real auth.
  GAP-ANALYSIS #4.
- **Success criterion:** Bearer-token + cookie-jar paths work; signed
  with HMAC-SHA256 + env-loaded key; capability claim parsed into
  `CapabilitySet`; tokens expire (default 1 h); replay-window check
  via `iat` (issued-at) + `jti` (jwt-id) — `jti` stored in moka L1
  cache for the token's TTL to detect re-use. Comprehensive test
  matrix: valid, expired, malformed signature, wrong-issuer, replay.
- **Estimated days:** 3 (W7 D1–D3)

### G4 — Posts REST CRUD (5 endpoints, real DB, JWT-gated)

- **Why:** GAP-ANALYSIS #11. M1 ships 1 endpoint; production needs
  the full Posts CRUD surface to be useful for any client.
- **Success criterion:** `GET /api/v1/pages/:slug` (already
  shipped — promote to real repo); `GET /api/v1/pages` (paginated
  list); `POST /api/v1/pages` (admin create); `PATCH
  /api/v1/pages/:slug` (admin update); `DELETE /api/v1/pages/:slug`
  (admin soft-delete via `PostStatus::Archived`). Each handler:
  capability gate + oneshot integration test + capability-coverage
  marker.
- **Estimated days:** 5 (W7 D5 + W8 D1–D3 + finalisation)

### G5 — CSRF/nonce + revisions ADR + W8 wrap

- **Why:** GAP-ANALYSIS #15 — admin write paths need CSRF; G-S3 of
  master roadmap demands a revisions story (impl in M3, decision now).
- **Success criterion:** CSRF nonce middleware (SameSite-strict cookie
  + double-submit token) live on `POST/PATCH/DELETE` admin endpoints;
  ADR-009 documents the revisions/autosaves protocol (impl deferred
  to M3); RFC-005 documents the CSRF strategy.
- **Estimated days:** 3 (W8 D3–D5 + RETRO)

---

## B2 · Anti-goals (M2 explicit non-goals)

- ❌ **No revisions impl** — defer to M3 (just an ADR this month).
- ❌ **No application passwords** — defer to M3 (just JWT this month).
- ❌ **No image upload endpoint** — defer to M3.
- ❌ **No taxonomies / comments / themes / extensions** — later months.
- ❌ **No real production deploy** — testcontainers only.
- ❌ **No PR / push** — operator only.
- ❌ **No new workspace `Cargo.toml` deps** — `jsonwebtoken`,
  `tower-governor`, `sqlx` already in workspace.
- ❌ **No leptos SSR work** — admin UI is M5+M7.

---

## B3 · Weekly breakdown

| Week | Dates | Theme | Goals |
|---|---|---|---|
| **W5** | 06-22..06-26 | Migrations + RLS policies + proptest | G1 |
| **W6** | 06-29..07-03 | PgPostRepository + with_tenant + integration tests | G2 |
| **W7** | 07-06..07-10 | JwtVerifier + caps extraction + first CRUD endpoint | G3 + G4 start |
| **W8** | 07-13..07-17 | CRUD finish + CSRF + ADR-009 + RETRO | G4 finish + G5 |

---

## B4 · Dependency DAG

```
G1 (migrations + RLS) ────► G2 (PgPostRepository depends on RLS schema)
                                       │
                                       ▼
G3 (JwtVerifier) ──────────────────► G4 (CRUD endpoints depend on real repo + real caps)
                                       │
                                       ▼
                              G5 (CSRF + ADR-009 + RETRO)
```

G1 must finish W5 before G2 starts. G2 and G3 are MOSTLY independent
(G2 doesn't need JWT — uses test-only caps), so W6 G2 and W7 G3 can
proceed in series safely (or in parallel if extra hands appear).

---

## B5 · Slack budget

- W5 D5 light slack (migration verification, fix follow-ups)
- W6 D5 carries the CI integration-test job proposal (W5 carry-over
  or fresh)
- W7 D5 first CRUD endpoint as gentle ramp into W8
- W8 D5 RETRO + next-month bootstrap (no new feature scope)

5 buffer-day equivalents across the month.

---

## B6 · Exit criteria (end of M2)

1. `cargo check --workspace --all-targets` green
2. `cargo clippy --workspace --all-targets -- -D warnings` green
3. `cargo test --workspace --lib --no-fail-fast` ≥ 80 unit tests
4. `cargo test --workspace --tests -- --ignored` runs ≥ 12
   integration tests against testcontainers Postgres
5. 5 migrations applied successfully via `nas2-cli db migrate`
6. `cargo run -p xtask -- architecture-check / magic-check /
    capability-coverage / check-planning-refs` all green
7. RLS isolation property test (1000 rounds) passes — zero
   cross-tenant rows under random tenant interleaving
8. 5 REST endpoints for Posts shipped; each oneshot-tested for
   200 / 401 / 403 / 404 / 409 / 400 / 500 paths as applicable
9. JWT verification: valid / expired / malformed / replay all
   correctly mapped to 200 vs 401
10. CSRF nonce middleware live; cross-origin POST with no token
    returns 403
11. ADR-009 (revisions) committed; impl scheduled for M3 G5
12. `docs/perf/baseline.json` extended with first DB-touching bench
    (`repo_find_by_slug`) — captured on operator's quiet machine
13. ~14% cumulative WP parity (M1 6% + M2 8%)
14. RETRO-2026-07.md + `avtonom-month-bootstrap-2026-07.md` exist
    on 2026-07-17
