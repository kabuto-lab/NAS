# 2026-07-20 (Mon) · M3 · W1 · D1 — Architect Brief

> **Audience:** human operator at session-open.
> **Sibling:** [`senior-dev.md`](senior-dev.md) (AVTONOM-pasteable).
> **Authority:** binding once marked `ratified` in `../../../INDEX.md`.

---

## 1 · Master-plan cross-reference

**Source:** `docs/session-plans/WP-PLAN-12-MONTH.html`, M3 section, W1 first cell.

Verbatim cell content:

> **2026-07-20 · Пн · G1**
> *ADR-010 block-editor strategy — варианты (Gutenberg-wasm / Leptos / JSON+textarea)*

**Coordinates:**

| Axis | Value |
|---|---|
| Month | M3 (2026-07-20 → 2026-08-14) |
| Week | W1 — *ADR-010 editor strategy + Media model* |
| Day index | 1 of 5 (Monday) |
| Goal tag | **G1** (gates / migrations / RLS) — here read as *gating decision* (no SQL, but a binding architectural commitment) |
| Phase tag | **P1** (option study; P2 = finalization lands Tue 07-21) |
| Day-of-month | 1 of 20 |
| Day-of-twelve-month | 41 of 240 |

**Seed alignment:** `MONTH-SKELETON-03.md` §Goals · G1 — *"First architecturally-binding decision of P2. Compares: (a) port Gutenberg via wasm-bindgen (huge), (b) build Leptos block editor from scratch (smaller, fewer blocks), (c) read-only first + JSON textarea admin (ships fastest, technical debt against M7). Decision gates how M4 block library is consumed."*

---

## 2 · Why today matters (TLA, ENTITY §2)

This is an **option study**, not yet a commitment. The commitment lands tomorrow (W1 D2 · 2026-07-21) after a 24-hour soak. Today produces the *evidence base*: a written ADR draft enumerating three candidates, scored against a fixed criteria matrix, with the recommended choice argued.

### Layer 1 — Correctness

