---
name: project-next-day-plan
description: Canonical "what to do at next AVTONOM session" — referenced from ENTITY.md §22.0 step 6 and CLAUDE.md session-start ritual.
metadata:
  type: project
---

# Project — Next-Day Plan (canonical)

> **Last updated:** 2026-05-26 (mission-expansion session).
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
- New aggregates land at M2 W5+: `Product`, `ProductVariant`, `Order`, `OrderLine`, `Customer`, `Contact`, `Lead`.
- Capability enum gains 9 variants (`commerce.*` + `crm.*` namespaces).
- Domain module restructure (`content/` + `auth/` + `commerce/` + `crm/` + `shared/` subdirs) lands M2 W5 D1.

---

## 2 · Where the project is right now

**Date at last session-end:** 2026-05-26.
**Real-world calendar position:** M1 W3 D5 just completed; W4 D1 staged but not committed (see §4 below).
**Last commit pushed to `origin/main`:** `f056adb` (SESSION_LOG W1+W2+W3 close).
**Total commits in M1 to date:** 18 (governance + W1 + W2 + W3 + 3 session logs).
**Workspace tests passing:** ~129.
**WP-parity:** 6 % (foundation only; parity climbs in M2 with first content+commerce write paths).

---

## 3 · What the next AVTONOM session should do

### Real-world next session = M1 W4 D1 onward — execute as planned

The M1 W4 prompts at `docs/session-plans/daily/2026-06-15.md` through
`2026-06-19.md` are **foundation work** (xtask capability-coverage
real impl, planning backfill, ammonia clean_html, RETRO + M2 bootstrap).
This work is **unaffected by the commerce + CRM pivot** — it builds
the gates and the sanitization layer that commerce + CRM will use.

**Execute M1 W4 as the daily prompts specify.** Do NOT divert to
commerce work mid-W4; that would break the M1 close-out narrative.

### M1 W4 D5 RETRO (2026-06-19) is where the pivot is operationally absorbed

At the M1 W4 D5 RETRO session, the Council does the following EXTRA
work alongside the normal monthly retrospective:

1. **Read MPD-001 and MISSION-V2 again** (they are pinned doctrine now).
2. **Produce `docs/session-plans/MASTER-PLAN-12-MONTH-v2.html`** —
   the revised master plan reflecting commerce + CRM threads per
   MPD-001 §3. `WP-PLAN-12-MONTH.html` remains as historical record.
3. **Regenerate the M2 daily prompts** at
   `docs/session-plans/daily/2026-06-22..07-17.md` to weave commerce
   + CRM into the per-day work. The existing M2 prompts (pre-generated
   2026-05-25) are valid as a CONTENT-track baseline — the regenerated
   versions interleave product + customer + order scaffolding per
   MPD-001 §M2 revised theme.
4. **Update `dailies-v2/INDEX.md`** to reflect the v2 master plan rows.
5. **Migrator + Ecosystem outlooks** for M2 land in
   `dailies-v2/M02-2026-06/README.md` with explicit commerce + CRM scope.
6. **Commit the RETRO + the new master plan + the regenerated daily
   prompts as a single coherent commit** with trailer
   `Master-Plan-Diff: MPD-001` (or a v2 follow-up MPD if revisions
   accumulate during the planning).

### M2 onward — commerce + CRM threads in every week

After 2026-06-22 (M2 W5 D1), each daily prompt incorporates both content
work AND commerce/CRM work where natural. M2's revised theme per
MPD-001 §3:

- **M2 W5** (06-22..06-26) — migrations 0002..0008 (sites/posts + RLS
  + users + roles + caps + assignments). Foundation; commerce begins
  W6.
