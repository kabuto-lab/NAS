# VAL-002 — Pool-mode contract

- **Status:** Active
- **Date:** 2026-05-25
- **Owner:** AX•ARCHITECT
- **References:** ENTITY §3.4.1; [ADR-003](../adr/ADR-003-pool-isolation.md); P2 of AVTONOM 2026-05-25

## Success criteria (measurable)

1. `nas2_pool_validator::ensure_transaction_mode` returns `Ok(())` against a
   PgBouncer running with `pool_mode = transaction`.
2. The same call returns `Err(PoolValidationError::WrongMode { actual: Session })`
   against a PgBouncer running with `pool_mode = session`.
3. `detect_pool_mode` distinguishes `Statement` correctly when explicitly set.
4. `apps/server/main.rs` refuses to bind the API listener if any of the three
   pools (`http_pool`, `worker_pool`, `admin_pool`) fails the gate. Exit code is
   non-zero so `vps:after-pull` rolls back the deploy.

## Tests required

- [x] Domain unit — `PoolMode::from_str` parses every known mode + unknown
  fallback (`crates/pool-validator/src/lib.rs` `#[cfg(test)]`).
- [x] Integration — `crates/pool-validator/tests/pool_mode_integration.rs`:
  - `test_ensure_transaction_mode_accepts_transaction_pool`
  - `test_ensure_transaction_mode_rejects_session_pool`
  - `test_detect_pool_mode_returns_statement_when_configured`
  Each gated by `#[ignore = "needs Docker"]`; run with
  `cargo test -p nas2-pool-validator --tests -- --ignored`.
- [x] HTTP probe — `/health/pool` endpoint (`apps/server/main.rs:health_pool`)
  surfaces drift via 503; integration with deploy pipeline is the operability
  contract.
- [ ] Performance — `SHOW pool_mode` round-trip stays under 5 ms p95 on local
  PgBouncer (informational; not gating).

## Observability

- New event: `event = "pool_mode_drift", severity = "critical"` emitted from
  `ensure_transaction_mode` on any non-transaction value. Sentry should
  page-rule on this.
- New tracing span: `pool-mode contract satisfied` on success (info level) —
  appears once per pool at boot.
- New CLI gate: `cargo xtask pool-mode-check --database-url $DATABASE_URL_HTTP`
  for pre-deploy validation (P4 X1).

## Rollback

See `docs/rollback/ROLLBACK-pool-validator.md` — three concrete escape
hatches with cost/risk for each.

## Decision criteria for ENTITY §3.4.1 amendment

If, after 60 days of production observation, the validator never fires *and*
two independent infra audits confirm transaction-mode invariance is enforced
at provisioning time, the boot-time gate may be downgraded to a startup
warning (with the `/health/pool` endpoint kept as a defense in depth). Until
then: hard-gate stays.
