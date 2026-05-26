---
name: historian-init
description: Historian initial state at 2026-05-26 Adoption Pass — decision-graph state, ADR aging, prior-rejection log
metadata:
  type: project
---

# HISTORIAN — Init Dossier (2026-05-26 Adoption Pass)

## Decision-graph state

Canonical file: `docs/governance/decision-graph.md` (created at this Adoption Pass).

### Ratified ADRs (per `docs/adr/`)

| Slot | Title | Status | Date | Supersedes | Superseded by |
|---|---|---|---|---|---|
| ADR-002 | Allocator: mimalloc default + jemalloc fallback | Accepted | 2026-05-25 | — | — |
| ADR-003 | Pool isolation (http/worker/admin) | Accepted | 2026-05-25 | — | — |
| ADR-004 | Image pipeline: libvips workers | Accepted | 2026-05-25 | — | — |
| ADR-005 | Queue split: pgmq (transactional) + NATS JetStream (volume) | Accepted | 2026-05-25 | — | — |
| ADR-006 | Multi-level cache (L1 moka + L2 Dragonfly + L3 edge) | Accepted | 2026-05-25 | — | — |

### Anticipated ADRs (planned but not ratified)

| Slot | Title | Anticipated date | Anticipated source |
|---|---|---|---|
| ADR-001 | (slot reserved — not yet authored; check at M1 W4 D3 backfill) | 2026-06-17 | M1 W4 D3 backfill day |
| ADR-007..008 | (reserved for M1 W4 backfill) | 2026-06-17 | M1 W4 D3 |
| ADR-009 | Revisions concept | 2026-07-16 | M2 W4 D4 |
| **ADR-010** | **Block-editor strategy** | **2026-07-21** | **M3 W1 D2 (finalization day; option study on Mon)** |
| ADR-011 | Media storage adapter contract | 2026-07-21 | M3 W1 D2 |
| ADR-012 | Virus-scan hook (optional) | M3 W3 D2 (~2026-08-04) | M3 W3 |
| ADR-013 | Theme rendering substrate | 2026-12-07 | M8 W1 D1 |
| ADR-014 | WASM sandbox vs compile-time linkage | 2027-01-04 | M9 W1 D1 |
| ADR-015 | Edge cache invalidation mechanism | 2027-02-18 | M10 W3 D4 |
| ADR-016..022 | various (see WP-PLAN-12-MONTH M4-M11 cells) | 2026-08-31 onward | rolling |

### Ratified RFCs (per `docs/rfc/`)

| Slot | Title | Status | Date |
|---|---|---|---|
| RFC-002 | Edge architecture | Draft | 2026-05-25 |

### Anticipated RFCs

| Slot | Title | Anticipated date |
|---|---|---|
| RFC-001 | Block library (4 → 19 variants) | M1 W4 D3 backfill |
| RFC-003 | (reserved) | M1 W4 |
| RFC-004 | (reserved) | M1 W4 |
| RFC-005 | CSRF / nonce surface | M2 W4 D4 |
| RFC-006 | JWT verifier | M2 W4 D4 |
| RFC-007 | Media model | 2026-07-21 |
| RFC-008 | OpenAPI versioning policy | 2026-09-10 |
| RFC-009 | Plugin SDK shape | 2027-01-04 |

### Ratified PLANs / VALs / SECs

PLAN-001..004 ratified (libvips / tantivy / pgbouncer / pgmq).
VAL-002..004 ratified (pool-mode / pool-isolation / supervisor).
SEC-* unknown until M1 W4 D4 (perf baseline + SEC backfill).

## Prior-rejection log

(empty — first session; no option has yet been formally rejected at ADR scope)

## ADR aging audit

(empty — no `Status: Proposed` ADRs older than 7 days)

## Decision-graph integrity

- Edges: 0 `Supersedes` (no ADR supersedes another).
- Edges: 0 `Contradicts` (no D-9 drift recorded).
- All ratified ADRs are mutually consistent (verified at Adoption Pass).

## Constitutional position

The Historian writes the canonical `docs/governance/decision-graph.md`. Every new ADR + every RETRO updates it. The Historian is the anti-amnesia module of the Council.
