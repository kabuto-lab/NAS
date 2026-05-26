# PLAN-004 — pgmq queue adapter + bootstrap migration

- **Date:** 2026-05-25
- **Status:** implemented (unit tests green; integration test gated `#[ignore]`)
- **Scope:** application port `Queue`, infrastructure adapter `PgmqQueue`,
  migration `0001_pgmq_bootstrap.sql`
- **References:** ENTITY §3.4.2 (pool isolation), §3.8 (queue tiers); [RFC-004](../rfc/RFC-004-pgmq-queue.md) (ratified 2026-06-17, backfilled); [ADR-005](../adr/ADR-005-queue-split-pgmq-nats.md)
- **Closes:** P2 of AVTONOM 2026-05-25 session plan

## Files touched

| Path | Purpose |
|---|---|
| `crates/application/src/ports/mod.rs` | new — port re-exports |
| `crates/application/src/ports/queue.rs` | new — `Queue` trait + `QueueMessage` + `QueueError` |
| `crates/application/Cargo.toml` | +serde_json |
| `crates/application/src/lib.rs` | enable `pub mod ports` |
| `crates/infrastructure/src/queue/mod.rs` | new — adapter module |
| `crates/infrastructure/src/queue/pgmq.rs` | new — `PgmqQueue` adapter |
| `crates/infrastructure/src/lib.rs` | enable `pub mod queue` |
| `crates/infrastructure/Cargo.toml` | dev-dep nas2-application |
| `crates/infrastructure/tests/pgmq_integration.rs` | new — `#[ignore]` round-trip |
| `migrations/0001_pgmq_bootstrap.sql` | new — `CREATE EXTENSION pgmq` + 3 queues |

## Architectural rationale (ENTITY §2.1)

- **L1 Correctness** — pgmq is transactional by construction; the adapter is
  a thin SQL wrapper that never mutates queue state without an explicit
  `delete` or `archive`. The `Queue` trait surface forces explicit lifecycle:
  no implicit ack-on-read.
- **L2 Performance** — `read(qty)` is one roundtrip returning N rows; no N+1.
  Payload type is `serde_json::Value` to share the deserialization buffer with
  the HTTP body decoder later.
- **L3 Scalability** — the adapter holds a clone of the §3.4.2 `worker_pool`;
  it never touches `http_pool`. Adding NATS JetStream for high-volume eventing
  (ENTITY §3.8) is additive — a sibling module under `crates/infrastructure/queue`.
- **L4 Operability** — every error path returns `QueueError::Backend(String)`
  carrying the upstream message, so operators can grep server logs directly.

## SQL macro choice — runtime, not compile-time

`sqlx::query`/`query_scalar` (runtime SQL) is used rather than `sqlx::query!`
(compile-time). Reason: the `pgmq` schema is not present at workspace
`cargo check` time (`.sqlx/` is gitignored and the dev database may not have
pgmq installed). Forcing `query!` would block every developer on a stack-wide
DB bootstrap. The pgmq function signatures are stable across the supported
version range; the `#[ignore]` integration test catches drift the moment a
human runs it.

This trade-off is recorded as **AI-Default** in `SESSION_LOG.md`.

## Verification

- `cargo test -p nas2-application -p nas2-infrastructure --lib` — 1 passed
  (`map_err_preserves_message`); the rest of the surface is exercised by
  integration.
- `cargo clippy --workspace --all-targets -- -D warnings` — green.
- Integration: `PGMQ_IMAGE=ghcr.io/tembo-io/pgmq:latest cargo test -p
  nas2-infrastructure --tests pgmq_integration -- --ignored` — requires
  Docker + a pgmq-shipped image; deferred to operator until §3.4.2
  workflow is wired.

## Migration application

```bash
cargo run --bin nas2-cli -- db migrate          # uses admin_pool (ENTITY §3.4.2)
```

Idempotent: `pgmq.create(name)` is a no-op when the queue already exists.

## Rollback

Not applicable — bootstrap migration is additive (extension + three queues).
A future contraction migration would drop the queues with `pgmq.drop_queue`
and finally `DROP EXTENSION pgmq CASCADE`. Recorded in `docs/rollback/` once
a real reverse path is requested.
