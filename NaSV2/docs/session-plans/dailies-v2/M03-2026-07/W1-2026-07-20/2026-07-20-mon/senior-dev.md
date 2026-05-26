AVTONOM: AX•CMS · M3 · W1 · D1 · G1.P1 — ADR-010 block-editor strategy (option study)
Дата: 2026-07-20 (Mon). Senior Rust architect prompt — concrete ADR skeleton, decision
matrix template, verification commands, pitfalls. Вопросов не задавать. Не пушить.

> **Sibling:** [`architect.md`](architect.md) — read it once on session-open to lock
> the strategic frame, then operate from this file.

═════════════════════════════════════════════════════════════════════
CARRY-OVER from 2026-07-17 (M2 W4 D5 · RETRO close)
═════════════════════════════════════════════════════════════════════

  - HEAD on `main`: <fill from `git log -1 --format=%h main` at session start>
  - V1..V4 green at M2 close; `cargo xtask architecture-check / magic-check /
    capability-coverage / check-planning-refs` all green.
  - Repo deltas since 2026-07-17: 0 (weekend); working tree must be clean.
  - WP-parity: 14% (M2 exit per `MASTER-ROADMAP-2026-2027.md`).
  - Last new ADR slot used: `ADR-009-revisions.md` (M2 W4 D4). Next free
    expected: `ADR-010`. **Verify before assuming the slot.**
  - Carry-over flag from M2 RETRO §recommendations: none open.

═════════════════════════════════════════════════════════════════════
CONTEXT
═════════════════════════════════════════════════════════════════════

Entering state:

  - `docs/adr/` contains slots up to `ADR-009-revisions.md` ratified.
  - `docs/rfc/RFC-001-block-library.md` ratified at M1 W4 D3.
  - `crates/domain/src/lib.rs` exports `Block` enum with 4 variants
    (`Heading`, `Paragraph`, `Image`, `CodeBlock`) — verify by
    `Grep -n "enum Block" crates/domain/src/`.
  - `MONTH-SKELETON-03.md` §Goals · G1 reads: *"First architecturally-binding
    decision of P2. Compares: (a) port Gutenberg via wasm-bindgen (huge), (b)
    build Leptos block editor from scratch (smaller, fewer blocks), (c) read-only
    first + JSON textarea admin (ships fastest, technical debt against M7)."*
  - `WP-PLAN-12-MONTH.html` M3 cell row 1 confirms the Mon scope.
  - `dailies-v2/INDEX.md` row 2026-07-20 status = `drafted` (set by this session
    after the ADR file lands).
  - **No spine touches authorized today** — see architect.md §6.

Exit state expected by EOD:

  - `docs/adr/ADR-010-block-editor-strategy.md` lands as a ≥ 600-line Madr-v3
    ADR with the 5-driver × 3-option matrix populated.
  - `dailies-v2/INDEX.md` row updated to status `drafted` with links live.
  - `SESSION_LOG.md` at repo root overwritten with the AVTONOM template
    (§ below).
  - Single commit on `main`, not pushed, message references `ADR-010`,
    trailer `AI-Assisted: AX-ARCHITECT (Claude Opus 4.7)`.
  - `cargo xtask check-planning-refs` exits 0 on the new commit.

═════════════════════════════════════════════════════════════════════
SCOPE
═════════════════════════════════════════════════════════════════════

P0 · Verification gate (V1..V4 → docs/session-logs/avtonom-2026-07-20-cargo-*.log)
  V1 `cargo check --workspace --all-targets`
  V2 `cargo fmt --all -- --check`
  V3 `cargo clippy --workspace --all-targets -- -D warnings`
  V4 `cargo test --workspace --lib --no-fail-fast`
  Up to 3 iterations if red (lower than usual because today is docs-only;
  any V1..V4 redness blocks the day and triggers a green-restore phase
  before the ADR work).

P1 · Slot probe
  - `Glob docs/adr/ADR-*.md` → confirm `ADR-010-*` is free.
  - If occupied, claim next free integer; record in
    `SESSION_LOG.md §AI-Defaults applied`.
  - If `ADR-009-revisions.md` is absent, abort to a CARRY-OVER repair
    phase: today's primary deliverable shifts to "ratify ADR-009",
    and ADR-010 work slips to Tue. Update `INDEX.md` accordingly.

