---
name: forgemaster-init
description: Forgemaster initial state at 2026-05-26 Adoption Pass — perf baselines, allocation budgets, bench-slot claims
metadata:
  type: project
---

# FORGEMASTER — Init Dossier (2026-05-26 Adoption Pass)

## Live perf state

- **HEAD:** `d81946c`
- **Cached read p95 baseline:** NOT YET MEASURED (no benches landed). `apps/server` serves `/health/{live,ready,pool}` only. M2 lands the first content endpoint.
- **§7 hard targets (binding):**
  | Workload | Target | Status |
  |---|---|---|
  | Cached public read (L1 hit) p95 | 10–20 ms | unmeasured |
  | Cached public read throughput | ≥ 10 K req/s/core | unmeasured |
  | Typical public page (L1 miss, L2 hit, no image) p95 | 35–60 ms | unmeasured |
  | Complex write + image (async ack) | < 100 ms | unmeasured |
  | Cold start | ≤ 300 ms | unmeasured |
  | RSS idle (0 tenants) | ≤ 50 MB | unmeasured |
  | RSS / warm tenant | ≤ 1 MB | unmeasured |

## Bench / VAL slots claimed

| Slot | Purpose | Trigger day | Status |
|---|---|---|---|
| VAL-002 | Pool-mode contract verification | M1 ✓ | already ratified |
| VAL-003 | Pool isolation | M1 ✓ | already ratified |
| VAL-004 | Task supervisor | M1 ✓ | already ratified |
| VAL-005 | RLS isolation proptest (1000 rounds) | M2 W1 D5 (2026-06-26) | claimed |
| VAL-006 | Media RLS isolation | M3 W1 D5 (2026-07-24) | claimed |
| VAL-007 | libvips idempotency | M3 W2 D5 (2026-07-31) | claimed |
| VAL-008 | Autosave protocol (no-updated_at-bump invariant) | M3 W4 (2026-08-12+) | claimed |
| **VAL-009** | **Editor bundle-size guard ≤ 200 KB gz** | **M3 W3 D5 (2026-08-07)** | **claimed at 2026-05-26 Adoption Pass** |

## Allocation budgets (per request, where defined)

- `/health/live` — 0 allocations target (current impl returns `&'static str "ok"`).
- `/health/ready` — 1 SQL roundtrip (`SELECT 1`); allocation budget bound by sqlx prepared-statement cache.
- `/health/pool` — 3 SQL roundtrips (3 pools); same prepared-statement bound.
- All future hot-path endpoints — **per-request allocation count must be stated** before ratification.

## Static-dispatch vs dyn surface

- Workspace policy: prefer monomorphized generics over `Box<dyn T>` (ENTITY §8.3-§8.4).
- Existing dyn sites: zero in `apps/server` hot path; `Box<dyn StorageAdapter>` planned for M3 W3 D3 — admissible (cold init).
- ADR-010 commitment: `EditableBlock` per-variant impls are static-dispatch. Re-verified at M3 W3 implementation.

## Lock surfaces

- `parking_lot::Mutex` allowed in short critical sections (ENTITY §8.7).
- `tokio::Mutex` forbidden in hot paths.
- Current lock count: 0 in `apps/server`.

## Async runtime

- Tokio multi-thread default. TPC (`monoio`/`glommio`) requires per-microservice ADR per ENTITY §3.13. Currently no TPC binary planned.

## Open positions

- **Allocator benchmark deferred:** mimalloc default; jemalloc fallback under feature flag (ADR-002 ratified). Comparative bench at M12 nightly when canonical workload exists.
- **simd-json hot-path adoption:** workspace dep present; integration sites depend on first body-parsing endpoint (M2 W3+).
- **`Bytes` body-zero-copy:** all current handlers return `&'static str` or sqlx scalars; first `Bytes` site lands when CMS pages stream HTML (M3 W3+).
