# RFC-003 — TaskSupervisor (bounded spawn governance)

- **Status:** Accepted · backfilled 2026-06-17
- **Date:** 2026-06-17
- **Owner:** AX•ARCHITECT
- **References:** ENTITY §4.8 (graceful shutdown), §29 (TaskCategory isolation), §8 (engineering rules); [VAL-004](../validations/VAL-004-task-supervisor.md); `crates/runtime/src/supervisor.rs`

## Context

A direct `tokio::spawn` is the path of least resistance — and it is
exactly the path that produces three classes of production incident:

- **Silent leaks.** A spawned future that holds an `Arc<DbPool>` or
  a `Receiver<T>` keeps half the runtime alive until the process
  exits. No log line, no metric, no signal.
- **Unobservable failures.** A panic in a bare `tokio::spawn` is
  caught by Tokio's panic-handler and *that's it* — no `tracing`
  span, no error counter increment, no oncall page.
- **Hostile shutdown.** SIGTERM arrives, the binary exits, in-flight
  spawned work is killed mid-step. Half-written queue ack? Half-
  uploaded media variant? You lose them and you find out from a
  customer ticket.

The supervisor is the contract that lets the rest of the codebase
treat async tasks as observable, drainable, blast-radius-isolated
resources.

## Why

ENTITY §4.8 mandates "graceful shutdown" — that is unimplementable
on top of `tokio::spawn`. ENTITY §29 mandates per-`TaskCategory`
failure isolation — that requires multiple supervisors, each owning
a subset of the runtime budget. Neither is achievable as a code-
review-only norm; they need a tracker and a code-path that *cannot*
be circumvented short of `unsafe` patching the workspace.

## Decision

- All async tasks go through `TaskSupervisor::spawn(category, name,
  future)`.
- Each supervisor owns:
  - a `TaskCategory` tag (Http / Queue / Image / Report / Email /
    SearchIndex)
  - a `CancellationToken` propagated to every spawned task
  - a `TaskTracker` for bounded drain
- `xtask magic-check` (W1 D2) bans raw `tokio::spawn` outside
  `crates/runtime/src/supervisor.rs` — regex scan, exit 1 on hit.
- The eventual clippy `disallowed-methods` lint will codify the same
  ban at the workspace boundary.

## TLA layers

- **L1 Correctness** — `drain(deadline)` takes ownership of the
  supervisor (`self`, not `&mut self`), so post-drain spawns become
  a compile error rather than a silent leak. `DrainError::Timeout
  { in_flight: usize }` is explicit; operators see how many tasks
  refused to honor the cancel.
- **L2 Performance** — `TaskTracker` is internally `Arc<…>`; spawn
  cost is one atomic increment + the future allocation. No per-call
  lock acquisition.
- **L3 Scalability** — one supervisor per `TaskCategory` isolates
  blast radius. A runaway image worker cannot starve HTTP because
  HTTP holds its own supervisor + its own `tokio::JoinHandle` pool.
- **L4 Operability** — every spawn enters a `tracing::Span` carrying
  `category`, `name`, and a per-task UUID. An operator can pivot
  from a log line straight to every event from that task.

## Consequences

- (+) Graceful shutdown is implementable and tested (VAL-004:
  `test_drain_times_out_on_stuck_task` proves the contract).
- (+) Each `TaskCategory` is a metric label and a tracing field —
  flame-graphing per-category is trivial.
- (+) `magic-check` catches drive-by `tokio::spawn` at PR time, not
  at incident time.
- (−) Every new async surface adds one supervisor wiring step
  (constructor call in `apps/server/src/main.rs`). Trade-off
  acknowledged; the cost is one line of glue per category.
- (−) The cancellation token is cooperative — a future that never
  awaits cancellation can still hang past the drain deadline. The
  test enforces the timeout; operators must escalate if a category
  repeatedly fails to drain.

## References

- [VAL-004](../validations/VAL-004-task-supervisor.md) — 3 unit
  tests prove the contract
- `crates/runtime/src/supervisor.rs` — the implementation
- ENTITY §4.8, §29, §8.7 (lock-minimization), §10 (failure-mode
  analysis required per component)
