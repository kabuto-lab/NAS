# ROADMAP ENGINE — How the 12-Month Plan is Consumed and Evolved

> **Status:** binding · v1.0 · 2026-05-26
> **Authority:** subordinate to `CONSTITUTION.md`, `ENTITY_SYSTEM.md`, `EXECUTION_PROTOCOL.md`.
> **Purpose:** define how the master plan (`WP-PLAN-12-MONTH.html` + `MASTER-ROADMAP-2026-2027.md`) is consumed by the Council, how it is allowed to evolve, and how drift between plan and reality is detected and repaired.

---

## §0 · The Three Views of the Roadmap

The roadmap is not a single document; it is **three views** of one underlying state:

| View | File(s) | Authority | Mutability |
|---|---|---|---|
| **PLANNED** | `WP-PLAN-12-MONTH.html` (visual) + `MASTER-ROADMAP-2026-2027.md` (narrative) + `MONTH-SKELETON-{03..12}.md` (per-month seeds) | binding for *intent*; not binding for *exact dates* | mutable at RETROs (§4) |
| **EXECUTING** | `dailies-v2/MNN-YYYY-MM/...` (architect.md + senior-dev.md per day) | binding for *today*; reflects PLANNED + Council ratifications | mutable up to ratification, frozen at `ratified`, archived to `_superseded/` at `executed` |
| **EXECUTED** | git history + `SESSION_LOG.md` (root) + `docs/governance/decision-graph.md` + per-month `RETRO-YYYY-MM.md` | binding for *reality* — the source of truth about what actually happened | append-only |

These three views must reconcile. The reconciliation engine **is** the Council loop (§EXECUTION_PROTOCOL).

---

## §1 · Inputs to the Engine

At any given session, the engine reads:

1. **PLANNED inputs**
   - `WP-PLAN-12-MONTH.html` — visual cell for today (master-plan row).
   - `MASTER-ROADMAP-2026-2027.md` §master-plan + §risk-register + §year-2-candidates.
   - `MONTH-SKELETON-{current month}.md` — the month's G1–G5 goals.
   - `MONTH-SKELETON-{next month}.md` — to verify forward-inheritance.

2. **EXECUTING inputs**
   - `dailies-v2/INDEX.md` — today's status row.
   - `dailies-v2/MNN-.../<today>/architect.md` + `senior-dev.md` — today's plan.

3. **EXECUTED inputs**
   - `git log` (last 14 days) — commits actually landed.
   - `SESSION_LOG.md` (yesterday) — what carry-over today inherits.
   - `docs/governance/decision-graph.md` — ratified ADRs and their lineage.
   - `docs/governance/CHANGELOG.md` — constitutional amendments.

4. **Memory inputs** (per entity per `ENTITY_SYSTEM.md §17`)
   - `memory/MEMORY.md` index.
   - Each entity's own memory dossier.
   - `memory/project_next_day_plan.md` if present.

5. **Reality probes** (live, not cached)
   - `Grep` / `Glob` over the repo.
   - `cargo check / clippy / test / xtask <gate>` results.
   - `nas2-cli db migrate --check` output.
   - `/health/pool` (when server is running).

---

## §2 · The Drafting Pipeline (planned → drafted)

The transition from a master-plan cell into a `drafted` daily artifact:

```
WP-PLAN cell      →   architect.md (Orchestrator + Historian)
        │
        ▼
MONTH-SKELETON G  →   senior-dev.md (Forgemaster + Sentinel)
        │
        ▼
prior carry-over  →   architect.md §CARRY-OVER, senior-dev.md §CARRY-OVER
        │
        ▼
Council Tier-1    →   Forgemaster Memo, Sentinel Audit, Simplifier counter
        │
        ▼
Council Tier-2    →   Historian Trace, Economist Ledger
        │
        ▼
Council Tier-3/4  →   (per activation matrix)
        │
        ▼
Conflict?         →   Judge if any
        │
        ▼
ratify            →   INDEX.md status `drafted` (pre-Adoption) /
                       `ratified` (post-Council-pass)
```

