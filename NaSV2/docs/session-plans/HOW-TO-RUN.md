# HOW-TO-RUN — AX•CMS month-bootstrap runbook

> Operator runbook for the AVTONOM month-bootstrap system.
> Living document — update as edge cases surface.

---

## 1 · Daily routine (Mon–Fri)

1. Open Claude Code with `cwd = F:\Users\a\Documents\_DEV\Tran\ES\barbie\AX\NaSV2`
2. Read `docs/session-plans/daily/$(today).md`
3. Paste its **entire** content as the opening message (the file starts
   with `AVTONOM:` on line 1 — that triggers AVTONOM mode per `CLAUDE.md`)
4. Hit enter. Do not interrupt unless AVTONOM explicitly stops with a
   HARD STOP (then read `SESSION_LOG.md` for the cause).
5. Session ends when AVTONOM writes its `SESSION_LOG.md` and commits.

PowerShell helper for step 2:

```powershell
Get-Content "docs\session-plans\daily\$(Get-Date -Format yyyy-MM-dd).md" | Set-Clipboard
```

---

## 2 · Missed-day rollover

If a working day is skipped (illness, holiday, planned absence):

1. The next working day's AVTONOM reads its own daily prompt as usual.
2. The previous day's prompt becomes **deferred**. Before pasting the
   new day's prompt, the operator should:
   - Look at the previous day's prompt's P1..PN
   - Pick the highest-leverage subset (often only P1)
   - Manually insert it into the new day's `## CARRY-OVER from yesterday`
     section by editing the daily prompt file *before* pasting

If multiple days are missed in a row, the operator may run only the
most recent day's prompt and let the deferred items absorb into
`RETRO-2026-06.md` `## 5 · Debt acquired`.

---

## 3 · HARD STOP recovery

When an AVTONOM session ends with `HARD STOP`:

1. Read `SESSION_LOG.md` — the cause is in `## Skipped / Blocked`.
2. Fix the underlying issue manually (or run a narrow Claude Code
   session in MANUAL mode to fix it).
3. Re-run the same daily prompt the next working day — the prompt is
   idempotent for P0; P1..PN that already completed will compile-skip.

Most common HARD STOPs and their fixes:

| Symptom | Likely cause | Fix |
|---|---|---|
| `cargo check > 5 iterations red` | breaking change in a workspace dep | revert the offending crate, or pin the dep |
| `Spine-touch without authorization` | day's prompt didn't authorize a needed spine edit | edit the prompt to add explicit authorization for the next run |
| `Disk full / OOM` | accumulated `target/` build artifacts | `cargo clean` then re-run |
| `Internet beyond cargo registry` | day's prompt accidentally needed `gh` / `docker pull` | document as operator action; skip remainder of day |

---

## 4 · Mid-month course correction

If a critical issue surfaces (e.g. CI broken on `main`, security CVE)
that overrides the month plan:

1. Open Claude Code in **MANUAL** mode (no `AVTONOM:` prefix).
2. Fix the urgent issue with normal review-as-you-go discipline.
3. Commit with a clear message referencing the deviation.
4. The next day's AVTONOM picks up where the plan left off; treat the
   manual session as a "ghost day" not counted in the velocity model.

Document any such deviation in the next day's prompt's `## CARRY-OVER`
section so RETRO captures it.

---

## 5 · End-of-month flow

W4 D5's daily prompt (`docs/session-plans/daily/2026-06-19.md`) is
already engineered to:

