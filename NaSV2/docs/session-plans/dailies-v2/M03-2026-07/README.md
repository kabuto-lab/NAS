# M3 · 2026-07-20 → 2026-08-14 · Media + Revisions + Editor-Strategy ADR

> **Master plan row (`WP-PLAN-12-MONTH.html`):**
> `<section class="month">` for M3 — theme `Media pipeline (libvips workers) · Post revisions · ADR-010 block-editor frontend strategy`, parity `+7% → 21%`.
>
> **Source-of-truth seed:** `docs/session-plans/MONTH-SKELETON-03.md`.

---

## Why this month is load-bearing

| Reason | Downstream months that inherit |
|---|---|
| **ADR-010** picks the editor UI substrate. | M4 (block library expansion), M7 (block editor MVP — the demo milestone), M9 (plugin block contributions), M12 (WP importer must round-trip into chosen substrate). |
| **Media pipeline (libvips workers)** establishes the first non-HTTP-domain `TaskCategory`. | M6 (comment-author avatars), M7 (media browser), M8 (theme assets), M10 (cache invalidation on media variant ready), M11 (search media filenames + alt-text). |
| **Storage adapter contract (ADR-011)** locks the S3-vs-local-fs boundary. | M10 (backup story), M12 (R2 static delivery via CF edge). |
| **Revisions + autosave** is the first persistent write that decouples user-visible "save" from `posts.updated_at`. | M5 (taxonomy edits inherit revision history), M7 (autosave UX), M10 (cache: revisions are cold; only published rows hot). |

If M3 ships incorrect editor strategy, M7 demo milestone is at risk; if M3 ships a non-idempotent variant pipeline, M10 cache-invalidation fan-out becomes meaningless.

---

## Entering state (assumed from M2 exit · 2026-07-17)

- `PgPostRepository` real impl + RLS applied
- `JwtVerifier` middleware live; caps from JWT
- 5 REST endpoints for Posts (GET single, GET list, POST, PATCH, DELETE)
- `nas2-application::commands::{CreatePost, UpdatePost}`
- ADR-009 (revisions concept) drafted; ADR-010 NOT yet decided
- `~14%` cumulative WP-parity
- `apps/server/src/main.rs` still single-domain (HTTP) — no Image supervisor yet
- `migrations/` at 0008 (the M2 outbox)

If any of the above is **not** true at session start, the day's `architect.md` instructs a CARRY-OVER repair pass.

---

## Goals for M3 (G1..G5)

| G | Title | Lead deliverable | TLA leans |
|---|---|---|---|
| **G1** | **ADR-010 + ADR-011 + RFC-007** | docs only: editor-strategy ADR, storage-adapter ADR, media-model RFC | L1 Correctness (decision), L3 Scalability (S3 vs local), L4 Operability (signing) |
| **G2** | **Media aggregate + PgMediaRepository + 3 migrations** | `crates/domain::Media`, `crates/infrastructure::PgMediaRepository`, `migrations/0009_media.sql`, `0010_media_variants.sql`, `0011_rls_media.sql` | L1 RLS, L2 zero-copy on bytes, L3 partition-by-tenant |
| **G3** | **libvips worker + pgmq consumer + variant matrix** | `crates/image-pipeline::Worker`, `TaskCategory::Image` wired in `apps/server` | L2 sub-ms resize, L3 backpressure via pgmq vt, L4 traceable spans per variant |
| **G4** | **Upload endpoint + StorageAdapter** | `POST /api/v1/media`, `trait StorageAdapter`, `LocalFsStorage`, `S3StorageAdapter` (feature `s3`) | L1 MIME validation, L3 chunked upload, L4 observable upload progress |
| **G5** | **Post revisions + autosave protocol** | `migrations/0012_revisions.sql`, `PostRevision` aggregate, 3 endpoints, autosave query param | L1 invariant (revision belongs to post belongs to tenant), L2 cold-path write, L4 audit trail |

Exit cumulative WP-parity: **21%**.

---

## ADR / RFC / PLAN / VAL slots claimed this month

| Slot | Status entering M3 | Status at M3 exit |
|---|---|---|
| `RFC-007-media-model.md` | not yet | **ratified** |
| `ADR-010-block-editor-strategy.md` | not yet | **ratified** |
| `ADR-011-media-storage-adapter.md` | not yet | **ratified** |
| `ADR-012-virus-scan-hook.md` *(optional)* | not yet | drafted-if-needed |
| `PLAN-005-media-aggregate.md` | not yet | ratified |
| `PLAN-006-libvips-worker.md` | not yet | ratified |
| `PLAN-007-upload-endpoint.md` | not yet | ratified |
| `PLAN-008-revisions.md` | not yet | ratified |
| `VAL-006-media-rls-isolation.md` | not yet | ratified |
| `VAL-007-libvips-idempotency.md` | not yet | ratified |
| `VAL-008-autosave-protocol.md` | not yet | ratified |

`cargo xtask check-planning-refs` MUST pass on every commit referencing these slots.

---

