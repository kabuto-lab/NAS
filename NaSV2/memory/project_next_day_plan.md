---
name: project-next-day-plan
description: Canonical "what to do at next AVTONOM session" — referenced from ENTITY.md §22.0 step 6 and CLAUDE.md session-start ritual.
metadata:
  type: project
---

# Project — Next-Day Plan (canonical)

> **Last updated:** 2026-05-26 (M1 W4 close-out AVTONOM session).
> **Read order:** this file is read at session-start step 6 per
> `ENTITY.md §22.0`. Read AFTER governance docs (CLAUDE.md §STOP
> steps 1-4) but BEFORE parsing the operator's opening message.
>
> **If this file is older than 7 days at session-open, treat with
> caution and verify against `git log` + the latest RETRO before
> acting on it.**

---

## 1 · Active mission scope (binding)

**AX•CMS is a Content + Commerce + CRM platform**, not just a CMS.

The originating mission in `ENTITY.md §0` ("WordPress replacement") is
**extended**, not replaced, by:

- `docs/governance/MISSION-V2-COMMERCE-CRM.md` — comprehensive rationale + architectural impact
- `docs/governance/master-plan-diffs/MPD-001-commerce-crm-pivot.md` — operational diff against the 12-month roadmap

**Read both before making any architectural decision touching M2+.**

The platform stack (ENTITY.md §3 + CONSTITUTION §4 Immutables) is
**unchanged**. The pivot is additive:

- Existing aggregates (`Post`, `Page`, `Site`, `Media`, `User`, `Role`, `Capability`) stay.
- New aggregates land at M2 W6 D1: `Product`, `ProductVariant`, `Order`, `OrderLine`, `Customer`, `Contact`, `Lead`.
- Capability enum gains 9 variants (`commerce.*` + `crm.*` namespaces).
- Domain module restructure (`content/` + `auth/` + `commerce/` + `crm/` + `shared/` subdirs) lands M2 W6 D1.

---

## 2 · Where the project is right now

**Date at last session-end:** 2026-05-26 (M1 W4 D2..D5 close-out).
**Real-world calendar position:** M1 closed (20 of 20 daily prompts complete).
**Last commit pushed to `origin/main`:** **NOT YET PUSHED** — last commit on `main` at HEAD is `d42a55a` (bootstrap-2026-07).
**M1 W4 D2..D5 commits (5 in this session):**
- `7aede89` W4 D2 — xtask capability-coverage real impl
- `3641ed5` W4 D3 — planning trail backfill (5 new docs)
- `8f3806d` W4 D4 — ammonia clean_html + perf baseline + SEC-002/003
- `9cbbdf6` W4 D5 — RETRO-2026-06
- `d42a55a` W4 D5 — bootstrap-2026-07

**Total M1 commits (b67302e..HEAD, NaSV2/ scope):** 37 (includes session-logs, mission expansion, RETRO, bootstrap).
**Workspace tests passing:** 136 (lib + integration); was ~129 pre-W4 D2..D5; +10 this session (xtask +5, common +5).
**WP-parity:** still 6% (foundation only; parity climbs in M2 with first content + commerce write paths).

---

## 3 · What the next AVTONOM session should do

### M1 is closed. The next session bootstraps M2.

**Recommended trigger:** operator pastes
`docs/session-plans/avtonom-month-bootstrap-2026-07.md` as the
opening message of a new Claude Code session.

The bootstrap will produce, in one session:
1. `docs/session-plans/AUDIT-2026-06-22.md` — deep state-of-repo audit
2. `docs/session-plans/ROADMAP-2026-07.md` — monthly plan, 4 weeks
3. `docs/session-plans/WEEK-{05..08}.md` × 4 — week-by-week scope
4. `docs/session-plans/daily/2026-06-22..07-17.md` × ~20 — daily prompts
5. `docs/session-plans/HOW-TO-RUN.md` refresh

The bootstrap is engineered to ask no questions and to interleave
commerce + CRM threads per MPD-001 §3 revised theme for M2.

