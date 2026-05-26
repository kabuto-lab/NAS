# SESSION PLAN — AVTONOM 2026-05-26 (second call) · bootstrap verify + MPD-001 weave

> **Mode:** AVTONOM (operator: `AVTONOM: avtonom-month-bootstrap-2026-07.md`).
> **Trigger:** operator pasted the M2 bootstrap prompt name as opening
> message. Per `memory/project_next_day_plan.md §3` this is the
> anticipated trigger to enter M2; per AVTONOM doctrine the session
> selected an idempotent verification + MPD-001 weave default instead
> of destructive regeneration.
> **Outcome:** **1 local commit on `main`, no push.**

---

## Why verification, not regeneration

The first AVTONOM call of 2026-05-26 (W4 D2..D5 close-out, commit chain
`7aede89..d42a55a`) **already produced all bootstrap artifacts**:

| Artifact | Status entering this call |
|---|---|
| `AUDIT-2026-06-22.md` | exists (196 lines) |
| `ROADMAP-2026-06.md` | exists (145 lines) |
| `WEEK-05.md..WEEK-08.md` | exist (53/53/53/58 lines) |
| `daily/2026-06-22.md..2026-07-17.md` × 20 | exist |
| `HOW-TO-RUN.md` | exists |
| `SESSION_LOG.md` | exists (208 lines) |

Re-running Phase A..E would have either:
- (a) **overwritten** carefully-reviewed content (destructive), or
- (b) produced duplicate artifacts with stale-relative-to-fresh tension.

**AI-Default applied (logged below):** instead, run **P0 V1..V4
verification gate**, then apply the **MPD-001 weave overlay** that
the first call did NOT inject (the first call predates the operator's
PF1.0 ratification of MPD-001 by < 24 h and missed the weave
instruction).

---

## P0 verification gate · all green