Choosing the editor substrate determines the **block-tree representation contract** between admin UI and the `nas2-domain::Block` enum. Any candidate whose serialization layer is lossy (e.g. WP's HTML-comment-delimited block syntax round-tripping through JSON) introduces ambiguity that propagates into M9 (plugin block contributions) and M12 (WP importer). The ADR must prove the candidate is *lossless* against `nas2-domain::Block` variants known today (the four from M1) and projected for M4 (the 19-variant set).

### Layer 2 — Performance

The substrate determines first-byte-to-interactive on `/admin/posts/:slug/edit`:
- WASM bundle size is the dominant cost (Gutenberg-wasm ≈ 2–4 MB gz; native Leptos block editor ≈ 100–200 KB gz; JSON-textarea ≈ 0).
- WASM cold-start CPU dominates p95 perceived latency on admin pages.
- Per ENTITY §3.2 the **200 KB gz ceiling** is a hard limit; any candidate breaching it requires an exemption ADR.

### Layer 3 — Scalability

Per-admin-session memory: WASM hydration holds the entire block library tree in linear memory; for a 50-author tenant editing concurrently this is the difference between 50 × 4 MB and 50 × 200 KB on the client devices. Server-side: a server-rendered first paint (HTMX-style island hydration) keeps the server stateless, which is what §3.2 demands.

### Layer 4 — Operability

- **Debuggability** — a JSON-textarea admin is trivially debuggable (server logs see exactly what got posted); a WASM-rendered editor obscures the encoding boundary.
- **Telemetry** — block-insert / block-reorder events must be `tracing`-instrumentable on the client; only the Leptos and textarea paths admit this without bundling a JS SDK.
- **Rollback** — if the chosen substrate proves wrong by M7, can M3..M6 work survive? Only the substrate-thin candidate (JSON-textarea) gives a graceful rollback path; the substrate-thick candidates effectively lock M4 block library shape.

### Verdict before option study (architect's prior)

The TLA priors lean **strongly toward native Leptos block editor MVP, deferring to JSON-textarea fallback for M3-M5 if Leptos editor work would crowd out the media pipeline.** Today's ADR draft must put numbers behind that prior or refute it.

---

## 3 · Inputs (prerequisites today)

1. **M2 RETRO closed clean** at 2026-07-17 — verify by reading `docs/session-plans/RETRO-2026-07.md` opening summary.
2. **`docs/rfc/RFC-001-block-library.md` exists and is ratified.** Today's ADR cites it; if missing, today's first phase is to read M1's RFC-001 surrogate.
3. **`docs/adr/ADR-009-revisions.md` ratified.** Revisions interact with the editor's autosave protocol (M3 W4); the ADR-010 must acknowledge this.
4. **`crates/domain::Block` enum** is at its 4-variant M1 state (`Heading`, `Paragraph`, `Image`, `CodeBlock`). The option study must enumerate which candidates can scale to 19 variants without re-architecture.
5. **No outstanding spine question for today.** If the operator flags a spine issue (e.g. proposed ENTITY edit), defer the ADR option study to Tue and run the spine confirmation today instead.

---

## 4 · Outputs (today's artifacts)

1. **`docs/adr/ADR-010-block-editor-strategy.md`** (NEW, draft) — sections:
   - **Status:** `Proposed` (Tue moves to `Accepted`).
   - **Context:** block-tree representation contract; admin demo by M7; plugin contributions by M9.
   - **Decision drivers (5):** lossless serialization · bundle size ≤ 200 KB gz · M4 19-variant scalability · debuggability · M7 demo viability.
   - **Considered options (3):** Gutenberg-wasm bridge / native Leptos block editor / JSON-textarea minimal admin.
   - **Decision outcome:** **Native Leptos block editor (MVP) with JSON-textarea fallback gated by a W3 progress checkpoint.** (Final wording lands Tue.)
   - **Consequences:** what M4 G1 inherits (the variant-rendering contract), what M7 G3 inherits (the canvas component), what M9 G2 inherits (the hook surface for plugin-contributed blocks).
   - **Pros/cons matrix:** the 5 decision-driver columns × 3 option rows.
   - **Forward links:** `ADR-013` (theme rendering) M8, `ADR-014` (WASM sandbox) M9.

2. **Update to `INDEX.md`** — today's row moves from `planned` → `drafted`.

3. **Single commit** on `main` (local, not pushed) with trailer `AI-Assisted: AX-ARCHITECT (Claude Opus 4.7)`, referencing `PLAN-005-media-aggregate` (the closest active PLAN slot; ADR-010 is policy work that is *seeding* M3, not part of a numbered execution plan).

4. **`SESSION_LOG.md`** at repo root, overwriting yesterday's, in the standard AVTONOM format.

---

## 5 · Success criteria (gates)

| # | Gate | Pass condition |
|---|---|---|
| **G-Day-1** | V1–V4 baseline | `cargo check / fmt --check / clippy / test --workspace --lib` all exit 0. No regression vs. yesterday. |
| **G-Day-2** | ADR file exists | `docs/adr/ADR-010-block-editor-strategy.md` present, ≥ 600 lines, all 8 ADR sections populated. |
| **G-Day-3** | Decision matrix populated | The 5×3 matrix has a numeric or categorical entry in every cell — no `TBD`. |
| **G-Day-4** | `cargo xtask check-planning-refs` | Passes the commit message check (commit mentions `ADR-010`). |
| **G-Day-5** | Cross-link consistency | The ADR's "Considered options" link to RFC-001 (block library); INDEX.md links back to the new ADR; M03 README updates the `ADR-010` status row to `drafted`. |

If any gate fails, the day commits **partial work with `WIP:` prefix**, opens an entry in `SESSION_LOG.md §Skipped/Blocked`, and tomorrow's session begins with a CARRY-OVER repair phase rather than the ADR finalization.

---

## 6 · Spine impact

**Zero** spine files touched today.

- All edits land under `docs/adr/`, `docs/session-plans/dailies-v2/` (the INDEX), and `SESSION_LOG.md` (root) — all non-spine per ENTITY §12.

No mini-edit authorizations needed for today.

---

## 7 · Forward inheritance

What downstream months **must** read today's ADR before starting:

| Month / Day | What it inherits |
|---|---|
| **M3 W4 D3 · 2026-08-12** (autosave) | Editor protocol — autosave granularity (per-block? per-document?) is fixed by ADR-010. |
| **M4 W1 D1 · 2026-08-17** (Block::List + Quote + Table) | The variant-rendering contract — does the chosen editor render via server-rendered Leptos islands or client-side rerender on dirty state? |
| **M4 W2 D3 · 2026-08-26** (ReusableBlock ref-counting) | Editor must support `Block::Reusable {id}` as a transparent reference — fixed in ADR-010 §Consequences. |
| **M7 W3 (whole)** (block editor MVP) | The canvas component the chosen substrate produces. |
| **M9 W2 D1 · 2027-01-11** (Hook registry) | The hook surface for plugin-contributed block variants — only meaningful if the editor substrate supports dynamic block-type registration. |
| **M12 W1 D1 · 2027-03-29** (WP importer WXR) | Round-trip target — WP-exported blocks must deserialize into the substrate without semantic loss. |

The architect should physically open `MONTH-SKELETON-04.md`, `-07.md`, `-09.md`, `-12.md` and confirm none of them assumes a substrate inconsistent with today's recommendation.

---

## 8 · Risk register for today

| # | Risk | Likelihood | Blast | Mitigation pre-decided |
|---|---|---|---|---|
| R-D-1 | ADR author falls in love with Gutenberg-wasm because of WP-plugin compat draw, despite §3.2 bundle-size ceiling. | M | M4-M12 perf debt | The matrix has a *hard-fail row* (bundle size > 200 KB gz = ❌). No subjective override allowed today; an exemption ADR is a separate Tue P0. |
| R-D-2 | Time pressure leads to a thin ADR ("we'll figure it out"). | M | M7 demo at risk | The 5 decision drivers are pre-specified above; the matrix is the contract. Tue refuses to ratify a `Proposed` ADR with empty cells. |
| R-D-3 | ADR-010 collides with ADR-009 (revisions) protocol shape. | L | needs Tue rework | The "Considered options" section of ADR-010 must include a row "interaction with ADR-009 autosave protocol"; if any candidate breaks ADR-009 it is dropped without consideration. |
| R-D-4 | Spine pressure (someone asks to touch ENTITY today). | L | day lost | MANUAL mode default; refuse spine edits today; defer to next ENTITY-revision session. |

---

## 9 · Decision delegation (what the AVTONOM may resolve via AI-Default)

| Choice point | Default | Recorded as |
|---|---|---|
| ADR file numbering — `ADR-010` (predicted) vs first available slot. | Use `010` if `docs/adr/` does not already contain `ADR-010-*`; else next free integer. | Commit message `AI-Default: ADR slot N chosen because slot 010 occupied by …`. |
| ADR section order (Madr v3 vs free-form). | **Madr v3** (status / context / drivers / options / outcome / consequences / pros-cons / links). | Inline in the ADR `Status:` line. |
| Recommended option name in the draft. | "Native Leptos block editor (MVP) with JSON-textarea operator fallback during M3-M5." Confirmed by today's option-study evidence; refute & rewrite if evidence overturns it. | ADR §Decision Outcome. |
| Whether to include a 4th option (HTMX-only forms). | Yes, but as an explicit "discarded" entry in §Considered Options, not a row in the matrix. | ADR §Considered Options ¶ rejected. |

Choice questions **not** delegated (must block and AskUserQuestion):

- Any proposal to amend ENTITY.md or CLAUDE.md.
- Any proposal to change today's commit policy (no push, no force-push, no `--amend` on prior commits).
- Any proposal to defer `ADR-010` beyond Tue 2026-07-21 — that is a master-plan shift and needs human OK.

---

## 10 · Failure modes (what aborts the day)

| Trigger | Action |
|---|---|
| `V1..V4` red at session start. | First phase becomes a green-restore; ADR draft slips to W1 D2; Tue's P2 finalization shifts to W2 D1; M3 RETRO carries a 1-day amber flag. |
| Disk full / OOM. | Hard stop. Operator notified. |
| Internet outage beyond `cargo registry`. | Continue — today is offline-feasible. |
| Operator interrupts with a higher-priority directive. | Per CLAUDE.md §M MANUAL default — yield, ask. |

---

## 11 · End-of-day checklist for the human reviewer

Before marking today `ratified` in `INDEX.md`:

- [ ] `docs/adr/ADR-010-block-editor-strategy.md` opens cleanly and reads as an ADR, not a stream of consciousness.
- [ ] The 5×3 decision matrix has no `TBD`/`?` cells.
- [ ] The recommended option is named in §Decision Outcome and is consistent with §Pros/Cons.
- [ ] Forward links to M4/M7/M9/M12 ADR slots are present.
- [ ] Single commit; message references `ADR-010`; trailer `AI-Assisted: AX-ARCHITECT (Claude Opus 4.7)` present.
- [ ] No spine file touched (`git diff --name-only main..HEAD | grep -E '(ENTITY|CLAUDE|^Cargo\.toml|^docker-compose|^\.env|clippy\.toml|rustfmt\.toml|deny\.toml|^migrations/)'` returns empty).
- [ ] `cargo xtask check-planning-refs` exits 0.
- [ ] `SESSION_LOG.md` at root has all standard sections populated.

If all check, set status in `INDEX.md` to **`ratified`** and the day is closed.

---

## Council Review

> **Appended at the 2026-05-26 Adoption Pass** per `EXECUTION_PROTOCOL.md §14`.
> The Council reviewed this future-dated artifact (2026-07-20) at adoption time to
> establish the v2 contract. Re-review is automatic at session-open on 2026-07-20.

### ORCHESTRATOR

- **Master-plan alignment:** ✓ `WP-PLAN-12-MONTH.html` M3 W1 row-1 cell quoted verbatim in §1. `MONTH-SKELETON-03.md §Goals · G1` quoted in §1. `MASTER-ROADMAP-2026-2027.md` M3 parity arc (+7% → 21%) consistent.
- **Dependency status — checked at adoption time (2026-05-26):**
  - `docs/rfc/RFC-001-block-library.md` — **NOT YET PRESENT** at adoption (M1 W4 D3 has not run; `Grep` returns 0). At execution time (2026-07-20) this is a hard prerequisite. Drift detector D-10 armed: senior-dev.md P1 must `Glob` for the file at session start.
  - `docs/adr/ADR-009-revisions.md` — **NOT YET PRESENT** at adoption (lands M2 W4 D4 · 2026-07-16). Same D-10 armed in P1.
  - `crates/domain::Block` 4-variant state — **CURRENT REALITY: stub** (`crates/domain/src/lib.rs` is 20 LOC). Will be in 4-variant state by M1 W2 D4 · 2026-06-04. Live check required at execution time.
- **Forward-inheritance map:**
  - M4 W1 (block library expansion, 15 variants) inherits `EditableBlock` trait shape.
  - M7 W3 (block editor MVP) inherits canvas component substrate.
  - M9 W2 (Hook registry → plugin block contributions) inherits `BlockSchema` + WASM-export contract.
  - M12 W1 (WXR importer) inherits the `Vec<Block>` target shape for WP-block round-trip.
- **Drift detectors triggered at adoption:**
  - D-8 Forecast: `MONTH-SKELETON-03.md §Assumed entering state` says "ADR-009 written" — armed (must verify at execution).
  - D-1, D-3, D-5, D-6, D-7, D-9, D-10: green at adoption (docs-only day, no code surface).
- **Verdict:** **approve-with-conditions**. Conditions: (a) execution at 2026-07-20 verifies RFC-001 + ADR-009 + `Block` 4-variant state live, repairs via CARRY-OVER if missing; (b) Adoption-time `Status: drafted` upgrades to `ratified` only after the 2026-07-21 finalization session confirms.

### HISTORIAN TRACE

- **Decision-graph delta (this day, when executed):**
  - +Node: `ADR-010-block-editor-strategy` (state: Proposed → Accepted on 2026-07-21).
  - +Edges: `Consulted: RFC-001`, `Consulted: ADR-009`, `Forward-binds: ADR-013 (M8)`, `Forward-binds: ADR-014 (M9)`.
- **Prior-rejection check:** none. The block-editor substrate space has not been formally explored in any ratified ADR. Gutenberg-wasm, native Leptos, JSON-textarea are first-considerations.
- **Proposed-ADR aging audit:** ADR-010 is `Proposed` from 2026-07-20 EOD → 2026-07-21 EOD. ≤ 24 h window. Well within §F-10 (no proposed ADR > 7 days).
- **Decision-graph file update:** `docs/governance/decision-graph.md` receives the ADR-010 node at adoption (this Pass writes it as `Status: anticipated`, to flip to `Status: ratified` on Tue 2026-07-21 close).
- **Verdict:** **consistent**. No contradictions with ADR-009 (revisions/autosave protocol — ADR-010 §Consequences §Inherited by M3 W4 explicitly aligns). No contradictions with RFC-001 (block-library — ADR-010 cites it as input).

### JUDGE VERDICT

**Not invoked.** No conflict between Tier-1 / Tier-2 / Tier-4 verdicts. Simplifier produced a counterproposal (recorded in `senior-dev.md §Simplifier Counterproposal`); Forgemaster + Sentinel + Orchestrator did not contest it. The counterproposal is binding for the Tue 2026-07-21 ratification session.

---

**Adoption-Pass status:** this `architect.md` is now Council-reviewed. INDEX.md row status: `adopted-pilot` (custom status for the v1.0 governance bootstrap). Tue's finalization session is the first true `ratified` event.
