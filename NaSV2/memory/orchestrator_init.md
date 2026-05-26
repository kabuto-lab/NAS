---
name: orchestrator-init
description: Orchestrator initial state at 2026-05-26 Adoption Pass — roadmap snapshot, dependency graph, drift detectors armed
metadata:
  type: project
---

# ORCHESTRATOR — Init Dossier (2026-05-26 Adoption Pass)

## Active roadmap

- **Master plan version:** v1.0 (initial draft per 2026-05-25 AVTONOM bootstrap)
- **Source-of-truth files:**
  - `docs/session-plans/WP-PLAN-12-MONTH.html` (visual)
  - `docs/session-plans/MASTER-ROADMAP-2026-2027.md` (narrative)
  - `docs/session-plans/MONTH-SKELETON-{03..12}.md` (per-month seeds)
- **Cumulative WP-parity target at M12:** 100% WP-core
- **Year-2 candidates:** seeded by M12 RETRO; not part of v1.0 horizon

## Dependency graph (M3+)

| Day | Depends on | Status at adoption |
|---|---|---|
| 2026-07-20 (M3 W1 D1, ADR-010) | RFC-001 (block library) · ADR-009 (revisions) · `Block` 4-variant in domain | All **not yet present** at 2026-05-26 — armed for D-10 verification at execution |
| 2026-07-21 (M3 W1 D2) | ADR-010 Proposed → Accepted today | Pending Mon close |
| 2026-07-22..24 (M3 W1 D3-5) | Media model + repo + migrations 0009-0011 | Awaits M2 close |
| 2026-07-27..31 (M3 W2) | libvips system lib available; pgmq queue from M1 0001 | M1 0001 ✓ at HEAD |

## Drift detectors armed (D-1..D-10)

| Detector | Status | Notes |
|---|---|---|
| D-1 Scope | armed daily | T13 sweep per EXECUTION_PROTOCOL §7 |
| D-2 ADR drift | armed weekly | Friday EOD via Historian |
| D-3 Capability | green | `xtask capability-coverage` stub exists; real impl M1 W4 D2 |
| D-4 Bench | dormant until M12 nightly | Manual until then |
| D-5 Pool mode | green | apps/server/main.rs enforces at boot |
| D-6 Planning refs | green | `xtask check-planning-refs` stub |
| D-7 Architecture | green | `xtask architecture-check` real impl |
| D-8 Forecast | armed monthly | RETRO day sweep |
| D-9 Decision-graph | armed weekly | Historian Friday |
| D-10 Memory | armed at session-start | T1 read-before-trust per EXECUTION_PROTOCOL §2 |

## Open positions

- **ADR-010 forward bindings unresolved at adoption:** the chosen substrate (Option B Leptos block editor) implies static-dispatch + per-island lazy hydration. Constraints captured in pilot day's `senior-dev.md §Forgemaster Memo`. Re-engage M3 W3 + M7 W3.
- **MONTH-SKELETON-03 §Assumed entering state** lists "ADR-009 written" — must verify at 2026-07-20 execution; CARRY-OVER repair if missing.
- **Plugin SDK shape pre-commitment:** ADR-010 §Consequences pre-binds the M9 ADR-014 surface. RFC-009 to open M9 W1 D1.

## Inheritance map (write-back from architect briefs)

- M3 W1 D1 → M4 W1, M7 W3, M9 W2, M12 W1 (per pilot architect.md §7).
- M3 G2 (Media model) → M6 (comment avatars), M7 (media browser), M8 (theme assets), M10 (cache invalidation), M11 (search alt-text).
- M3 G3 (libvips workers) → first non-HTTP TaskCategory; precedent for M11 (search reindex) + M9 (plugin instances).

## Outstanding architect-level questions

(none open at adoption — all delegated decisions resolved in pilot)
