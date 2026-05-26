# WEEK-06 · Jun 29 – Jul 3, 2026 (M2 W2)

> Theme: **PgPostRepository + with_tenant single-RTT pattern + integration tests**
> Goal: G2

## Daily slots

| Date | Day | File | Scope summary | Est h |
|---|---|---|---|---:|
| 2026-06-29 | Mon | `daily/2026-06-29.md` | `nas2-tenant::with_tenant` helper + `set_config('app.tenant_id',$1,true)` single-RTT pattern + unit tests | 4 |
| 2026-06-30 | Tue | `daily/2026-06-30.md` | `crates/infrastructure/src/persistence/post.rs` — `PgPostRepository::find_by_slug` + `find_by_id` + 2 integration tests | 4 |
| 2026-07-01 | Wed | `daily/2026-07-01.md` | `PgPostRepository::list_published` (offset/limit pagination, single SQL, no N+1) + integration test | 4 |
| 2026-07-02 | Thu | `daily/2026-07-02.md` | `PgPostRepository::insert` + `update`; transactional outbox stub for M11 search reindex | 4 |
| 2026-07-03 | Fri | `daily/2026-07-03.md` | Wire real repo into `AppState`; replace mock in `tests/get_page_by_slug.rs`; first DB-touching bench `repo_find_by_slug`; CI integration-test job proposal in SESSION_LOG | 4 |

## Dependencies satisfied entering W6

- RLS policies applied (W5 D5)
- `app.tenant_id` GUC convention nailed in ADR-010
- 5 migrations on disk + applied
- VAL-005 RLS proptest green

## Definition of done (W6)

- [ ] `nas2-tenant::with_tenant<F, R>(pool, tenant, f)` ships
- [ ] All 5 `PostRepository` methods implemented in
  `nas2-infrastructure::persistence::post`
- [ ] Each method: ONE SQL roundtrip per call (verified via Postgres
  `pg_stat_statements` snapshot in integration test)
- [ ] 5 integration tests (#[ignore = "needs Docker"]) — one per repo
  method — covering happy + RLS-blocked paths
- [ ] `AppState::post_repo` wired to `PgPostRepository` in
  apps/server (was mock from M1)
- [ ] `presentation/tests/get_page_by_slug.rs` happy-path test now
  uses the real DB (#[ignore]) + retains the MockRepo path for fast
  CI lib tests
- [ ] `docs/perf/baseline.json` extended with `repo_find_by_slug`
  (operator-captured)
- [ ] No new workspace `Cargo.toml` deps

## Carry-over (W6 → W7)

If integration tests reveal an N+1 (e.g. list_published does a
follow-up query per row) → fix immediately, OR carry-over to W7 D1
as P0 before JWT work starts.

## W6-specific risks

| Risk | Mitigation |
|---|---|
| pgbouncer transaction-mode incompatible with prepared statements | sqlx already configured for runtime SQL (no `query!`); revisit if perf bench shows prepared-statement gap |
| `with_tenant` lifetime issues with closure capturing `&pool` | use `impl AsyncFn(...)` or boxed-future trait; sqlx examples confirmed the pattern |
| Repo error mapping pollutes AppError variants | introduce `infrastructure::PgError` internal type; map at the trait-impl boundary to `AppError` |

## MPD-001 weave overlay (W6)

| Day | ADD-ON | Budget | Spec |
|---|---|---:|---|
| 2026-06-29 (D1) | Domain module restructure (`content/`, `auth/`, `commerce/`, `crm/`, `shared/` subdirs + `lib.rs` re-exports) | 30 min | `docs/session-plans/MPD-001-M2-WEAVE.md §2` |

Restructure is mechanical (move + re-export); MUST keep `cargo check`
green. Hard stop: revert if not green at 15-min mark and defer to D2
CARRY-OVER.
