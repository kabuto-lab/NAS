# EXECUTION PROTOCOL — The Daily Council Loop

> **Status:** binding · v1.0 · 2026-05-26
> **Authority:** subordinate to `CONSTITUTION.md` and `ENTITY_SYSTEM.md`.
> **Purpose:** define the operational sequence by which the 14 entities cooperate to produce a ratified roadmap day, run an AVTONOM session, and close it with verifiable artifacts.

---

## §0 · The Daily Council Loop — overview

```
┌─────────────────────────────────────────────────────────────────┐
│   T0   SESSION-START RITUAL                                     │
│        ↓                                                        │
│   T1   READ-BEFORE-TRUST PASS  (verify entering state)          │
│        ↓                                                        │
│   T2   ORCHESTRATOR draft     (architect.md skeleton)           │
│        ↓                                                        │
│   T3   HISTORIAN trace        (decision-graph consistency)      │
│        ↓                                                        │
│   T4   FORGEMASTER memo       (senior-dev.md skeleton)          │
│        ↓                                                        │
│   T5   SENTINEL audit         (failure modes named)             │
│        ↓                                                        │
│   T6   SIMPLIFIER counter     (try to reduce)                   │
│        ↓                                                        │
│   T7   ECONOMIST ledger       (cost math)                       │
│        ↓                                                        │
│   T8   TIER-3 (when activated)                                  │
│           ▸ Adversary stress / Chaos drills / Test Pilot profile│
│        ↓                                                        │
│   T9   TIER-4 (monthly RETRO or trigger)                        │
│           ▸ Migrator / Ecosystem / Productor                    │
│        ↓                                                        │
│   T10  CONFLICT DETECT                                          │
│         ▸ if disagreement → JUDGE (§6)                          │
│         ▸ else → ratify draft                                   │
│        ↓                                                        │
│   T11  EXECUTE  (the AVTONOM session runs senior-dev.md)        │
│        ↓                                                        │
│   T12  SESSION-END RITUAL                                       │
│        ↓                                                        │
│   T13  ANTI-DRIFT SWEEP  (D-1..D-10 detectors run)              │
└─────────────────────────────────────────────────────────────────┘
```

T0–T10 produces a `ratified` daily artifact set. T11 is the execution run. T12–T13 close the day.

---

## §1 · Session-Start Ritual (T0)

Every AVTONOM session, the first ten things in order — no exceptions:

1. **Read `ENTITY.md` §1** (the AX•ARCHITECT persona). Persona binds for the session.
2. **Read `docs/governance/CONSTITUTION.md` §0 + §3 + §4** (authority, priority ladder, immutables).
3. **Read `docs/governance/ENTITY_SYSTEM.md` §14** (activation matrix — determines which Tier-3/4 entities are required today).
4. **Read `memory/MEMORY.md`** — load shared memory index.
5. **Read each Tier-1 entity's most recent memory file** (`memory/orchestrator_*.md`, `memory/forgemaster_*.md`, `memory/sentinel_*.md`) — entities recover state.
6. **Read `dailies-v2/INDEX.md`** — find today's row.
7. **Read today's `architect.md`** + sibling `senior-dev.md` from the v2 tree.
8. **Read the prior day's `SESSION_LOG.md`** — surface CARRY-OVER items.
9. **Emit the first-line status** per `CLAUDE.md` §S:
   ```
   [mode:MANUAL|SEMIAUTO|AVTONOM] phase:<name> epic:<id> spine:<clear|pending>
   ```
10. **Run V1..V4** (`cargo check / fmt --check / clippy / test --lib`) — establish a green baseline.

If any of 1-10 fails, the session opens in **repair mode**: today's planned work is suspended; the failure is fixed first; SESSION_LOG records the substitution.

---

## §2 · Read-Before-Trust Pass (T1)

Before any Council entity reasons about today's work, the following claims in today's `architect.md` and `senior-dev.md` are **verified live**:

| Claim type | Verification |
|---|---|
| "File X exists" | `Glob` or `Read` |
| "Function/type Y is defined" | `Grep -n` |
| "Test Z passes" | `cargo test --lib <pat>` |
| "Migration N is applied" | `nas2-cli db migrate --check` or test fixture |
| "ADR M is ratified" | `Read docs/adr/ADR-M-*.md`, check `Status: Accepted` |
| "Yesterday closed at SHA …" | `git log -1 --format=%H` |
| "WP-parity is N %" | `Read MASTER-ROADMAP-2026-2027.md §parity-arc` (truth is the roadmap, not memory) |

A claim that **fails verification** is repaired in the daily prompt before T2. The Historian records "memory drift D-10 repaired at T1" in their trace.

---

## §3 · The Architect Pass (T2–T3)

**Orchestrator** drafts (or refreshes) `architect.md`. Mandatory sections per `dailies-v2/README.md`:

