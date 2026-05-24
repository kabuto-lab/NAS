# ROLLBACK — `nas2-pool-validator`

- **Status:** Active
- **Date:** 2026-05-25
- **Owner:** AX•ARCHITECT
- **References:** VAL-002, PLAN-003, ENTITY §3.4.1

## Why this document exists

`nas2-pool-validator` is a **HARD GATE**: a non-zero exit from
`ensure_transaction_mode` aborts boot before the API listener binds. Such
gates need an explicit, pre-written escape path so an on-call engineer at
03:00 isn't writing it under pressure.

Three rollback options, ranked by cost (low → high):

---

## Option A — Fix the upstream PgBouncer config (preferred)

**When:** an ops change set `pool_mode = session` or `statement` by mistake.

```bash
# On the VPS, inspect what's running:
psql -h <host> -p <pgbouncer-port> -U postgres pgbouncer -c "SHOW CONFIG;" \
  | grep pool_mode

# Fix in ops/pgbouncer/pgbouncer.ini → pool_mode = transaction
# Then:
docker compose -f docker-compose.dev.yml restart pgbouncer
cargo xtask pool-mode-check --database-url "$DATABASE_URL_HTTP"   # expect "OK"
```

Risk: low. Reversal cost: zero (no code change).

---

## Option B — Soft-gate via environment override

**When:** a transient incident requires booting the server despite a stale
PgBouncer config (e.g., to drain in-flight requests during a maintenance
window).

Edit `apps/server/src/main.rs` `main()` and wrap the validator calls:

```rust
if std::env::var("AX_POOL_MODE_GATE_DISABLED").is_err() {
    validate_pool_mode("http",   &http_pool).await?;
    validate_pool_mode("worker", &worker_pool).await?;
    validate_pool_mode("admin",  &admin_pool).await?;
}
```

Set `AX_POOL_MODE_GATE_DISABLED=1` for the affected boot. **Critical:**
remove the env var immediately after the maintenance window; the file edit
should also be reverted by the next deploy.

Risk: medium — RLS contract becomes runtime-only. Reversal cost: low (one
revert commit + env-var unset).

---

## Option C — Disable the validator at the source

**When:** Options A and B are both blocked, or the validator itself is
buggy (e.g., a SQLx version drift breaks `SHOW pool_mode` parsing).

Comment out the three `validate_pool_mode(...)` calls in
`apps/server/src/main.rs`. Apply the change as a fast-roll deploy.

This downgrades §3.4.1 from boot-gate to deploy-time hope. The
`/health/pool` endpoint remains as a defense in depth (returns 503 if
drift is detected at runtime — at minimum the load balancer can de-list
the instance).

Risk: high — primary safety contract is off. Reversal cost: low (one
revert commit), but expect to lose the next debug session to "why is
session-mode silently corrupting our RLS context?".

---

## What MUST NOT happen during any rollback

- **Do not** push a `BYPASSRLS` role into Postgres "to make it work". RLS
  is the load-bearing wall of the multi-tenant doctrine (ENTITY §3).
- **Do not** delete `nas2-pool-validator` or remove it from
  `apps/server/Cargo.toml`. The validator is a contract; deleting it
  hides the contract violation, it does not fix it.
- **Do not** silence the `event = "pool_mode_drift"` Sentry alert without
  filing an incident ticket. The alert is the deferred enforcement.

## Re-enable path (any option used)

1. Set `pool_mode = transaction` in `ops/pgbouncer/pgbouncer.ini`.
2. Restart PgBouncer.
3. `cargo xtask pool-mode-check --database-url "$DATABASE_URL_HTTP"` returns
   exit 0.
4. `git revert` the rollback commit (Option B/C only).
5. Redeploy.