Drafting is the only stage where the day's *content* can change. After ratification, the artifact is read-only (excepting Council-recorded amendments per `CONSTITUTION.md §11`).

---

## §3 · The Execution Pipeline (ratified → executed)

The day runs:

```
T11 ratified daily artifact
        │
        ▼
AVTONOM session executes senior-dev.md
   ▸ V1..V4 baseline
   ▸ P0..PN phases
   ▸ commit(s) on main, no push
   ▸ SESSION_LOG.md written
        │
        ▼
T12 session-end ritual
   ▸ xtask gates green
   ▸ INDEX.md row → `executed`
   ▸ memory deltas saved
        │
        ▼
T13 anti-drift sweep
   ▸ D-1..D-10 detectors
   ▸ flagged drifts logged
        │
        ▼
git HEAD == EXECUTED truth
```

Execution is **never amended retroactively**. If today went wrong, yesterday's `SESSION_LOG` and `git log` remain truthful; the repair lands in tomorrow's plan.

---

## §4 · The Evolution Pipeline (RETRO → next-month bootstrap → master-plan diff)

The master plan **evolves**. Evolution is allowed only through the formal pipeline:

### §4.1 — Monthly RETRO

Last working day of every month (e.g. 2026-06-19, 2026-07-17, 2026-08-14, …):

```
RETRO day (W4 D5 of each month)
   ▸ Council full activation (Tier-1 + Tier-2 + Tier-3 + Tier-4)
   ▸ EXECUTION_PROTOCOL §7 monthly anti-drift sweep runs
   ▸ Output: RETRO-YYYY-MM.md
       §summary-of-month
       §goals-shipped vs goals-planned
       §AI-defaults-applied
       §drift-audit (all D-1..D-10 detectors)
       §carry-over to next month
       §master-plan-diff (if any)
       §migrator-outlook (Tier-4)
       §ecosystem-outlook (Tier-4)
       §productor-notes (Tier-4)
   ▸ Operator OK required for any §master-plan-diff
   ▸ Next-month bootstrap document drafted: avtonom-month-bootstrap-YYYY-MM+1.md
```

### §4.2 — Master-plan diff process

If a RETRO proposes to change the master plan:

1. **Diff document** — `docs/governance/master-plan-diffs/MPD-NNN-<slug>.md`:
   - Source RETRO.
   - Original master-plan cell text (verbatim from `WP-PLAN-12-MONTH.html`).
   - Proposed new cell text.
   - Rationale (≥ 200 words; cite ≥ 2 affected future months).
   - Council verdicts (all activated tiers).
   - Cost impact (Economist).
   - Risk impact (Sentinel).
   - Migration impact (Migrator).

2. **Operator OK** — explicit human sign-off line.

3. **Master-plan edit** — `WP-PLAN-12-MONTH.html` cell updated + `MASTER-ROADMAP-2026-2027.md` updated + impacted `MONTH-SKELETON-NN.md` updated.

4. **Commit** — single commit, trailer `Master-Plan-Diff: MPD-NNN`, no push without operator instruction.

5. **Roll-forward** — INDEX.md re-stubs affected future days (mass-edit from `planned` to `re-planned`).

### §4.3 — What evolution is forbidden

- Silently editing `WP-PLAN-12-MONTH.html` outside a RETRO + MPD.
- Reordering months (M3 cannot become M5 without breaking dependency graph; if the case arises, motion to operator).
- Reducing scope below 80 % WP-parity by M12 without explicit operator OK (the §1 mission of `ENTITY.md` is at stake).
- Reopening §4 Immutables of `CONSTITUTION.md` without §11 amendment.
- Adding speculative months 13+ (Year-2 is seeded by M12 RETRO, not Y1 mid-flight).

---

## §5 · Drift Detectors (formal definitions)

The ten drift patterns from `CONSTITUTION.md §6` get **formal definitions** here so detectors can be implemented:

