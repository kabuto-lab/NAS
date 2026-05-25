# WEEK-05 · Jun 22–26, 2026 (M2 W1)

> Theme: **RLS policy SQL + tenant-scoped migrations + proptest property of isolation**
> Goal: G1 (RLS + migrations)

## Daily slots

| Date | Day | File | Scope summary | Est h |
|---|---|---|---|---:|
| 2026-06-22 | Mon | `daily/2026-06-22.md` | Wire `check-planning-refs` into CI matrix; design migration sequencing + `app.tenant_id` GUC contract; start `migrations/0002_sites_posts.sql` | 4 |
| 2026-06-23 | Tue | `daily/2026-06-23.md` | Finish `0002_sites_posts.sql` (tables + indices); ROLLBACK doc; `nas2-cli db migrate` skeleton against admin_pool | 4 |
| 2026-06-24 | Wed | `daily/2026-06-24.md` | `0003_rls_sites_posts.sql` — RLS policies; ADR-010 shape; first integration test asserts policies present | 4 |
| 2026-06-25 | Thu | `daily/2026-06-25.md` | `0004_users_roles_caps.sql` + `0005_rls_users_roles.sql` + `0006_role_assignments.sql` | 4 |
| 2026-06-26 | Fri | `daily/2026-06-26.md` | Apply all 5 migrations end-to-end; VAL-005 RLS proptest (1000 rounds); RFC-006 JWT-auth-foundation seeded for W7 | 4 |

## Dependencies satisfied entering W5

- nas2-domain has Site / Post / User / Role / Capability aggregates (from M1)
- nas2-common has TenantId / SiteId / UserId / PostId newtypes
- pgmq bootstrap migration `0001_pgmq_bootstrap.sql` already applied
- admin_pool (apps/server) is the channel for migrations per ENTITY §3.4.2

## Definition of done (W5)

- [ ] 5 migrations (`0002`..`0006`) on disk + applied
- [ ] Each migration has a matching `ROLLBACK-NNNN-*.md`
- [ ] `app.tenant_id` GUC convention documented in ADR-010
- [ ] RLS policies enforce isolation under proptest (1000 rounds, 0 leaks)
- [ ] `xtask architecture-check / magic-check / capability-coverage /
      check-planning-refs` green
- [ ] No spine touches; no `Cargo.toml` workspace edits

## Carry-over policy (W5 → W6)

If RLS proptest finds any leak: that day's prompt becomes day N+1's
P0 BEFORE its own P1. Migration rollback is allowed (drop the
policy, re-apply). Schema changes after Fri MUST wait until W6 has
absorbed the change.

## Week-5-specific risks

| Risk | Mitigation |
|---|---|
| `sqlx::migrate!` macro requires DATABASE_URL at compile time | use `sqlx::migrate::Migrator::new("migrations").run(&pool)` — runtime API |
| pgbouncer `transaction` mode blocks `SET LOCAL`-based RLS | use `SELECT set_config('app.tenant_id', $1, true)` (transaction-scoped — works) — ENTITY §3.5 |
| proptest finds a leak → reveals an RLS hole | drop the policy, redesign, re-apply; consider `FORCE ROW LEVEL SECURITY` for the table |
| Windows testcontainers + Docker Desktop flakiness | run integration tests locally; CI matrix waits for W6 D5 patch |