| Gate | Command | Result | Wall | Log |
|---|---|---|---:|---|
| V1 | `cargo check --workspace --all-targets` | ✅ exit 0 | 2 m 01 s | `docs/session-logs/avtonom-2026-05-26-cargo-check.log` |
| V2 | `cargo fmt --all --check` | ✅ exit 0 | < 5 s | `docs/session-logs/avtonom-2026-05-26-cargo-fmt.log` |
| V3 | `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 | ~3 m | `docs/session-logs/avtonom-2026-05-26-cargo-clippy.log` |
| V4 | `cargo test --workspace --lib --no-fail-fast` | ✅ exit 0 | ~1 m | `docs/session-logs/avtonom-2026-05-26-cargo-test.log` |

Workspace state is clean. RETRO §7-3 fmt drift was either resolved
or applies to untracked files only (cargo fmt respects workspace
member list and skips orphan files).

---

## MPD-001 weave overlay applied

10 file ops, all additive, all non-spine:

| # | Op | Path |
|---:|---|---|
| 1 | NEW | `docs/session-plans/MPD-001-M2-WEAVE.md` (overlay spec; 9 sections) |
| 2 | EDIT | `docs/session-plans/ROADMAP-2026-06.md` (+§B7 weave overlay) |
| 3 | EDIT | `docs/session-plans/WEEK-06.md` (+§"MPD-001 weave overlay (W6)") |
| 4 | EDIT | `docs/session-plans/WEEK-08.md` (+§"MPD-001 weave overlay (W8)") |
| 5 | EDIT | `docs/session-plans/AUDIT-2026-06-22.md` (+§A12.5) |
| 6 | EDIT | `docs/session-plans/daily/2026-06-29.md` (+ADD-ON: domain restructure) |
| 7 | EDIT | `docs/session-plans/daily/2026-07-14.md` (+ADD-ON: Product seed) |
| 8 | EDIT | `docs/session-plans/daily/2026-07-15.md` (+ADD-ON: Customer seed) |
| 9 | EDIT | `docs/session-plans/daily/2026-07-16.md` (+ADD-ON: RFC-007 + RFC-008 + caps enum +4) |
| 10 | EDIT | `memory/project_next_day_plan.md` (§3 weave-applied note) |

Weave budget: **4.5 h total** across 4 M2 days, all marked
**OPTIONAL** and deferrable to next-day CARRY-OVER if content track
overruns.

---

## AI-Defaults applied (this call)

| Decision | Choice | Reason |
|---|---|---|
| Bootstrap re-run on already-complete artifacts | **Verification + weave**, NOT regeneration | Regeneration is destructive of reviewed content; verify-then-weave preserves prior work AND lands the MPD-001 instruction that the first call missed. |
| Weave shape: rewrite vs additive ADD-ON sections | **Additive ADD-ON** per daily | Each ADD-ON is bounded, deferrable, and removable. Rewriting destroys the content-track 4-h budget. |
| Domain restructure scope | **W6 D1 (2026-06-29)**, 30 min budget | MPD-001 §3 + project_next_day_plan §3 forecast both pin this date. Mechanical move + re-export; reverts cleanly via `git checkout`. |
| Aggregate scope: seed vs full CRUD | **Seed only** (Product W8 D2, Customer W8 D3) | Full CRUD = repo + migration + endpoint × 3 = 6+ h; out of budget. Seeds give M3 a complete domain to wire up. |
| Where commerce/CRM RFCs land | **W8 D4** (alongside ADR-009 + RFC-005 + RFC-006) | Doc-day already has 4 docs scoped; adding 2 more keeps RFC traffic batched. |
| `git push` | **Never** | AVTONOM rule (§22.3). Operator-only. |

---

## Skipped / not done (deliberate)

| Item | Reason |
|---|---|
| Re-generation of AUDIT/ROADMAP/WEEK/daily | Already complete from first AVTONOM call of 2026-05-26 (`d42a55a`). Idempotent. |
| ENTITY.md §0 amendment per MISSION-V2 | Spine touch; AVTONOM forbids ENTITY.md edits. Pending operator-authorized spine window. |
| Modification of content-track P1..PN in dailies | Out-of-scope of MPD-001 weave; would destroy reviewed content. |
| `apps/server/src/main.rs` change observed in `git status` | Spine file (CLAUDE.md §spine). NOT my edit; pre-existing modification from prior session. Logged but untouched. |
| `cargo update`, `cargo bench` | Bootstrap §HARD STOPS forbids both. |

---

## Drift detectors at session-end

| D | State | Note |
|---|---|---|
| D-1 Scope | green | M2 plan extended via additive overlay; G1..G5 unchanged. |
| D-2 ADR drift | dormant | M2 RETRO 2026-07-17 next sweep. |
| D-6 Planning trail | green | Weave doc references future RFC-007/008 which land W8 D4. |
| D-8 Forecast | green | project_next_day_plan §3 M2 forecast matches weave. |
| D-9 Decision-graph | green | MPD-001 + MPD-001-M2-WEAVE coherent with each other and with content-track. |
| D-10 Memory | repaired | project_next_day_plan §3 + footer updated. |

---

## Working tree at session-end (NaSV2/ scope)

Expected after the verify commit:
```
M docs/session-plans/ROADMAP-2026-06.md
M docs/session-plans/WEEK-06.md
M docs/session-plans/WEEK-08.md
M docs/session-plans/AUDIT-2026-06-22.md
M docs/session-plans/daily/2026-06-29.md
M docs/session-plans/daily/2026-07-14.md
M docs/session-plans/daily/2026-07-15.md
M docs/session-plans/daily/2026-07-16.md
M memory/project_next_day_plan.md
M SESSION_LOG.md (addendum)
?? docs/session-plans/MPD-001-M2-WEAVE.md
?? docs/session-plans/2026-05-26-AVTONOM-bootstrap-verify.md (this file)
?? docs/session-logs/avtonom-2026-05-26-cargo-*.log (gitignored)
```

Plus pre-existing dirty state (NOT in scope of this session):
```
M ../ENTITY.md             (parent repo's ENTITY.md — out of NaSV2)
M apps/server/src/main.rs  (spine; pre-existing; SKIPPED)
M ../ops/caddy/Caddyfile.snippets/cms-ax-pilots.caddy  (out of NaSV2)
M ../ТЗ.html               (out of NaSV2)
?? <many pre-existing NaSV2 untracked files from prior sessions>
```

---

## Next action for operator

The bootstrap is **complete and weave-overlayed**. To start M2:

1. Wait until 2026-06-22 (M2 W5 D1).
2. Open Claude Code in NaSV2/.
3. Paste contents of `docs/session-plans/daily/2026-06-22.md` as opening message.
4. The session will pick up CONTENT-track (RLS migration plumbing).
5. Subsequent days: paste `daily/<today>.md`. MPD-001 weave ADD-ONs trigger on 06-29, 07-14, 07-15, 07-16, 07-17.

Optional (if operator wants pre-flight check on 2026-06-22): the
daily prompt's `P0.5` does the micro-audit automatically — no operator
action needed.

---

**End of second AVTONOM call of 2026-05-26.** Bootstrap verified;
MPD-001 commerce + CRM thread overlay in place. M2 ready.
