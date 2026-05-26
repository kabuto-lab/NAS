# RFC-004 — pgmq queue (transactional tier)

- **Status:** Accepted · backfilled 2026-06-17
- **Date:** 2026-06-17
- **Owner:** AX•ARCHITECT
- **References:** ENTITY §3.4.2 (pool isolation), §3.8 (queue tiers); [PLAN-004](../plans/PLAN-004-pgmq-bootstrap.md); [ADR-005](../adr/ADR-005-queue-split-pgmq-nats.md)

## Context

AX•CMS has two distinct queue workloads that look superficially
similar but have opposite consistency requirements:

- **Transactional jobs.** Escrow state transitions, payment
  state-machine ticks, image-variant idempotency keys. These MUST
  commit atomically with the business row that triggered them — if
  the parent transaction rolls back, the queue entry must not exist.
- **High-volume event streaming.** Analytics events, cache
  invalidation broadcasts, webhook fan-out. These are at-most-once
  with replay; ordering matters only within a stream.

A single queue technology cannot serve both without compromising
one — either the transactional class loses atomicity (NATS) or the
high-volume class loses throughput (pgmq).

## Why

ENTITY §3.8 specifies the split: **pgmq** for the transactional
class, **NATS JetStream** (or Redpanda as fallback) for the high-
volume class. RFC-004 covers only the pgmq half; the NATS half
ships when the first analytics-class surface lands (M5+).

pgmq sits inside Postgres, so a `BEGIN ... INSERT row ... pgmq.send
... COMMIT` is a single atomic step. No two-phase commit, no
outbox table, no dual-write reconciliation job.

## Decision

- pgmq extension installed via `migrations/0001_pgmq_bootstrap.sql`
  (additive — creates the extension + three queues).
- Adapter `crates/infrastructure/src/queue/pgmq.rs` exposes the
  `Queue` port trait (`crates/application/src/ports/queue.rs`).
- Adapter holds a clone of the **§3.4.2 `worker_pool`** —
  `http_pool` is never touched by queue ops. Pool isolation is the
  point.
- SQL via `sqlx::query` (runtime) rather than `sqlx::query!`
  (compile-time). Reason: pgmq schema is not present at workspace
  `cargo check` time; forcing the macro would block every dev on a
  stack-wide DB bootstrap. Trade-off documented in PLAN-004 and
  recorded as `AI-Default` in the 2026-05-25 SESSION_LOG.
- `QueueMessage` payload is `serde_json::Value` (NOT a typed enum)
  so consumers can deserialize per-job-type without a shared schema
  ceremony.

## TLA layers

- **L1 Correctness** — pgmq is transactional by construction. The
  `Queue` trait surface forces explicit lifecycle (`send`, `read`,
  `delete`, `archive`) — no implicit ack-on-read, so a panic mid-
  processing leaves the message visible for redelivery.
- **L2 Performance** — `read(qty)` is one roundtrip returning N
  rows; no N+1. Payload type `serde_json::Value` shares the
  deserialization buffer with the HTTP body decoder (the same
  `simd-json` parser per ENTITY §3.11).
- **L3 Scalability** — worker_pool is sized independently of
  http_pool (§3.4.2 contract). Adding NATS for high-volume events
  is additive — sibling module `crates/infrastructure/queue/nats/`.
- **L4 Operability** — every error path returns
  `QueueError::Backend(String)` carrying the upstream message, so
  operators grep server logs directly. Visibility timeout (`vt`) is
  configurable per `read` call; redelivery rate is a derived metric
  from the duplicate-key collision count (FM-004 in
  `memory/sentinel_init.md`).

## Consequences

- (+) Atomic commit with business row — no outbox, no reconciliation.
- (+) Pool isolation preserved by construction.
- (+) Adding NATS later does not require ripping out pgmq; the
  `Queue` trait abstracts both.
- (−) `serde_json::Value` payloads sacrifice compile-time schema
  enforcement for flexibility. Consumers re-deserialize per job
  type; mistakes are runtime errors. Acceptable until M9 (when
  plugin SDK formalizes job schemas).
- (−) `sqlx::query` runtime SQL gives up compile-time prepared-
  statement validation. The integration test (`pgmq_integration.rs`
  `#[ignore]`) catches drift on operator-triggered local runs.
- (−) Visibility-timeout tuning is per-consumer; misconfiguration
  causes duplicate delivery (mitigated by FM-004 idempotency keys).

## References

- [PLAN-004](../plans/PLAN-004-pgmq-bootstrap.md) — file-level plan
- [ADR-005](../adr/ADR-005-queue-split-pgmq-nats.md) — the split
  decision
- ENTITY §3.4.2 (pool isolation), §3.8 (queue tiers)
- `memory/sentinel_init.md` FM-004 (vt re-delivery duplicates)