**Alternative trigger (if M2 daily prompts pre-exist already):**
operator pastes `docs/session-plans/daily/2026-06-22.md` (M2 W5 D1).

**Update 2026-05-26 (second AVTONOM call of the day):** MPD-001 weave
overlay applied to existing CONTENT-track baseline.
See `docs/session-plans/MPD-001-M2-WEAVE.md` for the spec.
Four daily prompts (`2026-06-29`, `2026-07-14`, `2026-07-15`,
`2026-07-16`) gained `## MPD-001 WEAVE ADD-ON` sections (4.5 h
cumulative budget, additive, each deferrable). ROADMAP §B7, WEEK-06
overlay, WEEK-08 overlay, AUDIT §A12.5 updated to reference the
weave doc. So either trigger (bootstrap re-paste or first daily
2026-06-22) will pick up the weave automatically.

### M2 forecast (per MPD-001 §3 revised theme)

- **M2 W5** (2026-06-22..26) — migrations 0002..0008 (sites / posts +
  RLS + users + roles + caps + assignments). Foundation; commerce
  begins W6.
- **M2 W6** (2026-06-29..07-03) — `PgPostRepository` AS PLANNED. Also
  begin `Product` aggregate scaffolding + `ProductRepository` port.
  **Domain module restructure (`content/`, `auth/`, `commerce/`,
  `crm/`) lands W6 D1.**
- **M2 W7** (2026-07-06..10) — JWT + login AS PLANNED. Auth is
  universal. Replaces `extract_caps_for_today` stub (closes FM-006
  per `memory/sentinel_init.md`).
- **M2 W8** (2026-07-13..17) — Revised: 3 Posts CRUD (read/list/create)
  + 3 Products CRUD + 1 Customer endpoint. RETRO 2026-07-17
  incorporates commerce metrics.

---

## 4 · Open carry-over items

### 4.1 RETRO-2026-06 §9 operator actions

- **Push** `b67302e..HEAD` to `origin/main`. AVTONOM never pushes;
  this is operator-only.
- **Re-baseline `docs/perf/baseline.json`** on quiet machine or CI
  runner (current bootstrap is operator-windows-host).
- **Wire `cargo xtask check-planning-refs --commit
  <merge-base>..HEAD` into CI** (use a range, not bare HEAD; see
  RETRO §7-5).

### 4.2 RETRO-2026-06 §7 next-month proposals (5; seed for AUDIT-2026-06-22 B1 goals)

1. **Image pipeline phase 1** — libvips + pgmq worker (M3 W2 prep)
2. **Real `PgPostRepository` + `PgUserRepository`** (M2 W6) — first
   SEC-003 contract test
3. **`cargo fmt --all` cleanup + pre-commit hook** (M2 W5 D1) — fix
   ~25-file fmt drift + prevent recurrence
4. **JWT-derived caps extraction** (M2 W3) — closes FM-006; replaces
   `extract_caps_for_today` stub
5. **`xtask check-planning-refs --commit` semantics fix** (M2 W5 D2)
   — change default so bare `HEAD` means single HEAD commit, not
   full ancestor walk

### 4.3 Binding outcomes (unchanged from prior carry-over)

- **VAL-009** — editor bundle-size guard ≤ 200 KB gz, M3 W3 D5 (2026-08-07)
- **ADR-010 ratification** — Tue 2026-07-21 with binding Simplifier counterproposal
- **Static-dispatch constraint** for per-variant `EditableBlock` impls (M3 W3+)
- **RFC-009 (plugin SDK shape)** — M9 W1 D1 (2027-01-04); MUST include commerce + CRM hook types per MISSION-V2 §3
- **Capability stub → JWT** — M2 W3 (security debt; FM-006)
- **xxhash-rust ADR** for cross-process cache key stability — M10 with Dragonfly L2
- **ENTITY.md §0 amendment** — pending operator-authorized ENTITY spine-touch window; until then MISSION-V2 + MPD-001 are canonical mission

### 4.4 Workspace topology notes (NEW)

- The git repo root is `barbie/AX/`, not `NaSV2/`. `git add` paths
  from the parent repo must include `NaSV2/` prefix.
