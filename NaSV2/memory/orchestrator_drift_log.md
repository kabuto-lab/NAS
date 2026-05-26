---
name: orchestrator-drift-log
description: Append-only log of drift detector trips (D-1..D-10) across the 12-month roadmap
metadata:
  type: project
---

# Drift Log — Orchestrator (append-only)

> Format per entry:
> `YYYY-MM-DD HH:MM  D-<N>  <severity>  <fact-summary>  <suggested-repair>`
>
> Severities: `info` (caught & repaired same session), `warn` (carried over),
> `critical` (escalated to operator).
> Detector definitions: `docs/governance/CONSTITUTION.md §6` + `ROADMAP_ENGINE.md §5`.

## Log

2026-05-26 23:00  D-8  info  Pilot day 2026-07-20 architect.md depends on RFC-001 + ADR-009 + Block 4-variant state — NONE present at adoption (M1 W4 + M2 W4 not yet run). Armed for D-10 verification at execution time. Repair: senior-dev.md P1 slot probe handles this if files missing.
2026-05-26 23:00  D-10  info  Memory dossiers created from cold; no prior memory to contradict. Read-before-trust will engage from next session.