P2 · Evidence gathering (read-only, ~30 min budget)
  - `Read docs/rfc/RFC-001-block-library.md` — record:
    - the 19-variant target for M4
    - the explicit "block tree is the data; renderer is the presentation" boundary
  - `Read docs/adr/ADR-009-revisions.md` — record:
    - autosave granularity expectations (whole-document snapshot per save)
    - any constraint on the editor protocol
  - `Read docs/session-plans/MONTH-SKELETON-{04,07,09,12}.md §G1` — for each,
    note any assumption the month makes about the editor substrate.
  - `Grep -n "enum Block" crates/domain/src/` — verify M1 4-variant state.
  - `Grep -n "200 KB" docs/ENTITY.md` and §3.2 — read the exact wording of
    the bundle-size ceiling.

P3 · Bundle-size empirical references (no measurement today — research only)
  Compile these figures into the ADR §Pros/Cons matrix, citing the source
  in the matrix footnote:

  | Substrate | Stated bundle size (gz, citation expected) | Status under §3.2 |
  |---|---|---|
  | Gutenberg-wasm (WP 6.x → wasm-bindgen via Yew, 2025 prototype) | 2.4 MB gz (per public Gutenberg-wasm POC, 2025) | **❌ violates 200 KB ceiling** |
  | Native Leptos block editor (4 variants, M3 state) | ~80–120 KB gz est. (extrapolated from `leptos_axum` 0.7 starter + 4 minimal islands) | **✓ within ceiling** |
  | Native Leptos block editor (19 variants, M4 target) | ~140–200 KB gz est. (linear extrapolation; lazy-loaded per-variant islands keep p95 closer to 100 KB) | **✓ at ceiling — needs careful island splitting** |
  | JSON-textarea admin | 0 KB (no client JS; pure server-rendered form) | **✓ trivially within ceiling** |

  If a candidate's number cannot be sourced credibly, mark it
  `[evidence: TBD M3 W2 D1 spike]` and proceed.

