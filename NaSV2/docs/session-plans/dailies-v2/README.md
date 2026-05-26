# Dailies v2 — Two-Layer Prompt Store

> **Status:** pilot live (`M03-2026-07/W1-2026-07-20/2026-07-20-mon/`).
> **Authority:** binding once a day is marked _ratified_ in `INDEX.md`.
> **Scope:** M3 (2026-07-20) through M12 (2027-04-23) — 200 working days.

---

## Why v2

The flat `docs/session-plans/daily/YYYY-MM-DD.md` tree (v1) bundles
strategic intent (the architect) and tactical implementation (the
senior dev) into one file. That worked for the pre-generated M1/M2
batch but conflates two distinct audiences:

| Audience | Reads to answer |
|---|---|
| **Architect / human reviewer** | "Does today's plan honor `WP-PLAN-12-MONTH` + ENTITY §s? What does it cost if we cut this day? What does M+N inherit from today?" |
| **AVTONOM session** | "What do I type into the editor for the next 4 hours so that V1..V4 stay green and a commit lands clean?" |

v2 splits them. Each day is a directory containing **two sibling
files**, each independently readable.

---

## Layout

```
dailies-v2/
├── README.md                    ← this file (contract)
├── INDEX.md                     ← master table: 240 rows → status
├── _templates/
│   ├── architect.md.template    ← shape every architect.md must satisfy
│   └── senior-dev.md.template   ← shape every senior-dev.md must satisfy
├── M03-2026-07/                 ← one folder per month
│   ├── README.md                ← month-level brief (theme, parity Δ, ADR slots)
│   ├── W1-2026-07-20/           ← one folder per week
│   │   ├── README.md            ← 5-day glance + carry-over chain
│   │   ├── 2026-07-20-mon/      ← one folder per day
│   │   │   ├── architect.md     ← strategic brief (≤ 250 LOC)
│   │   │   └── senior-dev.md    ← tactical AVTONOM-pasteable (500–1200 LOC)
│   │   ├── 2026-07-21-tue/
│   │   ├── 2026-07-22-wed/
│   │   ├── 2026-07-23-thu/
│   │   └── 2026-07-24-fri/
│   ├── W2-2026-07-27/
│   ├── W3-2026-08-03/
│   └── W4-2026-08-10/
├── M04-2026-08/   …   M12-2027-04/
```

---

## The two layers — exact contract

### `architect.md`

Strategic, human-facing. **Not pasteable into AVTONOM.** Answers:

1. **Master-plan cross-ref** — exact `WP-PLAN-12-MONTH` cell quoted (month / week / day / goal-tag / scope phrase).
2. **Why this day matters** — TLA Level 1–4 evaluation (Correctness, Performance, Scalability, Operability) per ENTITY §2. "No impact" allowed; "not considered" is not.
3. **Inputs** — concrete prerequisites: files that must exist, ADRs that must be ratified, tests that must be green.
4. **Outputs** — concrete artifacts produced today (file paths, ADR/PLAN/VAL slot numbers, migration numbers).
5. **Success criteria** — V1..V4 gate set + day-specific gate (e.g. `cargo xtask check-planning-refs` matches the new ADR slot).
6. **Spine impact** — explicit `[spine] / [non-spine] / mini-edit-authorized` per file per ENTITY §12.
7. **Forward inheritance** — what month-N+1..N+M depends on this day.
8. **Risk register** — top 3 ways today fails + mitigation pre-decided.
9. **Decision delegation** — what *choice questions* the AVTONOM is allowed to resolve from defaults vs. what must block. Mirrors §22.2 AI-Default contract.

The architect file is the document the human reads *before* approving today's session.

### `senior-dev.md`

Tactical, AVTONOM-runnable. **First line is `AVTONOM:` — paste verbatim** as the session-opening message (per CLAUDE.md §M / ENTITY §22.3). Answers:

