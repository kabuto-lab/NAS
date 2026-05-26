# VAL-001 — CMS pages tests (Post + Block + handler)

- **Status:** Active · backfilled 2026-06-17
- **Date:** 2026-06-17
- **Owner:** AX•ARCHITECT
- **Scope:** `crates/domain` Post FSM + Block roundtrip · `crates/application` `GetPublishedPageBySlug` use case · `crates/presentation` `get_page_by_slug` handler
- **References:** [RFC-001](../rfc/RFC-001-cms-pages-publish.md); [ADR-001](../adr/ADR-001-cms-pages-architecture.md); PLAN-G2 · PLAN-G3 · PLAN-G4

## Test pyramid coverage

| Layer | Where | Count | Status |
|---|---|---:|---|
| Unit — Post FSM transitions | `crates/domain/src/post.rs::tests` | 5 | ✅ W2 D2 |
| Unit — PostSlug validation | `crates/domain/src/post.rs::tests` (or site.rs) | 4 | ✅ W2 D2 |
| Unit — Block (de)serialization roundtrip | `crates/domain/src/block.rs::tests` | 4 | ✅ W2 D4 |
| Unit — Capability set + cache key hash | `crates/domain/src/capability.rs::tests` | 3 | ✅ W2 D3 |
| Use case — GetPublishedPageBySlug | `crates/application/src/queries/get_published_page_by_slug.rs::tests` | 5 | ✅ W3 D1 |
| Integration — axum oneshot 200 path | `crates/presentation/tests/get_page_by_slug.rs::returns_200_with_post` | 1 | ✅ W3 D5 |
| Integration — 403/404/400 matrix | `crates/presentation/tests/get_page_by_slug.rs` | 3 | ✅ W4 D1 |
| Integration — router smoke | `crates/presentation/tests/router_smoke.rs` | 3 | ✅ W3 D4 |

## Run commands

```bash
# Domain layer (unit only)
cargo test -p nas2-domain --lib

# Application layer (unit + use case)
cargo test -p nas2-application --lib

# Presentation layer (integration via axum oneshot)
cargo test -p nas2-presentation --tests
```

## TLA layers exercised

- **L1 Correctness** — the FSM is total; the slug validator
  rejects every malformed candidate enumerated in the test set;
  the use-case enforces capability before repo call. The 403 path
  proves the gate is wired (caps absent → 403, not 200).
- **L2 Performance** — no perf claims attached to these tests. The
  first `criterion` bench for the read path lands when the real
  `PgPostRepository` ships (M2 W6 D2, slot VAL-006 forward-binding).
- **L3 Scalability** — the use case takes an `Arc<dyn PostRepository>`
  port (static-dispatch on call, dynamic-dispatch on test substitution
  via mockall). Repo fan-out is the adapter's concern, not the use
  case's.
- **L4 Operability** — the handler returns explicit AppError variants
  that map to specific HTTP codes (`AppError::NotFound` → 404,
  `AppError::Forbidden` → 403, `AppError::Validation` → 400). Each
  test asserts the wire-shape directly, so an operator looking at a
  curl response can predict the test that would have caught a
  regression.

## What is missing (legitimately deferred)

- **Property tests** — RLS isolation, slug roundtrip under arbitrary
  Unicode. Slot: VAL-005 (RLS) ratified M2 W1 D5 (2026-06-26);
  slug-roundtrip proptest deferred to M3 W2.
- **Bench coverage** — the read-path benchmark is unmeaningful before
  the real DB repo lands. Slot: VAL-006 (Media RLS), M3 W1 D5;
  read-path criterion lands M2 W6 D2 alongside `PgPostRepository`.
- **Concurrency / saturation** — Test Pilot tier-3 activation deferred
  to M11+ when the production-shaped workload exists (per
  `memory/sentinel_init.md` threat surface table).

## Evidence in commits

```bash
git log --oneline --grep='PLAN-G[234]' | head
```

The 18-commit M1 series at HEAD `b67302e..` covers PLAN-G2 / G3 / G4
inclusive of this backfill.

## References

- [RFC-001](../rfc/RFC-001-cms-pages-publish.md) — why typed blocks
- [ADR-001](../adr/ADR-001-cms-pages-architecture.md) — the
  architectural choice
- PLAN-G2 / G3 / G4 — the implementation milestones (in
  `docs/session-plans/WEEK-0{1,2,3,4}.md` and the daily prompts)
- ENTITY §16 (testing pyramid)