- ~50% of the NaSV2 workspace is **untracked** in git: workspace
  `Cargo.toml`, scaffold crates (`edge-adapter`, `extension-api`,
  `image-pipeline`, `search-engine`, `theme-api`), ADRs (002..006),
  RFCs (002), PLANs (001..004), `.env.example`, `docker-compose.dev.yml`,
  many more. The code WORKS — these files exist on disk and the
  workspace builds + tests pass. They simply were never `git add`ed
  in prior sessions.
- This is operator-judgement scope: a batch `git add` would commit
  ~hundreds of files. Defer until operator decides.

### 4.5 Adversary watch (Tier-3 activation forecast)

Per `ENTITY_SYSTEM.md §14` activation matrix, Adversary auto-engages
for:
- Every commerce + CRM endpoint (commerce surface widens attack
  scope per MPD-001 Sentinel verdict)
- Every auth-touching day (JWT lands M2 W3)
- Every public input boundary

Next Tier-3 activation: M2 W5 D3 (first RLS-bearing migration);
M2 W6 D2 (first sqlx repo with `set_config`); M2 W7 D1 (JWT
extraction).

---

## 5 · Drift watch

| Detector | Last state | Notes |
|---|---|---|
| D-1 Scope | green | M1 complete; M2 entering state aligned with code |
| D-2 ADR drift | dormant | Next sweep at M2 RETRO 2026-07-17 |
| D-3 Capability | green | `xtask capability-coverage` ok at M1-end |
| D-4 Bench | green (info) | First baseline.json captured W4 D4 |
| D-5 Pool mode | green | server boot enforces |
| D-6 Planning trail | green for new commits | 20 historical bootstrap commits fail full-history scan; workaround = range |
| D-7 Architecture | green w/ documented WARN | 1 WARN: presentation→infrastructure waiver (documented) |
| D-8 Forecast | green | M2 entering state matches code state |
| D-9 Decision-graph | green | New ADR-001 + RFC-001/003/004 + VAL-001 coherent; no contradictions |
| D-10 Memory | repaired | This file updated to reflect M1 closed |

---

## 6 · Operator notes from this session (2026-05-26)

The operator typed bare "AVTONOM" — strict reading would default to
MANUAL, but the intent was clearly AVTONOM. Used `AskUserQuestion`
to disambiguate scope; operator chose **W4 D2..D5 close-out + full
autonomy on non-spine**.

The session completed all five M1 W4 D2..D5 daily prompts in one
operator-extended AVTONOM run. Commits per day (5 total). Council
protocol upheld throughout. No drift cascade, no quorum failure,
no Judge escalation. Two pre-existing conditions surfaced and were
logged but not fixed (out of scope): fmt drift in ~25 tracked files,
and ~half of NaSV2 untracked in git topology. Both are RETRO §7
proposals for M2.

---

## 7 · TL;DR for the next AVTONOM session

```
1. Load CLAUDE.md ## STOP ritual (4 steps).
2. Read this file (you ARE reading it — step 6 of §22.0).
3. Read RETRO-2026-06.md (full) — its §7 is the seed for M2 G1..G5.
4. Read MPD-001 + MISSION-V2 (binding mission frame for M2+).
5. The opening message will either be:
   (a) Operator pastes avtonom-month-bootstrap-2026-07.md — execute
       Phase A..D to generate AUDIT/ROADMAP/WEEK/daily for M2.
   (b) Operator pastes a specific M2 daily prompt — execute that day.
6. Carry MPD-001 awareness throughout M2 — commerce + CRM threads
   interleave with content work per MPD-001 §3 revised theme.
7. M2 RETRO at 2026-07-17 produces M3 bootstrap or continues the
   pattern.
```

---

**End of next-day plan.**
M1 closed 2026-05-26. M2 begins on operator trigger.
MPD-001 M2 weave overlay applied 2026-05-26 (second AVTONOM session).
Append-only drift entries to `memory/orchestrator_drift_log.md` if
any D-* detector trips.