1. Master-plan cross-ref (verbatim cell).
2. Why today matters (TLA L1-L4).
3. Inputs.
4. Outputs.
5. Success criteria.
6. Spine impact.
7. Forward inheritance.
8. Risk register.
9. Decision delegation.

Then **Historian** appends `### HISTORIAN TRACE` (§ENTITY_SYSTEM 5):

- Decision-graph delta (nodes added; edges added).
- Prior-rejection check (was any considered option already rejected? cite the ADR).
- Proposed-ADR aging audit (any `Status: Proposed` older than 7 days?).

If Historian flags a contradiction with a prior decision, the day **does not proceed** — the contradiction is either repaired (today's plan adjusted) or escalated (Motion to Constitution §11).

---

## §4 · The Engineer Pass (T4–T7)

**Forgemaster** drafts (or refreshes) `senior-dev.md`. Mandatory sections per `dailies-v2/README.md` (CARRY-OVER, CONTEXT, SCOPE P0..PN, code skeletons, tests, pitfalls, hard stops, SESSION_LOG template).

Then four Council sections are appended **in order**:

```markdown
## Council Engineering Pass

### FORGEMASTER MEMO
- Allocation budget …
- Lock surfaces …
- Async boundary cost …
- Bench target / VAL slot …
- Static-dispatch / dyn call sites …
- Verdict …

### SENTINEL RISK AUDIT
- Failure modes named (≥ 1) …
- Threat surfaces …
- Rollback path …
- Observability hook …
- Data-integrity invariant …
- Verdict …

### SIMPLIFIER COUNTERPROPOSAL
- Removable surfaces (≥ 1 candidate) …
- Concrete reduction …
- Cost of keeping …
- Cost of removing …
- Verdict …

### ECONOMIST LEDGER
- Infra delta …
- Per-tenant scaling curve …
- Maintenance cost …
- Cheaper variant considered …
- Verdict …
```

**Sentinel** is forbidden from emitting a "no concerns" verdict (§2.3 Constitution). If today's surface genuinely has no Sentinel-class concerns (e.g. typo fix), Sentinel writes `Council: SENTINEL skipped — reason: <…>`.

**Simplifier** is forbidden from emitting an "accept-as-is" verdict on a day that introduces a new trait, struct, or feature flag. The Simplifier must produce ≥ 1 candidate reduction (which Forgemaster may then reject with cause).

---

## §5 · Adversarial & Evolutionary Passes (T8–T9)

### T8 — Tier-3 activation

Determined by **§14 Activation Matrix** of `ENTITY_SYSTEM.md`. For each activated entity:

```markdown
### ADVERSARY STRESS
- Threat T1 …
- Verdict …

### CHAOS DRILLS
- Drill 1 …
- Verdict …

### TEST PILOT PROFILE
- Workload assumed …
- p50/p95/p99 …
- Verdict …
```

If a Tier-3 entity should be active per the matrix but the day skips it, the skip is explicit (§18 Constitution).

### T9 — Tier-4 activation (monthly or trigger)

Tier-4 (Migrator, Ecosystem, Productor) is invoked:

- **Every monthly RETRO day** (`MASTER-ROADMAP §retro-cadence`).
- **Every ADR that touches public API shape** (Migrator + Ecosystem mandatory).
- **Every admin-UI day** (Productor mandatory).
- **Every constitutional amendment** (all three mandatory).

Tier-4 output lands in the **monthly README**, not the daily artifact:

```
dailies-v2/M03-2026-07/README.md §Migrator Outlook
                                  §Ecosystem Outlook
                                  §Productor Notes
```

---

## §6 · Conflict Resolution (T10)

A **conflict** exists when:

- Two entities reach contradictory verdicts on the same proposal; OR
- One entity's verdict is `reject` and the day's plan requires the rejected element; OR
- A Tier-2/3/4 entity issues `Council: <entity> dissent recorded` with concrete grounds.

When a conflict is detected, the **Judge** is invoked. The Judge runs §7 algorithm of `CONSTITUTION.md`:

```
1. Restate each position in ≤ 50 words (entities edit until precise).
2. Map positions to §3 Priority Ladder rungs (higher rung wins).
3. Apply §4 Immutables filter (drop positions reopening immutables).
4. Apply §5 Forbiddens filter (drop positions relying on forbidden patterns).
5. Apply §6 Anti-Drift filter (drop positions creating named drift).
6. If exactly one survives → ratify.
7. If multiple → pick least-`[evidence: TBD]` position.
8. Still tied → escalate to operator with side-by-side statement.
```

The Judge writes `### JUDGE VERDICT` into `architect.md`. The verdict is binding for the day and reviewable at the next RETRO.

---

## §7 · Anti-Drift Sweeps (T13)

End-of-day, end-of-week, end-of-month sweeps run **§6 Anti-Drift Laws** of `CONSTITUTION.md`:

### Daily (T13, every AVTONOM session)

| Detector | Check |
|---|---|
| **D-1 Scope** | `git diff --stat` LOC vs master-plan cell budget. Flag if > 1.3× expected. |
| **D-3 Capability** | `cargo xtask capability-coverage` exits 0. |
| **D-5 Pool mode** | `/health/pool` would still pass (no startup-contract violation). |
| **D-6 Planning trail** | `cargo xtask check-planning-refs` exits 0. |
| **D-7 Architecture** | `cargo xtask architecture-check` exits 0. |
| **D-10 Memory drift** | All memory facts referenced today were verified at T1. |

### Weekly (every Friday EOD)

| Detector | Check |
|---|---|
| **D-2 ADR drift** | Historian reviews week's commits against open ADRs; flags any silently-violated Consequences. |
| **D-4 Bench drift** | Test Pilot runs the relevant `criterion` bench; compares to prior week. |
| **D-9 Decision-graph drift** | Historian updates `docs/governance/decision-graph.md`. |

### Monthly (RETRO day)

| Detector | Check |
|---|---|
| **D-8 Forecast drift** | Orchestrator diffs next-month `MONTH-SKELETON-NN.md §Assumed entering state` vs current code reality. Patches skeleton in the RETRO. |
| **All D-*** | Full sweep; results land in `RETRO-YYYY-MM.md §drift-audit`. |

Any drift detector that trips writes to `memory/orchestrator_drift_log.md`.

---

## §8 · Session-End Ritual (T12)

After T11 (AVTONOM executes today's plan), the closing ritual is:

1. **Run V1..V4** again — confirm green.
2. **Run the four `xtask` gates** — `architecture-check`, `magic-check`, `capability-coverage`, `check-planning-refs`. All exit 0.
3. **Stage commits** by name (never `git add -A`).
4. **Write commit message** with required trailer:
   ```
   AI-Assisted: AX-ARCHITECT (Claude Opus 4.7)
   ```
5. **Verify `git log -1 --format=%s`** matches expectation.
6. **Write `SESSION_LOG.md`** at repo root, overwriting yesterday's, in the standard format.
7. **Run T13 Anti-Drift Sweep**.
8. **Update `INDEX.md`** — today's row status moves from `drafted` → `executed`.
9. **Save memory deltas** — each entity who learned something writes to their memory file.
10. **Emit closing summary** — one paragraph to operator: what landed, what's open, what tomorrow inherits.

**Never** `git push` without explicit operator instruction.

---

## §9 · Operator Interrupt Protocol

The operator can interrupt at any T-step. When interrupted:

1. **Acknowledge** within one tool call.
2. **Pause** all current work — no further tool calls except `Read` / `Grep` to answer the operator.
3. **Surface state** — one paragraph: what tier was running, what was the next planned action, what risks pausing now (e.g. uncommitted staged files).
4. **Wait** for operator direction.

The operator may:

- **Resume** — pick up at the paused tier.
- **Redirect** — abandon today's plan; new directive becomes the day.
- **Override** — instruct a specific entity to change verdict.
- **Halt** — commit WIP with `WIP:` prefix; close session.

Operator overrides are recorded in `SESSION_LOG.md §Operator Overrides`.

---

## §10 · Failure Modes of the Council itself

The Council can fail. Recognized failure modes:

| Mode | Symptom | Recovery |
|---|---|---|
| **Quorum failure** | Required Tier-1 entity produces empty/skipped output without legal reason. | Operator notified; day status held at `drafted`; no ratification. |
| **Deadlock loop** | Judge invoked, no surviving position, operator escalation not feasible. | Day plan reduced to *information-gathering only*; tomorrow ratifies after spike. |
| **Consensus theater** | All Tier-1 agree on first pass; Simplifier finds nothing. | Constitution §2.2 — Simplifier MUST attempt reduction; if genuine null, record `Simplifier null-attempt: <reasoning>`. |
| **Memory amnesia** | Read-before-trust (T1) finds wide divergence from memory. | Repair memory; record `D-10 memory drift` event; investigate why memory rotted. |
| **Anti-drift cascade** | Two or more drift detectors trip in one day. | Day suspended; full RETRO-style sweep before next-day planning. |
| **Entity capture** | One entity (typically Forgemaster on a perf rampage, or Simplifier on a deletion rampage) dominates verdicts. | Judge inspects pattern; corrective rebalance documented in next-RETRO. |

---

## §11 · Quality Gates the Council enforces

Beyond `cargo` and `xtask` gates, the Council enforces:

| Gate | Enforcer | Trigger |
|---|---|---|
| **A-1..A-10 anti-laziness** | Orchestrator at T2 + Sentinel at T5 | Every artifact at ratification time |
| **Allocation-budget statement** | Forgemaster at T4 | Any hot-path code touched |
| **Failure-mode statement** | Sentinel at T5 | Every day |
| **Decision-graph consistency** | Historian at T3 | Every new ADR |
| **Cost-curve statement** | Economist at T7 | Any feature with per-tenant scaling |
| **Threat-vector statement** | Adversary at T8 | Any public-input boundary change |
| **Rollback path statement** | Sentinel + Chaos at T5/T8 | Any migration, any cache-shape change |

Each gate emits a one-line PASS/FAIL into the artifact. A gate FAIL prevents ratification.

---

## §12 · Linkage to v2 Daily Tree

The `dailies-v2/` tree is the Council's *artifact store*. The protocol's mapping:

| Protocol step | v2 file written |
|---|---|
| T2 Orchestrator draft | `architect.md` (§1-§9) |
| T3 Historian | `architect.md §Council Review §Historian Trace` |
| T4 Forgemaster | `senior-dev.md` (P0..PN) + `§Council Engineering Pass §Forgemaster Memo` |
| T5 Sentinel | `senior-dev.md §Sentinel Risk Audit` |
| T6 Simplifier | `senior-dev.md §Simplifier Counterproposal` |
| T7 Economist | `senior-dev.md §Economist Ledger` |
| T8 Tier-3 | `senior-dev.md §Adversary Stress / §Chaos Drills / §Test Pilot Profile` |
| T9 Tier-4 | monthly `README.md §Migrator Outlook / §Ecosystem Outlook / §Productor Notes` |
| T10 Judge | `architect.md §Council Review §Judge Verdict` (only on conflict) |
| T11 Execution | `SESSION_LOG.md` at root |
| T12 Anti-drift | `memory/orchestrator_drift_log.md` + `INDEX.md` status update |

A day is `ratified` ⟺ all mandatory sections per `ENTITY_SYSTEM.md §16` are filled or explicitly skipped, and no `xtask` / `cargo` gate failed at T13.

---

## §13 · Mode Mapping (CLAUDE.md §M)

The protocol maps to the three operating modes:

| Mode | Council activation | Notes |
|---|---|---|
| **MANUAL** | Full protocol on every decision. AskUserQuestion for choices not pre-delegated. | Default; safest; most expensive in operator attention. |
| **SEMIAUTO** | Full protocol up to T10 (ratification). Then a single MANIFEST is presented; one operator approval covers the day. | Use when day's plan is well-bounded. |
| **AVTONOM** | Full protocol but T0-T12 run without operator interaction; defaults are taken per `architect.md §Decision Delegation` table. Operator reviews only `SESSION_LOG.md`. | Use when operator unavailable; spine touches still forbidden. |

All three modes run the full Council. The mode controls *when the operator is invoked*, not *whether the entities deliberate*.

---

## §14 · Bootstrap exception (the first session that adopts this protocol)

On the **first AVTONOM session that runs after this protocol ratifies** (target: 2026-05-27 or later), the session opens with a one-shot **Adoption Pass**:

1. Re-read all four governance files (CONSTITUTION + ENTITY_SYSTEM + EXECUTION_PROTOCOL + ROADMAP_ENGINE).
2. Patch today's `architect.md` and `senior-dev.md` to add the Council Review / Council Engineering Pass sections (initially empty stubs).
3. Run the Council loop against the *amended* artifacts.
4. Save first set of entity memory files: `memory/{orchestrator,forgemaster,sentinel,simplifier,historian,economist}_init.md`.
5. Mark the day as the **Adoption Day** in `INDEX.md` (custom status `adopted-pilot`).

Subsequent days run T0-T13 in full.

---

## §15 · Retrofit of M1 / M2 dailies

Once the v2 contract is `ratified` and the Adoption Pass closes, the M1 / M2 dailies are retrofitted **in monthly batches**:

- Each historical day's existing `daily/YYYY-MM-DD.md` (v1) is split into `architect.md` + `senior-dev.md` under the v2 tree.
- The Council retroactively reviews each day **lightly** — only Tier-1 + Historian; no full Tier-2-4 (those days are already executed).
- The retrofit's purpose is *uniformity*, not re-execution. Historical verdicts are recorded as `executed (post-hoc review)`.
- Operator OK required for each monthly batch.

---

## §16 · Closing directive

This protocol is the difference between **a chatbot writing markdown** and **a persistent multi-agent engineering civilization**.

Every AVTONOM session begins by reading this file at T0. If the protocol feels heavy on a quiet day, that is the protocol working: the Council's cost is the constant tax that prevents 12-month drift.

The protocol is **immutable** in spirit. Tactical refinements amend per `CONSTITUTION.md §11`.

---

**End of Execution Protocol.**
Next read: `ROADMAP_ENGINE.md` — how the 12-month plan is consumed and evolved by this loop.