P4 · ADR-010 draft file — full skeleton

  Path: `docs/adr/ADR-010-block-editor-strategy.md`
  Length target: 600–900 lines including code fences.
  Format: Madr v3 (https://adr.github.io/madr/).

  Skeleton (paste, then fill the bracketed fields):

  ```markdown
  # ADR-010 · Block-Editor Frontend Strategy

  - **Status:** Proposed (to be Accepted on 2026-07-21 W1 D2)
  - **Date:** 2026-07-20
  - **Deciders:** AX•ARCHITECT, human operator (ratifies on 2026-07-21)
  - **Consulted:** RFC-001 (block library shape), ADR-009 (revisions),
    MONTH-SKELETON-{04,07,09,12} (downstream consumers)
  - **Supersedes:** none
  - **Superseded-by:** none

  ## Context and Problem Statement

  AX•CMS exposes content authoring via a block-tree data model
  (`nas2-domain::Block`, RFC-001). Authors interact with this model through
  an editor. M7 (2026-11) requires a demo-grade block editor; M9 (2027-01)
  requires plugin-contributed block types; M12 (2027-04) requires WXR-import
  round-tripping. The editor substrate choice gates all three.

  How should the admin block editor be implemented such that
  (a) the in-browser bundle stays within ENTITY §3.2's 200 KB gz ceiling,
  (b) the data model in `nas2-domain::Block` remains the single source of
  truth (no shadow representation in the editor), and
  (c) the substrate can scale from 4 variants (M1) to 19+ variants (M4) and
  plugin-contributed variants (M9) without re-architecture?

  ## Decision Drivers

  1. **Lossless serialization** — round-trip `Block` ↔ editor state without
     semantic loss; required for M9 plugin variants and M12 importer.
  2. **Bundle size ceiling** — ENTITY §3.2 hard limit: client JS+WASM bundle
     for `/admin/posts/:slug/edit` ≤ **200 KB gzipped**. Violation requires
     an exemption ADR.
  3. **Variant scalability** — the substrate must accept new `Block`
     variants without per-variant client-side machinery beyond a
     declarative registration call.
  4. **Operability / debuggability** — server log must show the exact block
     tree authored; client → server boundary must be JSON-serializable for
     tracing (no opaque binary blobs).
  5. **M7 demo viability** — a usable editor with insert / reorder / edit /
     autosave by 2026-12-04 (M7 RETRO day).

  ## Considered Options

  ### Option A — Gutenberg-wasm bridge (port WP Gutenberg into Rust client)

  **Description.** Reuse WP's mature block editor by compiling it to WASM
  (e.g. via the 2025 Gutenberg-wasm POC) and bridging to Rust via
  `wasm-bindgen`. Editor state lives in Gutenberg's React tree; our server
  receives WP's `serialize_blocks()` HTML-comment-delimited output and
  parses it back into `nas2-domain::Block`.

  Pros:
    - Massive WP-ecosystem familiarity (authors transitioning from WP feel
      no friction).
    - All 90+ WP blocks "for free" — accelerates M4 if compatible.
    - WP plugin block code can theoretically be loaded with shims.

  Cons:
    - **Bundle size:** ~2.4 MB gz observed in the 2025 POC — 12× over §3.2
      ceiling. **Hard-fails decision driver #2.**
    - React + Redux toolkit + Lodash drag-in — violates ENTITY §3.2 "JS
      only where strictly required".
    - HTML-comment block serialization is a lossy encoding for our block
      shape (e.g. `Block::CodeBlock { lang }` needs careful attr
      round-tripping).
    - WP plugin code expects WP REST API surface, which AX•CMS will never
      faithfully implement.
    - Eternal upstream-merge tax (Gutenberg moves; we'd need to track).

  Quantitative score: **0/5 drivers passed.**

  ### Option B — Native Leptos block editor

  **Description.** Build a Leptos-based block editor as a set of islands
  hydrated server-side per ENTITY §3.2 ("Leptos SSR + Islands"). Each
  `Block` variant maps to a Leptos component implementing a
  `EditableBlock` trait. The editor's working state IS a `Vec<Block>` —
  no shadow model. JSON over `fetch` for autosave; HTMX-style server-rendered
  first paint.

  Pros:
    - Lossless: editor working state IS `Vec<Block>`; no encoding boundary.
    - Bundle size budget: ~80–120 KB gz for 4 variants (M3 baseline);
      ~140–200 KB gz at 19 variants with per-variant lazy hydration.
    - Plugin variants (M9) register a `EditableBlock` impl + a server
      `BlockSchema` — symmetric extension model.
    - Server log captures the literal `Vec<Block>` POSTed at autosave —
      maximum debuggability.
    - First paint server-rendered → time-to-interactive dominated by
      island hydration, not bundle download.

  Cons:
    - Build cost — M3 W3-W4 + M7 W3-W4 are non-trivial Leptos work.
    - Drag/drop ergonomics in Leptos are less mature than React's
      ecosystem.
    - 19-variant target sits AT the bundle ceiling — careful island
      splitting required (M4 architect must monitor).
    - No "for free" advanced blocks (mediastack, embeds with provider
      shims) — those have to be built or deferred.

  Quantitative score: **5/5 drivers passed** (with monitoring on #2 from M4).

  ### Option C — JSON-textarea minimal admin (read-only-first)

  **Description.** Ship M3-M5 with an admin that renders posts read-only
  + a `<textarea>` accepting a JSON `Vec<Block>` payload validated
  server-side. Editor work deferred — the "editor" is the operator's text
  editor of choice; the admin form posts the JSON.

  Pros:
    - Zero client JS — bundle size driver passes trivially.
    - Ships fastest — M3 W3 frees up entirely for the media pipeline
      buffer (today's W1 budget is unrelated).
    - Lossless trivially — the JSON IS the block tree.
    - All M9 plugin variants "work" because the operator can type the
      JSON for any variant.

  Cons:
    - **Driver #5 fails:** by M7 demo we need a click-to-edit UI;
      JSON-textarea ships demo as a JSON window. Unshippable as a demo.
    - All editor work pushed to M7; M7 has 20 days and ~12% parity to
      cover — adding the editor build there crowds out the media
      browser + user manager surfaces.
    - WP authors will hate it during pilot.

  Quantitative score: **3/5 drivers passed** (#2, #3 driver-by-fallback, #4)
  — but driver #5 is a structural fail.

  ## Decision Outcome

  **Chosen:** **Option B — Native Leptos block editor.**

  Rationale:

  1. Only option that passes all 5 decision drivers.
  2. Substrate-thin enough that the M7 demo is reachable in the planned
     M7 W3 schedule (the 5 days are explicitly themed "Block editor MVP
     (insert / reorder / edit)" per `WP-PLAN-12-MONTH.html`).
  3. Aligns with ENTITY §1.1 ("reject JavaScript-heavy approaches") and
     §3.2 ("avoid large WASM bundles"); rejects Option A on a hard
     numeric ceiling.
  4. Option C is retained as a **graceful fallback only** — if M3 W3
     reveals that the upload-endpoint work is at risk because M3 W1-W2
     Leptos scaffolding cost too much, a Tue-2026-07-21 scope review may
     temporarily ship Option C for M3-M5 with explicit M7 catch-up
     scheduled.

  ## Pros/Cons Decision Matrix

  | Driver | A · Gutenberg-wasm | B · Leptos editor | C · JSON-textarea |
  |---|---|---|---|
  | 1 · Lossless serialization | ⚠ HTML-comment lossy on attrs | ✓ direct `Vec<Block>` | ✓ JSON is the tree |
  | 2 · Bundle ≤ 200 KB gz | ❌ ~2.4 MB | ✓ 80-120 KB (4v) / 140-200 KB (19v) | ✓ 0 KB |
  | 3 · 19-variant scalability | ⚠ "free" via WP, but plugin-API tax | ✓ per-variant `EditableBlock` | ✓ trivial |
  | 4 · Debuggability | ⚠ JS bridge opaque | ✓ POST body IS the tree | ✓ trivial |
  | 5 · M7 demo viability | ⚠ if upstream API matches | ✓ click-to-edit UI | ❌ unshippable as demo |
  | **Score** | **0/5** | **5/5** | **3/5 + structural fail #5** |

  ## Consequences

  **Inherited by M4 (block library expansion, 15 new variants):**
    - Each new `Block` variant ships with a paired Leptos
      `EditableBlock` impl in `crates/presentation/src/admin/blocks/<name>.rs`.
    - Bundle-size budget owner = M4 architect; must monitor `cargo
      leptos build --release` size at each variant addition.

  **Inherited by M7 (block editor MVP):**
    - M7 W3 D1-D5 schedule is unchanged — canvas + insert + reorder + edit
      + autosave all assume Option B substrate.
    - Drag/drop ergonomics: M7 W3 D3 uses `keyboard up/down` as the
      primary reorder UX, drag/drop as enhancement; reflects Option B's
      Leptos drag-API maturity gap.

  **Inherited by M9 (plugin block contributions):**
    - WASM-sandboxed plugins (ADR-014, M9) export both a server-side
      `BlockSchema` + a client-side `EditableBlock` impl (compiled to WASM
      and lazy-loaded as a separate island bundle).
    - Hook registry (M9 W2 D1) gains a `block_register` hook type.

  **Inherited by M12 (WXR importer):**
    - WP block-comment serialization is parsed into `Vec<Block>` at import
      time (W12 W1 D1-D2 work); editor never sees WP's HTML-comment form.

  **Not inherited / explicitly out of scope today:**
    - Drag/drop physics — M7 owns.
    - Embed-block provider shims — punted to M9 plugin authors.
    - Real-time collaborative editing — Year-2 scope (`MASTER-ROADMAP
      §year-2-candidates`).

  ## Links

  - RFC-001 — block library data model (this ADR's input)
  - ADR-009 — revisions + autosave (this ADR's protocol partner)
  - ADR-013 — theme rendering substrate (M8; consumer of the chosen
    editor's frontend output contract)
  - ADR-014 — WASM sandbox (M9; consumer of the EditableBlock + Schema
    plugin export contract)
  - `WP-PLAN-12-MONTH.html` M3 W1 D1 / M4 W1 / M7 W3 / M9 W2 / M12 W1
  - ENTITY.md §3.2 (frontend stack), §9.3 (forbidden: giant frontend
    hydration), §9.10 (forbidden: oversized WASM admin)
  ```

P5 · INDEX.md update

  Open `docs/session-plans/dailies-v2/INDEX.md`. Find the row:

  ```
  | 2026-07-20 | Mon | G1 | ADR-010 ... | **drafted** ✓ | [link]... | [link]... |
  ```

  Verify the row already exists; if it shows `planned`, change to `drafted`.
  Also update the §"Status roll-up" section to reflect the new drafted count.

P6 · Sanity sweep

  - `cargo check --workspace --all-targets` — must remain green (docs-only
    change must not regress).
  - `cargo xtask check-planning-refs` — exits 0 because the commit message
    references `ADR-010`.
  - `Grep -n "ADR-010" docs/` — at least 3 hits expected (the ADR itself,
    `INDEX.md` row, `M03-2026-07/README.md` updated ADR-status row,
    `architect.md` `forward inheritance`, `senior-dev.md` this file).
  - `Glob docs/adr/ADR-010-*` — exactly one file.

P7 · Commit

  Stage exactly:
    - `docs/adr/ADR-010-block-editor-strategy.md`
    - `docs/session-plans/dailies-v2/INDEX.md`
    - `docs/session-plans/dailies-v2/M03-2026-07/README.md` (only the ADR-010
      status row + the "ADR / RFC / PLAN / VAL slots claimed" table)
    - `SESSION_LOG.md`

  Do NOT stage:
    - Anything under `crates/`, `apps/`, `xtask/`, `migrations/`.
    - `architect.md` or `senior-dev.md` of today (they shipped on
      2026-05-26 in the pilot session — not today's commit).

  Commit message:

  ```
  docs(ax/m3-w1-d1,ADR-010): block-editor strategy option study (Proposed)

  Three options compared against five decision drivers; native Leptos
  block editor wins 5/5; Gutenberg-wasm rejected on §3.2 bundle ceiling;
  JSON-textarea retained as graceful M3 fallback only.

  Status: Proposed. Acceptance lands on 2026-07-21 (W1 D2) after 24-hour
  soak. Forward links to ADR-013/-014 + MONTH-SKELETON-04/-07/-09/-12.

  AI-Assisted: AX-ARCHITECT (Claude Opus 4.7)
  ```

  No `git push`. No `--amend`. Verify with `git log -1 --format=%s` that
  the message is exact.

═════════════════════════════════════════════════════════════════════
EXPECTED CLIPPY ALLOWS (and WHY)
═════════════════════════════════════════════════════════════════════

  None. Today is docs-only — no Rust code touched. If V3 (clippy) flags
  anything, it is a pre-existing issue unrelated to today's work and is
  recorded as a `SKIP` in `SESSION_LOG.md §Skipped/Blocked`. Do not chase.

═════════════════════════════════════════════════════════════════════
COMMON PITFALLS
═════════════════════════════════════════════════════════════════════

  1. **ADR slot collision.** `docs/adr/` may have ratified an `ADR-010`
     in a parallel branch since 2026-07-17. P1 probe is mandatory; pick
     the next free integer if 010 is taken, and log the substitution in
     `SESSION_LOG.md §AI-Defaults`.
  2. **Madr v3 strictness.** Some review tools (`adr-tools`) reject
     ADRs missing `## Status` as the first H2. Skeleton above puts it
     first; keep it that way.
  3. **Quoted bundle-size figures.** The 2.4 MB Gutenberg-wasm figure
     is from a public 2025 POC; if a recent benchmark contradicts it,
     update the cell — but **do not** rely on a hypothetical "future
     Gutenberg shrinks". The decision today binds on today's evidence.
  4. **ADR length inflation.** A 1500-line ADR signals indecision.
     Cap at ~900 lines including matrix; if longer, factor out the
     "Considered Options" detail to `docs/adr/ADR-010-supplements/`.
  5. **Commit message scope.** Trailer is `AI-Assisted: AX-ARCHITECT
     (Claude Opus 4.7)` — exact string; case-sensitive; check-planning-refs
     parses for `ADR-010` token.
  6. **Forward-link rot.** ADR-013 and ADR-014 don't exist yet at
     2026-07-20. Reference them by *number* in the §Links list as
     "ADR-013 (planned M8)" / "ADR-014 (planned M9)" — DO NOT
     fabricate hyperlinks. `check-planning-refs` only requires the
     token; the link is human reference.
  7. **MONTH-SKELETON cross-link.** The `MONTH-SKELETON-NN.md` files
     are markdown — link relatively: `../../session-plans/MONTH-SKELETON-04.md`.
     The path depth from `docs/adr/ADR-010-*.md` is `../session-plans/...`.
  8. **`git diff` includes binary noise.** If `target/` shows in `git
     status`, `.gitignore` is OK (verified). Don't `git add -A`; stage
     each file by name.
  9. **VS Code "Save All" pitfall.** On Windows, a stray BOM in the
     new ADR file makes `cargo xtask check-planning-refs` happy but
     `prettier` (if installed locally) reflows; force UTF-8 LF on
     save. Verify with `file docs/adr/ADR-010-*.md` (Linux) or
     `Get-Content -Raw | Format-Hex | Select-Object -First 1` (PS) —
     first bytes must be `0x23 0x20 0x41 0x44` (`# AD`), not
     `0xEF 0xBB 0xBF` (UTF-8 BOM).
  10. **Russian text mixing.** The ADR is in English by team convention
      (`MASTER-ROADMAP-2026-2027.md` precedent). Russian inline phrases
      from the master plan cell ("варианты") should be translated in
      the ADR; preserve them only in §Links as the verbatim
      master-plan quote.

═════════════════════════════════════════════════════════════════════
PRE-RESOLVED DEFAULTS (AI-Defaults — recorded in commit if used)
═════════════════════════════════════════════════════════════════════

  • ADR template — Madr v3 (rationale: AX project default).
  • ADR slot — `010` if free; else next free integer.
  • Length budget — ≤ 900 lines.
  • Decision outcome — Option B (Leptos) — refute & rewrite if today's
    evidence overturns the prior.
  • Discarded 4th option (HTMX-only forms) — mentioned in §Considered
    Options ¶ rejected; no matrix row.
  • Bundle-size source for Gutenberg-wasm — 2025 POC (cite as
    "public 2025 Gutenberg-wasm prototype, ~2.4 MB gzipped main bundle";
    if exact figure unavailable, use "≥ 2 MB" and flag with [evidence:
    needs source] in the ADR — but do NOT block ratification on this).
  • Bundle-size source for Leptos editor — extrapolated; flag as
    "[evidence: spike to confirm on M3 W3 D5]" in the ADR.
  • Language — English (project precedent).

═════════════════════════════════════════════════════════════════════
HARD STOPS
═════════════════════════════════════════════════════════════════════

  1. Spine-touch attempt (ENTITY.md, CLAUDE.md, workspace Cargo.toml,
     etc.) — ABORT, log, defer.
  2. `V1..V4` red after 3 iterations — green-restore phase replaces ADR
     work today.
  3. `Glob docs/adr/ADR-009-revisions.md` returns nothing — CARRY-OVER
     repair: ratify ADR-009 first, ADR-010 slips to Tue.
  4. Disk full / OOM / `cargo` registry unreachable beyond P0 retry —
     ABORT.
  5. Operator interrupts — yield gracefully; save WIP commit with
     `WIP:` prefix; ask before continuing.

═════════════════════════════════════════════════════════════════════
ЗАПРЕЩЕНО АБСОЛЮТНО
═════════════════════════════════════════════════════════════════════

  • `git push`, `gh pr create`, `git push --force`.
  • `git commit --amend` on any commit not authored in this session.
  • Editing `ENTITY.md`, `CLAUDE.md`, `Cargo.toml` (workspace),
    `docker-compose.dev.yml`, `.env.example`, `clippy.toml`,
    `rustfmt.toml`, `deny.toml`, `rust-toolchain.toml`,
    `apps/server/src/main.rs`, `xtask/src/main.rs`,
    `migrations/*.sql` (applied).
  • Adding new workspace dependencies.
  • Touching `crates/`, `apps/`, `xtask/` source files.
  • Deleting files outside the explicit SCOPE list.
  • Running `cargo bench` (not even `--no-run` today — irrelevant).
  • Running `xtask bench-runner` (creates baseline noise on dev machine).
  • Refactor beyond P0–P7.
  • Fabricating numeric evidence (bundle sizes, benchmarks) in the ADR.
    If unknown, mark `[evidence: TBD spike]`.
  • Generating `ADR-013` / `ADR-014` content (those are M8 / M9 scope).
  • Closing M3 W1 D2 / D3 / D4 / D5 work today (one day at a time).

═════════════════════════════════════════════════════════════════════
SESSION_LOG.md формат (overwrite at NaSV2/ root)
═════════════════════════════════════════════════════════════════════

```markdown
# SESSION_LOG — AVTONOM 2026-07-20 HH:MM (M3 W1 D1 · G1·P1)

> Today's daily prompt: `docs/session-plans/dailies-v2/M03-2026-07/W1-2026-07-20/2026-07-20-mon/senior-dev.md`.
> Today's architect brief: `…/architect.md`.

## Outcome — one line per phase

| Phase | Outcome |
|---|---|
| P0 V1..V4 | green / amber-1-retry / red-blocked |
| P1 slot probe | ADR-010 free / claimed slot ADR-NNN instead |
| P2 evidence gather | RFC-001 + ADR-009 + MONTH-SKELETONs read |
| P3 bundle-size references | populated 3/3 / flagged TBD on N rows |
| P4 ADR draft | landed at NNN lines, all 8 sections populated |
| P5 INDEX update | done / pending |
| P6 sanity sweep | green |
| P7 commit | <SHA> · pushed: NO |

## Plan (detailed status)

(per-phase narrative; bullet points OK)

## AI-Defaults applied

| Decision | Choice | Reason |
|---|---|---|
| (e.g. ADR slot fallback) | (e.g. used 011 because 010 taken) | (e.g. parallel branch landed first) |

## Skipped / Blocked

| Item | Reason | Suggested follow-up |
|---|---|---|

## Commits made (local, not pushed)

| Phase | SHA | Title |
|---|---|---|

## Recommendations for human review

(bullet list — what the operator should read first, what to push back on,
what to look for in tomorrow's session)

## Working tree at end of session

```
git status --short
```

## Time budget

- Wall: HH:MM
- Tokens / iterations on V1..V4: N
- Reads / Writes / Greps / Bashes: counts if helpful

## CARRY-OVER for tomorrow (2026-07-21 W1 D2)

(One block, pre-drafted for tomorrow's session log header. Lists:
- HEAD sha tonight
- ADR-010 status (`Proposed` after today; tomorrow ratifies to `Accepted`)
- Any blockers tomorrow must resolve before P1 (e.g. "ADR-009 was missing,
  was ratified late today, so tomorrow can proceed without slip"))
```

═════════════════════════════════════════════════════════════════════
END OF PROMPT
═════════════════════════════════════════════════════════════════════

---

## Council Engineering Pass

> **Appended at the 2026-05-26 Adoption Pass** per `EXECUTION_PROTOCOL.md §14`.
> All Tier-1 + Tier-2 entities reviewed this future-dated prompt at adoption time.
> Tier-3 entities skipped with documented reason. Tier-4 outputs live in the
> monthly README (per `ENTITY_SYSTEM.md §16`).

### FORGEMASTER MEMO

- **Allocation budget (this change):** **N/A — docs-only day.** No Rust code touched. Per-request allocation budget for the *future implementation* of Option B (Leptos block editor) is the binding question, owned by M3 W3 + M7 W3 sessions. Today merely fixes the decision.
- **Lock surfaces touched:** none.
- **Async boundary cost:** none.
- **Bench target / VAL slot:** **VAL-009 reserved** — bundle-size guard for the chosen substrate. Trigger: M3 W3 D5 (first `cargo leptos build --release` against the M3 4-variant set). p95 bundle ≤ 200 KB gz per ENTITY §3.2. Re-run at every variant addition during M4.
- **Static-dispatch / dyn call sites:** Today: zero. *For Option B implementation:* the `EditableBlock` trait is implemented per-variant; Forgemaster mandates **static dispatch via monomorphized generics** on the per-variant component path. `Box<dyn EditableBlock>` is forbidden in the rendering hot loop — admissible only at the variant-discriminator boundary if benches show monomorphization bloat. ADR-010 §Consequences must explicitly state this constraint when ratified Tue.
- **Verdict:** **approve.** Caveat captured: Tue's ratification must add the static-dispatch constraint to §Consequences. VAL-009 slot claimed.

### SENTINEL RISK AUDIT

- **Failure modes named (≥ 1 required):**
  1. **ADR-010 ratifies with vague Option B language → M3 W3 implementation drift.**
     - Detector: Historian D-2 weekly check against ADR §Consequences; Forgemaster bundle-size bench M3 W3 D5.
     - Recovery: superseding ADR-010.1 — *not* silent re-interpretation.
  2. **Leptos 0.7 → 1.0 transition mid-year breaks the editor substrate.**
     - Detector: `cargo.lock` minor-version pin + manual `crates.io` watch on `leptos` major releases.
     - Recovery: M3 RETRO scopes a substrate-port spike; if cost > 2 weeks, MPD opens.
  3. **Plugin-contributed `EditableBlock` impls (M9) become an exploitable client surface — XSS via plugin block render.**
     - Detector: Adversary (M9 W1 D2) — review ADR-014 sandbox boundary.
     - Recovery: enforce `ammonia` sanitize-on-write at the plugin-published-content boundary (ENTITY §3.6 Immutable I-8 already binds — verified consistent).
  4. **JSON-textarea fallback path covertly survives past M5 → dual-track admin burden.**
     - Detector: Simplifier counterproposal addresses this preemptively (see below).
     - Recovery: Tue ratification removes the fallback wording per Simplifier's reduction.
- **Threat surfaces:** none widened today (no public-facing change). Future-binding: the ADR's choice of Leptos commits us to the WASM client surface — sandboxing of plugin-contributed islands becomes Adversary's M9 burden.
- **Rollback path:** revert single commit; ADR file deleted; decision returns to "open"; M3 plan adjusts at next session-start (RETRO not required for a revert at `Proposed` status).
- **Observability hook:** none added today. Future: ADR-010 §Consequences must require a `tracing` span per editor mount + per autosave POST (M7 W3 D5 implementation cost).
- **Data-integrity invariant:** none touched. Future-binding: ADR-010 commits the system to *editor working state IS `Vec<Block>`* — the invariant "no shadow representation of the block tree exists in client memory" must be enforced by code review at M3 W3 onward.
- **Verdict:** **approve.** All four failure modes have named detectors + recovery paths. No "no concerns" verdict (avoids §2.3 Constitution violation).

### SIMPLIFIER COUNTERPROPOSAL

- **Removable surfaces:**
  1. **The 4th-option HTMX mention** in §Considered Options ¶ rejected. Status: keep — costs 3 lines, prevents "did you consider HTMX?" from re-emerging at a future review.
  2. **Option C's "graceful fallback during M3-M5" provision** inside §Decision Outcome. Status: **DELETE.** Reason: optionality at the decision boundary creates dual-track risk and an implicit "covert path-switch" that bypasses the MPD pipeline. If M3 W3 implementation slips, that becomes an explicit MPD event (`ROADMAP_ENGINE.md §4.2`), not a silently-activated fallback.
  3. **The 4-variant vs 19-variant bundle-size table in P3** of senior-dev.md (the table on the architect's bundle research). The information is in §Pros/Cons of the ADR — duplication. Status: keep in P3 because P3 is a *research scaffold* the AVTONOM consumes; the ADR is the *durable artifact*.
- **Concrete reduction:** remove from ADR-010 §Decision Outcome the sentence *"Option C is retained as a graceful fallback only — if M3 W3 reveals that the upload-endpoint work is at risk because M3 W1-W2 Leptos scaffolding cost too much, a Tue-2026-07-21 scope review may temporarily ship Option C for M3-M5 with explicit M7 catch-up scheduled."* Replace with: *"Option C is rejected (structural fail on driver #5). Any future scope-pressure to revisit must open a Master-Plan-Diff (MPD) per `ROADMAP_ENGINE.md §4.2`."*
- **Cost of keeping:** 60 words of dual-track optionality; +1 implicit decision boundary; mid-month scope-switch becomes possible without operator visibility.
- **Cost of removing:** if Leptos scaffolding genuinely slips in M3 W3, the MPD pipeline adds a 24-48 h ratification overhead vs the current "silent switch". Acceptable; MPDs exist to make scope-pressure visible, not to slow it.
- **Verdict:** **reduce — binding for Tue 2026-07-21 ratification.** Tue's session opens with this counterproposal pre-resolved; ADR-010 lands at `Accepted` *with* this edit. Recorded in commit message as `AI-Default: Option C fallback wording removed per Simplifier counterproposal (CONSTITUTION §2.2)`.

### ECONOMIST LEDGER

- **Infra delta this work:** **$0** today (docs only). Future: Option B implementation is the dominant cost line for M3-M7. Engineer-week estimate (from Forgemaster's prior + comparable Leptos editor projects): ~5 engineer-weeks total = M3 W3 (1 week, scaffold) + M3 W4 (0.5, autosave wiring) + M4 W4 (0.5, per-variant `EditableBlock` factory) + M7 W2-W3 (3, full MVP). No new infra dependencies; no new vendor.
- **Per-tenant scaling:** editor is **O(1) per browser tab**. WASM bundle is per-load (or cached at edge); no per-tenant compute on the server beyond the existing handler path. Per-tenant *storage* impact of revisions (M3 W4) is the binding scaling concern — Economist re-engages at M3 W4 D5 on the retention ADR slot.
- **Maintenance cost:** per-variant `EditableBlock` impl = 1 new test fixture (Rust unit + a Leptos render smoke test). M4 introduces 15 variants → 15 new fixtures (~3 LOC each in the harness, ~30 LOC each per impl). Total M4 fixture cost: ~500 LOC test surface. On-call surface: zero (admin-only path, no SLO).
- **Cheaper variant considered:** Option C (zero engineering today, ~3 engineer-weeks salvage in M7 to retrofit UI). Rejected on structural driver #5 (Productor concurs — JSON-textarea unshippable as a demo).
- **Verdict:** **accept.** Cost trajectory is bounded and amortizes across M3 (1.5 weeks) + M4 (0.5 weeks) + M7 (3 weeks). No infra cost surprise.

### Tier-3 — skipped with documented reason

- **Council: ADVERSARY skipped** — reason: docs-only day; no public-input boundary changed; no new attack surface widened. The ADR has *future-binding* security implications (plugin block render in M9, theme rendering in M8); those are reviewed when the implementation lands, not today.
- **Council: CHAOS skipped** — reason: docs-only day; no distributed coordination touched; no queue / cache / supervisor change.
- **Council: TEST PILOT skipped** — reason: docs-only day; no hot path; no perf-target surface introduced. VAL-009 (Forgemaster) is the scheduled bench engagement at M3 W3 D5.

### Tier-4 — landed in monthly README

See `dailies-v2/M03-2026-07/README.md` §Migrator Outlook / §Ecosystem Outlook / §Productor Notes — appended at this Adoption Pass.

---

**Adoption-Pass status:** this `senior-dev.md` is now Council-reviewed. The Simplifier counterproposal is **binding for Tue 2026-07-21**: ADR-010 lands at `Accepted` with the Option C fallback wording removed.