| ID | Detector contract |
|---|---|
| **D-1 Scope** | `git diff --stat <yesterday-SHA>..HEAD -- 'crates/**' 'apps/**' 'migrations/**'` reports `>= 1.3 × ` the LOC budget implied by `WP-PLAN-12-MONTH.html` cell scope text (heuristic: cell text < 100 chars ⇒ budget ≤ 400 LOC; cell text < 200 chars ⇒ budget ≤ 800 LOC; else ≤ 1500 LOC). Threshold trip = D-1 flag. |
| **D-2 ADR drift** | For every ADR with `Status: Accepted`, every commit landed since the ADR ratification is checked against the ADR's `Consequences` section. Violations are flagged. Implementation: weekly cron in `docs/governance/scripts/` (future). |
| **D-3 Capability** | `cargo xtask capability-coverage` exit code != 0. |
| **D-4 Bench** | `cargo bench --bench <hot-path>` p95 > prior week's p95 + 5 %. (Nightly CI gate at M12; manual until then.) |
| **D-5 Pool mode** | `SHOW pool_mode` returns anything but `transaction` on http_pool / worker_pool / admin_pool. |
| **D-6 Planning trail** | `cargo xtask check-planning-refs` exit code != 0. |
| **D-7 Architecture** | `cargo xtask architecture-check` exit code != 0. |
| **D-8 Forecast** | Diff: `MONTH-SKELETON-{next}.md §Assumed entering state` ↔ current code state (grep for named files / functions / aggregates). Any mismatch = D-8 flag. |
| **D-9 Decision-graph** | New ADR contradicts a prior ADR (Historian compares §Consequences). |
| **D-10 Memory** | Memory file fact ≠ live state (read-before-trust at T1 fails). |

Each detector writes a line to `memory/orchestrator_drift_log.md`:

```
YYYY-MM-DD HH:MM  D-<N>  <severity>  <fact-summary>  <suggested-repair>
```

---

## §6 · Re-Planning Triggers

When does the engine **force** a master-plan revision rather than wait for the next RETRO?

| Trigger | Action |
|---|---|
| **Two D-1 (scope) trips in one week** | Schedule mid-month RETRO; operator OK required; new MPD if scope adjustment ratified. |
| **Any D-2 (ADR drift) on a ratified Accepted ADR** | Stop normal work; emergency ADR-supersede pass; resume only after `decision-graph.md` reconciles. |
| **Any D-5 (pool mode)** | Production-grade incident even in dev; Sentinel takes lead; root cause must be named before next AVTONOM. |
| **Bench regression > 15 % on a §7 hard target** | Open PERF-NNN; M-current goals shed feature work for perf-restore week. |
| **Operator declares re-planning** | Direct master-plan amendment; bypass normal RETRO cadence. |

A re-planning trigger fires a **§4.2 MPD process** with `Source: re-planning-trigger-<id>` instead of `Source: RETRO-YYYY-MM`.

---

## §7 · Master-Plan Versioning

`WP-PLAN-12-MONTH.html` and `MASTER-ROADMAP-2026-2027.md` are versioned in the following way:

| Version | Source-of-change | Notation |
|---|---|---|
| `v1.0` | Initial draft (2026-05-25 AVTONOM bootstrap) | header line in both files |
| `v1.N` | Monthly RETRO MPD ratifications | `v1.<count of MPDs to date>` |
| `v2.0` | M12 RETRO + Year-2 candidates promotion | new file `MASTER-ROADMAP-2027-2028.md` (Year-2 doesn't replace Year-1; it succeeds it) |

Versioning is **monotonic**. A v1.5 cannot become v1.4 — any rollback creates v1.6 that mirrors v1.4 content.

A versioning bump is part of the §4.2 MPD commit (same commit; trailer carries both `Master-Plan-Diff: MPD-NNN` and `Master-Plan-Version: v1.<N>`).

---

## §8 · Linkage Map

How each governance file participates in the engine:

```
                  ┌─────────────────────────────────┐
                  │     OPERATOR (sovereign)        │
                  └─────────────┬───────────────────┘
                                │
              ┌─────────────────┴──────────────────┐
              │                                    │
              ▼                                    ▼
  ┌──────────────────────┐           ┌─────────────────────────┐
  │   ENTITY.md          │           │   CONSTITUTION.md       │
  │   (platform code     │  binds    │   (entity governance    │
  │    constitution)     │ ─────────►│    constitution)        │
  └─────────┬────────────┘           └─────────┬───────────────┘
            │                                  │
            │                                  ▼
            │                        ┌─────────────────────────┐
            │                        │   ENTITY_SYSTEM.md      │
            │                        │   (14 minds dossiers)   │
            │                        └─────────┬───────────────┘
            │                                  │
            │                                  ▼
            │                        ┌─────────────────────────┐
            │                        │  EXECUTION_PROTOCOL.md  │
            │                        │  (daily Council loop)   │
            │                        └─────────┬───────────────┘
            │                                  │
            │                                  ▼
            │                        ┌─────────────────────────┐
            └──────────►─────────────│   ROADMAP_ENGINE.md     │
                                     │   (this file —          │
                                     │    plan/execute/evolve) │
                                     └─────────┬───────────────┘
                                               │
                ┌──────────────────────────────┼──────────────────────────────┐
                │                              │                              │
                ▼                              ▼                              ▼
  ┌──────────────────────┐    ┌──────────────────────────┐    ┌──────────────────────┐
  │  WP-PLAN-12-MONTH    │    │  MONTH-SKELETON-{NN}.md  │    │  RETRO-YYYY-MM.md    │
  │  (visual master)     │    │  (per-month seeds)       │    │  (per-month closing) │
  └──────────┬───────────┘    └────────┬─────────────────┘    └────────────┬─────────┘
             │                         │                                   │
             └─────────────────┬───────┴───────────────────────────────────┘
                               │
                               ▼
                  ┌────────────────────────────────────┐
                  │   dailies-v2/                      │
                  │     MNN-YYYY-MM/                   │
                  │       WN-YYYY-MM-DD/               │
                  │         YYYY-MM-DD-dow/            │
                  │           architect.md             │
                  │           senior-dev.md            │
                  │   INDEX.md (status table)          │
                  └─────────────────┬──────────────────┘
                                    │
                                    ▼
                  ┌────────────────────────────────────┐
                  │   git HEAD + SESSION_LOG.md +      │
                  │   decision-graph.md                │
                  │   = EXECUTED truth                 │
                  └────────────────────────────────────┘
```

Every arrow is a *binding* relation. A change at any node must propagate or be rejected.

---

## §9 · Worked example — what happens on 2026-07-20 (M3 W1 D1, the pilot day)

1. **PLANNED inputs at session start:**
   - `WP-PLAN-12-MONTH.html` M3 W1 row 1 cell text — verbatim quoted in `architect.md §1`.
   - `MONTH-SKELETON-03.md §Goals · G1` — quoted in `architect.md §2`.
   - `MASTER-ROADMAP-2026-2027.md` M3 row — verifies parity arc.

2. **EXECUTING inputs:**
   - `dailies-v2/INDEX.md` row 2026-07-20 status = `drafted` (from the 2026-05-26 pilot session).
   - `dailies-v2/M03-2026-07/W1-2026-07-20/2026-07-20-mon/architect.md` and `senior-dev.md` exist.

3. **EXECUTED inputs:**
   - `git log -1 --format=%H` — HEAD on `main` is the 2026-07-17 RETRO commit.
   - `SESSION_LOG.md` (yesterday) — confirms M2 W4 D5 RETRO clean close.
   - `decision-graph.md` — ADR-009 ratified; ADR-010 not yet present.

4. **Reality probes:**
   - `Glob docs/adr/ADR-010-*` — empty (slot free).
   - `Grep enum Block crates/domain/src/` — 4 variants (M1 state) confirmed.

5. **Council pass:**
   - Orchestrator confirms master-plan alignment.
   - Historian confirms no contradiction with ADR-009 / RFC-001.
   - Forgemaster: docs-only day, allocation budget N/A; emits skip-with-reason.
   - Sentinel: ADR is a *risk reduction* artifact; flags drift risk if ADR-010 is left vague (mitigation: §A-9 anti-laziness check).
   - Simplifier: option set might be reducible — Option C is borderline; verdict: keep, with deletion criterion (if M3 W3 evidence supports, delete C from M4 onwards).
   - Economist: $ delta = 0; complexity delta tracked.
   - Tier-3: NOT activated (docs-only).
   - Tier-4: NOT activated (not RETRO, not public-API).

6. **Conflict?** None.

7. **Ratify** → INDEX.md status `drafted` → on adoption pass `ratified`.

8. **Execute** (T11) — the AVTONOM session writes `ADR-010-block-editor-strategy.md`, commits.

9. **Anti-drift sweep:**
   - D-1: docs-only, scope OK.
   - D-6: commit references `ADR-010` — check-planning-refs green.
   - D-10: memory facts verified at T1.

10. **Close** — INDEX.md row → `executed`; SESSION_LOG written; carry-over for 2026-07-21 prepared.

This worked example is the *paradigm* for every day. M3 D1 is light (docs only); a heavy day (e.g. M3 W2 D5 libvips integration test) activates more entities.

---

## §10 · Anti-fragility

The engine is **anti-fragile**: each drift detected and repaired makes the next month's plan *more* accurate, not less. RETROs that ratify MPDs are a feature, not a failure.

What is **not** anti-fragile:

- Silent drift (not detected) — corrosive.
- Drift detected but not logged — pretends-not-to-exist.
- Drift logged but never repaired — becomes background radiation.

The Council's job is to convert drift → repair within ≤ 1 week of detection (D-2, D-9), within the day (D-1, D-3, D-5, D-6, D-7), or at the next RETRO (D-4, D-8, D-10).

---

## §11 · The Three Failure Modes of Roadmap Engines

Historical roadmap engines fail in three predictable ways. The Council is structured to resist each:

### §11.1 — Plan-vs-reality blindness

**Symptom:** The plan says "M3 ships media pipeline"; reality at M3 EOD is "media model only; pipeline slipped". The team continues planning M4 assuming media pipeline shipped.

**Mitigation:** `D-8 Forecast drift` detector + monthly RETRO §master-plan-diff + Historian's decision-graph reconciliation.

### §11.2 — Sunk-cost preservation

**Symptom:** A subsystem was planned in M5; by M7 it's clear it shouldn't ship; but it's "already 60 % done" so the team finishes.

**Mitigation:** Economist's cost-curve at every RETRO; Migrator's rewrite-probability at every monthly outlook; Operator-OK required for `complete sunk` decisions.

### §11.3 — Roadmap calcification

**Symptom:** The plan is so detailed (e.g. 240 pre-written daily prompts) that the team stops thinking and just executes; no evolution; reality diverges silently.

**Mitigation:** The plan is **evolutive**, not prescriptive. Every monthly RETRO is expected to produce ≥ 1 MPD (zero MPDs in a RETRO is itself a flag — Historian raises it). The pre-generated dailies are *forecasts*, not commitments.

---

## §12 · Closing directive

This engine exists to **convert intent (the master plan) into reality (the codebase) without losing coherence over 12 months**.

It does this by:

- Making the plan **visible** (three views, all reconciled).
- Making the plan **mutable but disciplined** (RETRO + MPD pipeline, never silent).
- Making the plan **observable** (10 drift detectors, written to memory).
- Making the plan **self-correcting** (every RETRO ratifies amendments).

If the engine ever feels like overhead, two questions:

1. **Is today's work touching the plan?** If yes, the engine is doing its job. If no, the engine should be invisible.
2. **Is the engine paying for itself?** If at month 6 the cumulative drift is < 5 % of planned scope, yes. If it's > 20 %, the Council itself needs a Tier-5 review.

The engine is **not** the plan. The engine is **the system that keeps the plan honest**.

---

**End of Roadmap Engine.**
The four governance documents are now complete:

| File | Question answered |
|---|---|
| `CONSTITUTION.md` | What are the laws? |
| `ENTITY_SYSTEM.md` | Who applies the laws? |
| `EXECUTION_PROTOCOL.md` | How do they apply the laws daily? |
| `ROADMAP_ENGINE.md` | How does the law-applying system stay aligned with the 12-month plan? |

Read in order: Constitution → Entity System → Execution Protocol → Roadmap Engine.