## Migrations claimed this month

| Migration | Day landed | Rollback doc |
|---|---|---|
| `0009_media.sql` | 2026-07-24 (W1 Fri) | `docs/rollback/ROLLBACK-0009.md` |
| `0010_media_variants.sql` | 2026-07-24 (W1 Fri) | `docs/rollback/ROLLBACK-0010.md` |
| `0011_rls_media.sql` | 2026-07-24 (W1 Fri) | `docs/rollback/ROLLBACK-0011.md` |
| `0012_revisions.sql` | 2026-08-10 (W4 Mon) | `docs/rollback/ROLLBACK-0012.md` |

Forward-only, append-only per ENTITY §17. Any DROP COLUMN deferred to a contraction migration in M4+ (≥2-shift gap).

---

## Weekly themes

| Week | Mon — Fri | Theme | Closing artifact |
|---|---|---|---|
| **W1** | 07-20 → 07-24 | ADR-010 editor strategy + Media model | ADR-010, RFC-007, ADR-011, 3 migrations |
| **W2** | 07-27 → 07-31 | libvips worker + variant pipeline | `crates/image-pipeline` real impl + integration test |
| **W3** | 08-03 → 08-07 | Upload endpoint + StorageAdapter | `POST /api/v1/media` e2e green |
| **W4** | 08-10 → 08-14 | Revisions + autosave + **RETRO-2026-08** | `RETRO-2026-08.md` + `avtonom-month-bootstrap-2026-08.md` |

---

## Spine impact (ENTITY §12)

The full month touches spine **only** in three places, each pre-authorized per day:

1. `apps/server/src/main.rs` — wire `TaskCategory::Image` supervisor (W2 D2 · 2026-07-28). Mini-edit; ~15 LOC added.
2. `crates/*/Cargo.toml` — add `nas2-image-pipeline` to `apps/server/Cargo.toml` deps; add `libvips` feature to `crates/image-pipeline/Cargo.toml` (W2 D1 · 2026-07-27).
3. `migrations/` — three new files (W1 Fri) + one (W4 Mon). New migration files are non-spine per §12 ("only additive new migration files are non-spine"); listed here for clarity.

`ENTITY.md`, `CLAUDE.md`, workspace `Cargo.toml`, `docker-compose.dev.yml`, `.env.example`, `clippy.toml`, `rustfmt.toml`, `deny.toml`, `rust-toolchain.toml`, `xtask/src/main.rs` are **NOT** touched this month.

---

## Risk register

| # | Risk | Likelihood | Blast radius | Mitigation pre-decided |
|---|---|---|---|---|
| R-M3-1 | ADR-010 chooses an editor whose bundle size violates ENTITY §3.2 ("avoid > 200 KB WASM gzip"). | M | M7 demo at risk | Decision matrix in W1 D1 includes a hard *bundle-size budget* row; any option > 200 KB gz is rejected before write-up. |
| R-M3-2 | libvips system lib absent on developer machine → W2 D5 integration test red. | M | W2 closes red | Testcontainers image pin to a debian-slim variant that pre-bundles libvips; docker-compose dev override available. |
| R-M3-3 | `pgmq` vt re-delivery semantics misaligned with libvips idempotency contract → silent duplicate variants. | L | data integrity | Variant generation keys on `media_id × variant_kind × theme_hash`; INSERT ... ON CONFLICT DO NOTHING per the M3 plan. |
| R-M3-4 | Multipart upload memory blow-up on > 100 MB images. | M | DoS surface | Tower's `RequestBodyLimitLayer` per route (default 50 MB, override per ENV); stream-to-disk via `tokio::fs` for the original byte stream — no `Vec<u8>` materialisation. |
| R-M3-5 | Revisions table grows unbounded → IO regression by M7. | L | M7 perf | Schema includes `BRIN` index on `created_at`; ADR-022 (slot reserved in M10) will define retention policy. |

---

## Carry-over seeds

**Into M4:**
- Block library expansion (G1 of M4) consumes `ADR-010` decision — bootstrap reads ADR-010 first.
- Variant matrix is hardcoded in M3; M8 theme installer parameterizes it.

**Into M7:**
- Editor substrate (chosen W1 D1) is the demo-milestone target.

**Into M10:**
- Media variant ready event must publish to NATS for L2 / edge cache fan-out — schema is fixed in M3 W3 D5 even though publisher is wired only in M10 W3.

---

## Exit gate

All five conditions must hold at 2026-08-14 EOD:

1. `POST /api/v1/media` returns `201 Created` with media id; variants generated asynchronously; `GET /api/v1/media/:id` shows them when ready.
2. `cargo xtask architecture-check / magic-check / capability-coverage / check-planning-refs` all green.
3. ≥ 3 new integration tests against testcontainers Postgres + libvips.
4. Revisions accumulating; revert endpoint restores; autosave silent (no `updated_at` bump).
5. ADR-010 + ADR-011 + RFC-007 ratified and committed; M4 bootstrap reflects the editor choice.

