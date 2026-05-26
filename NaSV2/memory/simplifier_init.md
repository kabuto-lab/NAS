---
name: simplifier-init
description: Simplifier initial state at 2026-05-26 Adoption Pass — deletion graveyard, refused-additions log
metadata:
  type: project
---

# SIMPLIFIER — Init Dossier (2026-05-26 Adoption Pass)

## Deletion graveyard (things considered, removed before landing)

(empty — first session)

## Refused-additions log (things proposed, rejected pre-implementation)

(empty — first session)

## Active counter-proposals — binding for upcoming sessions

| Day | Counter-proposal | Verdict | Source |
|---|---|---|---|
| 2026-07-21 (M3 W1 D2 — ADR-010 finalization) | Remove Option C "graceful fallback" wording from §Decision Outcome. Reduce to a single ratified path; future scope-pressure opens an MPD, not a covert switch. | **binding** | Adoption-Pass Council review · `2026-07-20-mon/senior-dev.md §Simplifier Counterproposal` |

## Workspace surface watch-list

Surfaces the Simplifier monitors for unnecessary abstraction:

- `crates/extension-api/` — currently a stub. Will gain `BlockSchema` + `EditableBlock` traits at M9. **Watch for:** trait soup, over-generic bounds, unused associated types.
- `crates/theme-api/` — currently a stub. Will gain `Theme` + `Template` traits at M8. **Watch for:** premature plugin-hook surface.
- `crates/infrastructure/src/queue/` — currently `pgmq.rs` (126 LOC, real impl). **Watch for:** premature NATS abstraction before M10 measures actual fan-out load.
- `crates/runtime/src/supervisor.rs` — only sanctioned `tokio::spawn` site. **Hold the line:** no shortcuts, no second sanctioned spawn site.

## Anti-patterns watched

| Pattern | Status across codebase |
|---|---|
| Trait for a single call site | 0 occurrences (good) |
| Feature flag without deletion criterion | 0 occurrences (mimalloc/jemalloc has explicit criterion — bench-driven) |
| `Box<dyn T>` in hot path | 0 occurrences |
| Macro body > 50 lines | 0 occurrences (xtask `magic-check` enforces) |
| `lazy_static!` outside `crates/common/registry` | 0 occurrences |
| `unwrap_used` outside tests | clippy warns; M12 final sweep deny |

## Constitutional position

The Simplifier is required (per `CONSTITUTION.md §2.2`) to attempt reduction whenever Tier-1 reaches first-pass consensus. A null-attempt is acceptable only with documented reasoning.