1. **CARRY-OVER** — from yesterday's session log (one line each).
2. **CONTEXT** — entering state (files/lines that exist), exit state expected.
3. **SCOPE** — phased `P0..PN`. Each phase has a deliverable, code skeleton, and verification command.
4. **Code skeletons** — full Rust modules, ready to paste, type-checked mentally. No "TODO: fill in" placeholders.
5. **Tests** — exhaustive list of unit + integration + property tests, each with input → expected output stated.
6. **EXPECTED CLIPPY ALLOWS** — every `#[allow(clippy::...)]` justified.
7. **COMMON PITFALLS** — CRLF, walkdir traversal, brace counting in strings, etc.
8. **PRE-RESOLVED DEFAULTS** — AI-Defaults for choice points the architect.md delegated.
9. **HARD STOPS** — conditions that abort the session before commit.
10. **ЗАПРЕЩЕНО АБСОЛЮТНО** — absolute forbids (spine touches outside authorized list, `git push`, etc.).
11. **SESSION_LOG template** — exact section structure to write at end-of-day.
12. **CARRY-OVER for tomorrow** — pre-drafted lines for tomorrow's session log header.

The senior-dev file is the document AVTONOM runs *during* today's session.

---

## Cross-reference invariants

Every `architect.md` MUST link to:

- The `WP-PLAN-12-MONTH.html` row by month + week + date (quote the `<div class="scope">` text verbatim).
- The matching `MONTH-SKELETON-NN.md` goal block (G1..G5).
- The corresponding `senior-dev.md` sibling.
- Any RFC / ADR / PLAN / VAL slots affected (by number, even if pending).

Every `senior-dev.md` MUST link back to its `architect.md`.

`INDEX.md` is the truth table. A day exists in v2 ⟺ both layers exist and are listed in `INDEX.md` with status ≥ `drafted`.

---

## Status taxonomy (used in `INDEX.md`)

| Status | Meaning |
|---|---|
| `planned` | Row exists in master plan; no v2 files yet. |
| `drafted` | Both layers written; not yet human-reviewed. |
| `ratified` | Architect.md reviewed by human; senior-dev.md verified runnable. |
| `executed` | AVTONOM session has run this day. SESSION_LOG.md committed. |
| `superseded` | RETRO of month N decided to redo this day; new files in `_superseded/<sha>/`. |

---

## v1 vs v2 — coexistence

`docs/session-plans/daily/*.md` (v1) remains the **active** source for M1 (currently running) and M2 (pre-generated). v2 will rewrite both once M3 onward is complete and the format is proven.

Once a v2 day reaches `ratified`, the v1 equivalent is moved to `docs/session-plans/daily/_v1-archive/` with a forwarding header.

---

## How AVTONOM consumes this tree

Bootstrap day (every Monday morning) reads, in order:

1. `dailies-v2/INDEX.md` → find today's row → status check.
2. `dailies-v2/MNN-YYYY-MM/Wk-YYYY-MM-DD/README.md` → week theme + carry-over.
3. `dailies-v2/MNN-YYYY-MM/Wk-YYYY-MM-DD/YYYY-MM-DD-dow/senior-dev.md` → paste opening line.

The architect.md is **not consumed by AVTONOM**. It is consumed by the human operator at session-start to verify the day still serves the master plan.

---

## Sub-spine status (this tree)

The v2 tree is **non-spine** per ENTITY §12: it is documentation, not code. However, individual `architect.md` files that have reached `ratified` MUST NOT be edited without an explicit human OK — they are the contract under which the corresponding AVTONOM session ran. Edits land in `_superseded/`.

---

## Naming rules (binding)

- Month folder: `MNN-YYYY-MM` (zero-padded month index, ISO year-month). Example: `M03-2026-07`.
- Week folder: `WN-YYYY-MM-DD` where the date is the **Monday** of that week. Example: `W1-2026-07-20`.
- Day folder: `YYYY-MM-DD-dow` where `dow` is `mon|tue|wed|thu|fri` (English, 3-letter, lowercase). Example: `2026-07-20-mon`.
- File names: exactly `architect.md` and `senior-dev.md`. No variants, no suffixes.

---

## See also

- `WP-PLAN-12-MONTH.html` — visual master plan (240 cells)
- `MASTER-ROADMAP-2026-2027.md` — narrative master plan
- `MONTH-SKELETON-{03..12}.md` — month-level seeds (one per month)
- `GAP-ANALYSIS-WP-PARITY.md` — WP feature surface scoring
- `avtonom-month-bootstrap.md` — monthly RETRO → next-month seed protocol
