# PLAN-003 — PgBouncer split-pool topology

- **Status:** Implemented (P3 of AVTONOM 2026-05-25 session)
- **Date:** 2026-05-25
- **Owner:** AX•ARCHITECT
- **References:** ADR-003, VAL-003, ENTITY §3.4.2

## Objective

Materialise the three logical pools (`http_pool`, `worker_pool`,
`admin_pool`) inside PgBouncer so the server's three `PgPool` instances
each speak to a dedicated, budget-isolated alias.

## Files (all delivered)

| # | File | Spine? | Status |
|---|------|--------|--------|
| 1 | `ops/pgbouncer/databases.ini` | non-spine | **NEW** — three logical pool aliases over physical `ax_cms_dev` (pool_size 25/10/5) |
| 2 | `ops/pgbouncer/pgbouncer.ini` | non-spine | **NEW** — master config with `%include /etc/pgbouncer/databases.ini`, `pool_mode = transaction` |
| 3 | `ops/pgbouncer/userlist.txt` | non-spine | **NEW** — auth_file required by PgBouncer even under `auth_type = trust` |
| 4 | `docker-compose.override.yml` | non-spine | **NEW** — mounts the three files into edoburu/pgbouncer:1.23.1, replaces entrypoint so env-driven generation cannot overwrite mounts |
| 5 | `.env.example.split-pool` | non-spine | **NEW** — reference doc with `DATABASE_URL_HTTP/WORKER/ADMIN` examples |
| 6 | `docker-compose.dev.yml` | **spine** | UNTOUCHED — the override covers all delta |
| 7 | `.env.example` | **spine** | UNTOUCHED — `.env.example.split-pool` documents the additions separately |

## Validation

- VAL-002 — pool-mode contract: covered by `nas2-pool-validator` + the
  integration tests added in P2.
- VAL-003 — pool isolation: design covered here, runtime smoke-test deferred
  pending multi-pool integration fixture.

## Rollout

1. `docker compose -f docker-compose.dev.yml up -d` (override auto-merges).
2. `psql -h localhost -p 6450 -U postgres http_pool -c "SHOW pool_mode;"`
   → expect `transaction`.
3. Set the three `DATABASE_URL_*` env vars per `.env.example.split-pool`.
4. `cargo run -p nas2-server` — boot order:
   - build three `PgPool` instances
   - `ensure_transaction_mode` against each
   - bind listener
5. Open `/health/pool` — expect 200 (all three pools report `transaction`).

## Rollback

See `docs/rollback/ROLLBACK-pool-validator.md` — the validator is the
single hard-gate enforcing this PLAN; toggling it is the rollback path.

## Risks / open questions

- edoburu/pgbouncer entrypoint behavior with mounted config — pinned tag
  1.23.1 in the override so the behavior cannot regress under `:latest`.
- For production: a hardened Dockerfile that bakes the config in (rather
  than bind-mounting from host) and switches `auth_type` to
  `scram-sha-256` is the next deliverable (out of scope of this PLAN).
