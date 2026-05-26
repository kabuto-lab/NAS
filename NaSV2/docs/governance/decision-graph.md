# Decision Graph — AX•CMS

> **Canonical store for all ratified architectural decisions and their lineage.**
> Written and maintained by the **HISTORIAN** entity per `ENTITY_SYSTEM.md §5`.
> Format: append-only nodes; `Supersedes` / `Consulted` / `Forward-binds` edges.

---

## Nodes — Ratified ADRs (as of 2026-05-26 Adoption Pass)

### ADR-002 · Allocator: mimalloc default + jemalloc fallback
- **Status:** Accepted
- **Date:** 2026-05-25
- **Consulted:** ENTITY.md §3.12
- **Forward-binds:** every binary in `apps/`
- **Superseded-by:** —

### ADR-003 · Pool isolation (http / worker / admin)
- **Status:** Accepted
- **Date:** 2026-05-25
- **Consulted:** ENTITY.md §3.4.2
- **Forward-binds:** `apps/server/src/main.rs` boot contract; PgBouncer dev config; production deploy
- **Superseded-by:** —

### ADR-004 · Image pipeline: libvips workers
- **Status:** Accepted
- **Date:** 2026-05-25
- **Consulted:** ENTITY.md §3.7, §9.2
- **Forward-binds:** `crates/image-pipeline/` impl (M3 W2); ADR-011 (storage adapter, M3 W1 D2)
- **Superseded-by:** —

### ADR-005 · Queue split: pgmq (transactional) + NATS JetStream (volume)
- **Status:** Accepted
- **Date:** 2026-05-25
- **Consulted:** ENTITY.md §3.8
- **Forward-binds:** M3 W2 (image jobs); M10 W3 (cache invalidation fan-out); M11 W2 (search reindex)
- **Superseded-by:** —

### ADR-006 · Multi-level cache (L1 moka + L2 Dragonfly + L3 edge)
- **Status:** Accepted
- **Date:** 2026-05-25
- **Consulted:** ENTITY.md §3.9
- **Forward-binds:** M10 W1-W3; ADR-015 (edge invalidation, M10 W3 D4); cache key composition (tenant + slug + capability_hash + theme_hash)
- **Superseded-by:** —

---

## Anticipated nodes — ADRs planned but not ratified

Each entry records the *anticipated* node so D-9 (decision-graph drift) can detect divergence. Status: `anticipated`. Promoted to ratified node after the ADR commits.

### ADR-001 · (slot reserved — backfill M1 W4 D3)
- **Status:** anticipated
- **Anticipated date:** 2026-06-17

### ADR-007..008 · (slots reserved — M1 W4 backfill)
- **Status:** anticipated

### ADR-009 · Revisions concept
- **Status:** anticipated
- **Anticipated date:** 2026-07-16 (M2 W4 D4)
- **Forward-binds:** ADR-010 §Consequences §Inherited by M3 W4 (autosave protocol)

### ADR-010 · Block-editor frontend strategy ★ pilot
- **Status:** anticipated — Proposed on 2026-07-20 (Mon), Accepted on 2026-07-21 (Tue)
- **Anticipated date:** 2026-07-21
- **Consulted:** RFC-001 (block library); ADR-009 (revisions)
- **Forward-binds:** ADR-013 (M8 theme rendering); ADR-014 (M9 WASM sandbox); RFC-009 (M9 plugin SDK)
- **Adoption-Pass note (2026-05-26):** Council pre-reviewed today. Binding outcomes captured in `dailies-v2/M03-2026-07/W1-2026-07-20/2026-07-20-mon/{architect,senior-dev}.md`. Simplifier counterproposal binding for Tue ratification: Option C "graceful fallback" wording removed from §Decision Outcome.

### ADR-011 · Media storage adapter contract
- **Status:** anticipated
- **Anticipated date:** 2026-07-21

### ADR-012..022 (rolling)
- See `WP-PLAN-12-MONTH.html` M3-M11 cells.

---

## Edges — current

| From | Edge | To | Notes |
|---|---|---|---|
| ADR-004 | Forward-binds | ADR-011 (anticipated) | image pipeline needs storage |
| ADR-005 | Forward-binds | ADR-015 (anticipated) | NATS for edge fan-out |
| ADR-006 | Forward-binds | ADR-015 (anticipated) | L3 cache shape |
| ADR-009 (anticipated) | Forward-binds | ADR-010 (anticipated) | autosave protocol |
| ADR-010 (anticipated) | Forward-binds | ADR-013 (anticipated) | theme rendering shape |
| ADR-010 (anticipated) | Forward-binds | ADR-014 (anticipated) | WASM sandbox + plugin block contract |
| ADR-010 (anticipated) | Forward-binds | RFC-009 (anticipated) | plugin SDK shape |

No `Supersedes` edges yet. No `Contradicts` edges (would indicate D-9 drift).

---

## RFCs

### RFC-002 · Edge architecture
- **Status:** Draft (2026-05-25)

### Anticipated RFCs

| Slot | Title | Anticipated date |
|---|---|---|
| RFC-001 | Block library (4 → 19 variants) | M1 W4 D3 backfill |
| RFC-003, RFC-004 | (reserved, M1 W4 backfill) | 2026-06-17 |
| RFC-005 | CSRF / nonce surface | M2 W4 D4 |
| RFC-006 | JWT verifier | M2 W4 D4 |
| RFC-007 | Media model | 2026-07-21 |
| RFC-008 | OpenAPI versioning policy | 2026-09-10 |
| RFC-009 | Plugin SDK shape | 2027-01-04 |

---

## Integrity invariants

- Every ratified ADR has a unique slot number.
- Every ratified ADR has `Status: Accepted` and a date.
- No two ratified ADRs contradict in `Consequences` sections (Historian D-2 weekly sweep enforces).
- No ratified ADR depends on a *future* ratified ADR (forward-binds are directional).
- Every `Forward-binds` edge resolves within the 12-month horizon.

---

## Update cadence

- **Daily** — at session-close, the Historian updates this file if any ADR/RFC ratified or anticipated.
- **Weekly** — Friday EOD, Historian runs D-2 / D-9 sweeps across the graph.
- **Monthly** — RETRO day, Historian produces a §decision-graph-delta block in the RETRO doc.

---

**Initialized at:** 2026-05-26 Adoption Pass (governance v1.0 ratification).