1. Produce `RETRO-2026-06.md`
2. Produce `docs/session-plans/avtonom-month-bootstrap-2026-07.md`
   (the next month's bootstrap, seeded by the RETRO)

After W4 D5 completes, the operator's only action is:

- On the first working day of the next month (likely 2026-06-22 Mon):
  paste `avtonom-month-bootstrap-2026-07.md` as the opening message.
  Wait ~30–60 min. A new daily/ subdirectory will fill with 20 fresh
  daily prompts.

---

## 6 · Optional: fully hands-off mode

If the operator wants the system to run without daily intervention:

1. Schedule a daily script that opens Claude Code, pastes
   `docs/session-plans/daily/$(today).md`, and lets it run.
2. The bootstrap explicitly does NOT do this automatically — the
   operator must opt in. Reason: pasting unattended into an AI agent
   makes it impossible to react to surprises before they grow.

Sketch (PowerShell, Windows Task Scheduler):

```powershell
# C:\scripts\nasv2-daily.ps1
$today = Get-Date -Format yyyy-MM-dd
$file  = "F:\Users\a\Documents\_DEV\Tran\ES\barbie\AX\NaSV2\docs\session-plans\daily\$today.md"
if (-not (Test-Path $file)) {
    "No prompt for $today (weekend / out-of-month). Skipping." | Out-File -Append "C:\scripts\nasv2-daily.log"
    exit 0
}
$content = Get-Content $file -Raw
# Pipe to Claude Code CLI here — exact invocation depends on installed version
# claude-code-cli --paste-as-opening-message --cwd "F:\...\NaSV2" $content
```

The bootstrap project does NOT ship this script — too brittle. Keep
the operator-in-the-loop default.

---

## 7 · When things genuinely break

If three consecutive daily AVTONOMs HARD STOP for unrelated reasons,
the system is sick. Stop using daily prompts. Open a MANUAL Claude Code
session and:

1. Run pre-flight (PF1–PF4 from `avtonom-month-bootstrap.md`) by hand.
2. Compare current state to `AUDIT-2026-05-25.md`.
3. If the divergence is significant, regenerate a partial AUDIT
   (don't rerun the full bootstrap — too costly) and edit the remaining
   `daily/*.md` files manually to reflect the new reality.
4. Resume daily mode from the next prompt.

---

## 8 · File reference

| Path | Purpose | When to read |
|---|---|---|
| `docs/session-plans/avtonom-month-bootstrap.md` | One-time month bootstrap (this month's source) | At month start |
| `docs/session-plans/AUDIT-2026-05-25.md` | Deep audit driving the month plan | When debugging plan scope |
| `docs/session-plans/ROADMAP-2026-05.md` | 4-week goal map | When prioritizing carry-over |
| `docs/session-plans/WEEK-WW.md` | Per-week breakdown | When a daily prompt references a week-level decision |
| `docs/session-plans/daily/YYYY-MM-DD.md` | Today's AVTONOM opening message | Every working day |
| `SESSION_LOG.md` (repo root) | Last AVTONOM session's report | After every HARD STOP, before next day starts |
| `docs/session-plans/RETRO-2026-06.md` | Month-end retrospective | At month end (auto-generated W4 D5) |

---

## 9 · What this system never does without you

- `git push` (commits stay local; you push)
- create GitHub PR via `gh`
- modify `ENTITY.md`, `CLAUDE.md`, workspace `Cargo.toml`, or any
  other spine file from `ENTITY §12`
- delete files outside `docs/session-plans/`
- run long `cargo bench` jobs (only `--no-run`)
- pull new Docker images
- touch the parent repo (anything outside `NaSV2/`)
- modify CI workflow files

If you need any of those, do them yourself — the system stays
predictable specifically because it never crosses these lines.

---

## 10 · CI gate matrix (as of 2026-05-27)

| Gate | Real? | CI? | Notes |
|---|:---:|:---:|---|
| `cargo fmt -- --check` | ✅ | ✅ | always-on |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ | ✅ | always-on |
| `cargo test --workspace --lib --no-fail-fast` | ✅ | ✅ | `--ignored` integration tests deferred (need Docker in CI) |
| `cargo xtask architecture-check` | ✅ | ✅ | warns on `nas2-presentation → nas2-infrastructure` (Phase-B DI refactor pending — documented waiver in `crates/presentation/Cargo.toml`) |
| `cargo xtask magic-check` | ✅ | ✅ | R1 raw `tokio::spawn` outside supervisor · R2 `lazy_static!` outside registry · R3 `macro_rules!` body > 50 LOC (ENTITY §9.4, §18.2) |
| `cargo xtask check-planning-refs --commit HEAD` | ✅ | ⚠️ NOT wired (full range) | ENTITY §13 commit-message gate (`\b(RFC\|ADR\|PLAN\|VAL)-\w{1,16}\b`). 20/51 historical commits fail; backfill scheduled M1 W4 D3 (G5.P2). CI must use `--commit origin/main..HEAD` range (PR-scoped) until backfill lands. |
| `cargo xtask capability-coverage` | ❌ stub | ✅ (stub passes) | Real impl lands M1 W4 D2 (G5.P1) per `MONTH-SKELETON-01.md`. Stub returns 0 so CI shape stays stable. |
| `cargo deny check` | ✅ (workspace action) | ✅ | Supply-chain · banned crates · GPL contamination · RUSTSEC advisories |

### Local run-all (recommended after any non-trivial commit)

```bash
cargo fmt --all -- --check && \
cargo clippy --workspace --all-targets -- -D warnings && \
cargo test --workspace --lib --no-fail-fast && \
cargo run -p xtask -- architecture-check && \
cargo run -p xtask -- magic-check && \
cargo run -p xtask -- check-planning-refs --commit HEAD~1..HEAD && \
cargo run -p xtask -- capability-coverage && \
cargo deny check
```

### Status as of 2026-05-27 verification sweep

All 4 xtask gates exit 0 against current `main` HEAD:

| Gate | Result |
|---|---|
| `architecture-check` | ok · 16 crates inspected · 1 documented warning (presentation→infra) |
| `magic-check` | ok · 31 files scanned across `crates/` `apps/` `xtask/` |
| `check-planning-refs --commit HEAD~1..HEAD` | ok · 1 commit (HEAD references `PLAN-G1`) |
| `capability-coverage` | stub — implementation pending (planned M1 W4 D2) |

Full-history `check-planning-refs --commit HEAD` reports 20/51 historical violations — **expected**, scheduled M1 W4 D3 backfill. Do NOT wire full-history into CI until backfill commits land.