- **M2 W6** (06-29..07-03) — `PgPostRepository` AS PLANNED. Also begin
  `Product` aggregate scaffolding + `ProductRepository` port. Domain
  module restructure (`content/`, `auth/`, `commerce/`, `crm/`) lands
  W6 D1 (carry-over from this session's restructure decision).
- **M2 W7** (07-06..07-10) — JWT + login AS PLANNED. Auth is universal.
- **M2 W8** (07-13..07-17) — Original plan was 5 Posts CRUD endpoints.
  Revised: 3 Posts CRUD (read/list/create) + 3 Products CRUD + 1
  Customer endpoint. RETRO 2026-07-17 incorporates commerce metrics.

---

## 4 · Open carry-over items

### 4.1 W4 D1 work uncommitted

W4 D1 (`403/404/400 matrix on /api/v1/pages/:slug` + thread-local
caps override) is **done** but **not committed**. Files touched:

- `crates/presentation/src/caps.rs` — rewrote with always-on
  `thread_local!` + `with_caps_for_test(set) -> CapsGuard` RAII.
- `crates/presentation/tests/get_page_by_slug.rs` — added 3 new
  tests: `returns_403_when_caps_missing`, `returns_404_when_slug_not_found`,
  `returns_400_when_slug_invalid_chars`. All passing (4/4 in this
  test binary + 3/3 router_smoke = 7 presentation tests).

Next session's first action **before any other work**: commit this W4 D1
with the standard message:

```
feat(ax/g4-w4d1,PLAN-G4): handler 403/404/400 matrix + thread-local caps override for tests
```

(Body per the daily prompt at `docs/session-plans/daily/2026-06-15.md` P5.)

THEN proceed to W4 D2 (xtask capability-coverage real impl) and the
rest of W4.

### 4.2 Binding outcomes (carry-over from 18-commit session)

- **VAL-009** — editor bundle-size guard ≤ 200 KB gz, M3 W3 D5 (2026-08-07)
- **ADR-010 ratification** — Tue 2026-07-21 with binding Simplifier
  counterproposal (remove Option C "graceful fallback" wording)
- **Static-dispatch constraint** for per-variant `EditableBlock`
  impls (M3 W3+)
- **RFC-009 (plugin SDK shape)** — M9 W1 D1 (2027-01-04); **MUST**
  include commerce + CRM hook types per MISSION-V2 §3
- **Capability stub → JWT** — M2 W3 (security debt acknowledged)
- **xxhash-rust ADR** for cross-process cache key stability — M10
  with Dragonfly L2 fan-out
- **ENTITY.md §0 amendment** — pending operator-authorized ENTITY
  spine-touch window; until then MISSION-V2 + MPD-001 are the
  canonical mission statement

### 4.3 Adversary watch (Tier-3 activation forecast)

Per `ENTITY_SYSTEM.md §14` activation matrix, Adversary auto-engages
for:
- Every commerce + CRM endpoint (commerce surface widens attack scope
  per MPD-001 Sentinel verdict)
- Every auth-touching day (JWT lands M2 W3)
- Every public input boundary

Next Tier-3 activation: W4 D2 (when capability-coverage real-impl
gains a real attack surface? — no, it's a CI tool, internal. So Tier-3
remains skipped through W4 D2..D4. Re-engages at M2 W3 JWT and at M2
W6 first Product endpoint.)

---

## 5 · Drift watch

| Detector | Last state | Notes |
|---|---|---|
| D-1 Scope | green | M1 W4 within original plan |
| D-2 ADR drift | dormant | next sweep Friday 2026-06-19 (RETRO day) |
| D-3 Capability | green | xtask capability-coverage gates new handlers from W4 D2 |
| D-4 Bench | dormant | first criterion bench lands M2 W6 (PgPostRepository perf) |
| D-5 Pool mode | green | server boot enforces |
| D-6 Planning trail | green | every commit references PLAN-G* |
| D-7 Architecture | green | hex layers unviolated |
| D-8 Forecast | green | M2 entering state still matches code state |
| D-9 Decision-graph | green | MPD-001 ratification adds 1 node; no contradictions |
| D-10 Memory | info | this file is the canonical state |

---

## 6 · Operator notes from this session (2026-05-26)

The operator pushed the project hard with three "follow optimal plan"
directives, then paused to ask whether the Council protocol was
genuinely engaged. The honest answer (per the analysis written into
the SESSION_LOG before that pause): yes in this session because
governance was in the AI's context window from writing it, but a
fresh session WOULD have missed it unless CLAUDE.md was updated to
point at governance.

The operator authorized the spine touch on CLAUDE.md + ENTITY.md §22.0
in this same exchange. Those edits are now in place. **The next
AVTONOM session will load the Council automatically.**

The operator also announced the commerce + CRM mission expansion in
the same exchange. That is now ratified as MPD-001 and is binding for
all M2+ decisions.

---

## 7 · TL;DR for the next AVTONOM session

```
1. Load CLAUDE.md ## STOP ritual (4 steps).
2. Read this file (you ARE reading it — step 6 of §22.0).
3. Read MPD-001 + MISSION-V2 (they are the new mission frame).
4. First action: commit the pending W4 D1 work (§4.1 above).
5. Then proceed to W4 D2 per the daily prompt at
   docs/session-plans/daily/2026-06-16.md.
6. Carry MPD-001 awareness through W4. At W4 D5 RETRO, produce
   MASTER-PLAN-12-MONTH-v2.html and regenerate M2 daily prompts.
7. M2 onward: commerce + CRM threads interleaved per MPD-001 §3.
```

---

**End of next-day plan.**
Update this file at every session-close to reflect the new
"what's next" state. Append-only memory-drift entries to
`memory/orchestrator_drift_log.md` if any D-* detector trips.
