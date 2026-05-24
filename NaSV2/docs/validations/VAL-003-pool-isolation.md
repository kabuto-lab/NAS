# VAL-003 — Pool isolation (http / worker / admin)

- **Status:** Active
- **Date:** 2026-05-25
- **Owner:** AX•ARCHITECT
- **References:** ENTITY §3.4.2, ADR-003, PLAN-003, P3 of AVTONOM 2026-05-25

## Success criteria (measurable)

1. `apps/server/main.rs` builds **three distinct `PgPool` instances** from
   `DATABASE_URL_HTTP`, `DATABASE_URL_WORKER`, `DATABASE_URL_ADMIN` — never a
   single shared pool. Verified by code review + a future
   `cargo xtask architecture-check` rule.
2. Each pool's `application_name` (and PgBouncer logical alias) matches its
   role, so `pg_stat_activity` and `SHOW POOLS;` show three distinct rows.
3. Pool budgets sum to ≤ PgBouncer `max_client_conn` (1000) and ≤ the sum of
   per-alias `pool_size` (40 in dev). The boot path applies these caps via
   `BootConfig` and the `databases.ini` mounted by docker-compose.override.yml.
4. Under simulated load (worker indexing loop saturating `worker_pool`), HTTP
   p99 latency on `http_pool` stays within 10% of the no-load baseline.

## Tests required

- [ ] Integration — extend `pool-validator` tests with a load-isolation
  scenario: spin up postgres + pgbouncer with split-pool config, hammer
  worker alias from one task while measuring HTTP alias acquire-time. Not
  in scope of P3 (deferred — requires multi-pool docker fixture).
- [ ] HTTP smoke — `oha -c 50 -z 30s http://localhost:8000/health/ready`
  during a synthetic worker burst → assert p99 < 15 ms.
- [x] Boot gate — `validate_pool_mode("http"|"worker"|"admin", …)` calls
  `nas2_pool_validator::ensure_transaction_mode` separately, so any one
  misconfigured alias fails the boot.
- [x] `/health/pool` endpoint walks all three pools (see `health_pool`).

## Observability

- One Prometheus metric per pool (when wired): `db_pool_acquire_seconds`
  with label `pool = http|worker|admin`.
- One alert: `worker_pool_acquire_p99 > 1s for 5m` — likely runaway worker.
- One dashboard panel: per-pool active connections vs `pool_size` ceiling.

## Decision criteria

Pool isolation is non-negotiable per ENTITY §3.4.2. The only acceptable
amendment is **adding** a fourth pool (e.g., `analytics_pool`), never
collapsing the three.
