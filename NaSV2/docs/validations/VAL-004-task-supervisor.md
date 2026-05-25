# VAL-004 — `TaskSupervisor` bounded spawn governance

- **Date:** 2026-05-25
- **Status:** verified · 3 unit tests passing
- **Scope:** `crates/runtime/src/supervisor.rs` + wiring in `apps/server/src/main.rs`
- **References:** ENTITY §4.8, §29 · RFC-003 (pending) · PLAN (n/a — single crate)
- **Closes:** P1 of AVTONOM 2026-05-25 session plan

## Verification matrix

| Test | Scenario | TLA layer | Pass |
|---|---|---|---|
| `test_spawn_and_drain_completes` | 10 short tasks complete inside drain budget | L1 Correctness | ✅ |
| `test_drain_times_out_on_stuck_task` | task ignoring cancel → `DrainError::Timeout { in_flight: 1 }` | L1 + L4 | ✅ |
| `test_cancel_token_propagates` | well-behaved task observes cancel, drain succeeds | L1 + L3 | ✅ |

Run:

```bash
cargo test -p nas2-runtime --lib supervisor
```

Output (2026-05-25):

```
running 3 tests
test supervisor::tests::test_spawn_and_drain_completes ... ok
test supervisor::tests::test_cancel_token_propagates ... ok
test supervisor::tests::test_drain_times_out_on_stuck_task ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## TLA narrative

- **L1 Correctness** — drain takes ownership of the supervisor, so post-drain
  spawns become a compile error rather than a silent leak.
- **L2 Performance** — `TaskTracker` is internally `Arc<…>`, so `spawn` is a
  single atomic increment + a future allocation. No per-call locks.
- **L3 Scalability** — one supervisor per [`TaskCategory`] isolates the blast
  radius. A runaway image worker cannot starve HTTP.
- **L4 Operability** — every spawn enters a `tracing::Span` carrying
  `category`, `name`, and a per-task UUID, so an operator can pivot from a
  log line straight to all events from that task.

## Coverage gap (recorded, not blocking)

The `TaskHandle::abort()` path is not yet exercised by a test. It is a thin
wrapper over `JoinHandle::abort`, which tokio covers extensively; we will add
a dedicated test alongside the first consumer that needs explicit abort
(currently no such consumer exists).

## Follow-up

- `xtask magic-check` — once implemented (P6 architecture-check lays
  groundwork) must ban raw `tokio::spawn` outside `crates/runtime`.
- Add per-category Prometheus counter (`tasks_spawned_total{category=…}`)
  when the first non-HTTP supervisor (Queue/Image) lands.