If any condition fails on 2026-08-14, the RETRO **carries the failure into M4 W1 D1** with explicit catch-up scope; M4 plan adjusts.

---

## Tier-4 Council — appended at 2026-05-26 Adoption Pass

> Per `ENTITY_SYSTEM.md §16`, Tier-4 outputs (Migrator / Ecosystem / Productor) land in the
> monthly README, not the daily artifact. The 2026-05-26 Adoption Pass activated Tier-4 because
> the M3 W1 D1 day (`ADR-010 block-editor strategy`) commits to a public plugin SDK shape and
> the admin UX trajectory — both Tier-4 mandates per `ENTITY_SYSTEM.md §14`.

### MIGRATOR OUTLOOK (M3)

- **Semver impact this month:** **major** at the plugin-SDK boundary (no plugins yet exist, but the *shape* of `BlockSchema` + `EditableBlock` is fixed by ADR-010 Tue 2026-07-21). No public Rust API ships in M3 — the SDK is anticipated for M9 publication. Pre-commitment cost: M9 cannot deviate from the M3 W1 D1 + W1 D2 ratified contract without a Master-Plan-Diff (`ROADMAP_ENGINE.md §4.2`).
- **Vendor-lock surface added:** **none new**. Leptos 0.7 is already Constitution §4 Immutable I-6 (`ENTITY.md §3.2`). No new vendor pinned by ADR-010.
- **Rewrite probability — editor subsystem at 18 months:** **medium**.
  - Driver: Leptos 0.7 → 1.0 transition (announced upstream for late 2026 / early 2027 — within Year-1 horizon).
  - Driver: WASM compilation toolchain churn (`wasm-bindgen`, `wasm-pack`).
  - Mitigation: pin Leptos minor version; Historian tracks upstream RFCs in MONTH-SKELETON forward-look section; M3 W4 D5 RETRO re-evaluates if 0.8 lands by then.
- **Verdict:** **approve.** Action item: M8 ADR-013 (theme rendering) must explicitly reconcile its rendering contract with M3 ADR-010's editor output shape — same `Vec<Block>` ↔ HTML pipeline. If a divergence appears, MPD opens.

### ECOSYSTEM OUTLOOK (M3)

- **Public surface delta:** ADR-010 ratification is the **first commitment** to the plugin SDK surface:
  - `BlockSchema` — serde-derived data shape every block variant declares server-side.
  - `EditableBlock` — Leptos component contract every variant exports client-side (per-variant lazy-loaded island).
  - Manifest format — ADR-010 §Consequences pre-commits the M9 ADR-014 surface (plugin sandbox export shape).
- **Plugin author cost (estimated for a "hello-world plugin" contributing one new block variant):**
  - 1 file declaring `BlockSchema` + serde derives (~ 20 LOC).
  - 1 file declaring `EditableBlock` Leptos component (~ 40 LOC).
  - 1 manifest TOML stub (~ 20 lines).
  - Total: ~ 80 LOC. Below WordPress plugin-skeleton complexity (~ 200 LOC) → competitive ecosystem positioning.
- **Theme contract stability:** decoupled from editor at ADR-013 (M8). Editor produces `Vec<Block>`; theme renders `Vec<Block>` → HTML. Editor changes don't break themes; theme changes don't break the editor. M3 ratification preserves this decoupling.
- **Verdict:** **approve.** Action item: open **RFC-009 (plugin SDK shape draft)** at M9 W1 D1 — ratified during M9; the M3 ADR-010 §Consequences serve as its strawman input.

### PRODUCTOR NOTES (M3)

- **New surfaces this month:** none user-facing in M3 itself (the editor lands UI at M7 W3). M3 is the *commitment* that the M7 demo is reachable. M3 also adds the media-upload endpoint (W3 D1) — UX impact: a `multipart/form-data` POST is a CLI-friendly surface, but the *admin upload UI* lands in M7 (W4 D1).
- **Time-to-first-publish impact:**
  - **M3-M6 phase:** raw-JSON authoring via `POST /api/v1/pages` (M2). Time-to-first-publish for a developer-author: ~ 2 min (curl + JSON in `$EDITOR`). For a non-technical author: blocked until M7.
  - **M7 demo phase:** UI parity with WordPress Gutenberg basics. Time-to-first-publish for a non-technical author: targeting ~ 5 min from login.
- **Error-message audit:** N/A this month (no new user-facing error path lands). M3 W3 D1 `POST /api/v1/media` errors will be audited at that day's Productor pass.
- **Verdict:** **approve.** Watch item: at M7 W3 D5 the Productor re-engages on the editor MVP and, if the editor is at risk of slipping past 2026-12-04 demo day, calls early for a JSON-fallback option (which would itself be a Master-Plan-Diff per `ROADMAP_ENGINE.md §4.2`, not a covert switch — Simplifier counterproposal binding here).

---

**Adoption-Pass status:** Tier-4 Council outputs for M3 are now recorded. Re-activated at M3 W4 D5 RETRO (2026-08-14) for the next-month carry-over assessment.
