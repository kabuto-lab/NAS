# Dailies v2 — Master Index

> Truth table for the 240-day plan. A day exists in v2 ⟺ it appears here
> with status ≥ `drafted`. Status taxonomy in `README.md`.

**Legend:**
`G1` gates/migrations/RLS · `G2` domain/repos · `G3` middleware/auth · `G4` handlers/UI · `G5` docs/tests/RETRO

---

## M3 · 2026-07-20 → 2026-08-14 · Media pipeline + revisions + editor-strategy ADR · parity +7% → 21%

### W1 · 2026-07-20 — ADR-010 (editor strategy) + Media model

| Date | DoW | Goal | Scope (master-plan cell) | Status | Architect | Sr.Dev |
|---|---|---|---|---|---|---|
| 2026-07-20 | Mon | G1 | ADR-010 block-editor strategy — варианты (Gutenberg-wasm / Leptos / JSON+textarea) | **adopted-pilot** ✓ | [link](M03-2026-07/W1-2026-07-20/2026-07-20-mon/architect.md) | [link](M03-2026-07/W1-2026-07-20/2026-07-20-mon/senior-dev.md) |
| 2026-07-21 | Tue | G1 | ADR-010 финализация + RFC-007 media model + ADR-011 storage adapter | planned | — | — |
| 2026-07-22 | Wed | G2 | Media aggregate (Mime / checksum / variants Vec / alt / caption) | planned | — | — |
| 2026-07-23 | Thu | G2 | PgMediaRepository (find / list_for_tenant / insert) | planned | — | — |
| 2026-07-24 | Fri | G2 | migrations 0009-0010 media + media_variants + RLS | planned | — | — |

### W2 · 2026-07-27 — libvips worker + variant matrix

| Date | DoW | Goal | Scope | Status | Architect | Sr.Dev |
|---|---|---|---|---|---|---|
| 2026-07-27 | Mon | G3 | libvips worker scaffold в nas2-image-pipeline (no_capability_required) | planned | — | — |
| 2026-07-28 | Tue | G3 | pgmq consumer + TaskCategory::Image supervisor wiring | planned | — | — |
| 2026-07-29 | Wed | G3 | Variant matrix (thumb 320 / medium 768 / large 1280 / original) | planned | — | — |
| 2026-07-30 | Thu | G3 | Idempotent variant generation (resume на crash через pgmq vt) | planned | — | — |
| 2026-07-31 | Fri | G3 | Worker integration test (testcontainers PG + libvips system lib) | planned | — | — |

### W3 · 2026-08-03 — Upload endpoint + StorageAdapter

| Date | DoW | Goal | Scope | Status | Architect | Sr.Dev |
|---|---|---|---|---|---|---|
| 2026-08-03 | Mon | G4 | POST /api/v1/media multipart upload endpoint (media.upload cap) | planned | — | — |
| 2026-08-04 | Tue | G4 | MIME sniffing via infer crate + MediaScanner trait (virus-scan hook) | planned | — | — |
| 2026-08-05 | Wed | G4 | StorageAdapter trait + LocalFsStorage impl (default для dev) | planned | — | — |
| 2026-08-06 | Thu | G4 | S3StorageAdapter (feature-gated; aws-sdk-s3) | planned | — | — |
| 2026-08-07 | Fri | G4 | Upload e2e test: multipart → variant generation → CDN URL | planned | — | — |

### W4 · 2026-08-10 — Revisions + autosave + RETRO

| Date | DoW | Goal | Scope | Status | Architect | Sr.Dev |
|---|---|---|---|---|---|---|
| 2026-08-10 | Mon | G5 | migration 0011_revisions + PostRevision aggregate | planned | — | — |
| 2026-08-11 | Tue | G5 | RevisionRepository + GET /pages/:slug/revisions | planned | — | — |
| 2026-08-12 | Wed | G5 | Autosave protocol: PATCH ?autosave=true (revision + no updated_at bump) | planned | — | — |
| 2026-08-13 | Thu | G5 | POST /pages/:slug/revisions/:id/revert + provenance тест | planned | — | — |
| 2026-08-14 | Fri | RETRO | RETRO-2026-08 + M4 bootstrap (block library expansion) | planned | — | — |

---

## M4 · 2026-08-17 → 2026-09-11 · Block library expansion (15) + Patterns + Reusable + OpenAPI · +10% → 31%

(stub — entries seeded from `WP-PLAN-12-MONTH.html` on next session)

## M5 · 2026-09-14 → 2026-10-09 · Taxonomies + admin scaffolding · +9% → 40%

(stub)

## M6 · 2026-10-12 → 2026-11-06 · Comments + moderation + spam · +8% → 48%

(stub)

## M7 · 2026-11-09 → 2026-12-04 · Admin panel + editor MVP + media browser 🏆 · +12% → 60%

(stub)

## M8 · 2026-12-07 → 2026-12-31 · Themes API + minimal theme · +10% → 70%

(stub — 18 working days due to year-end)

## M9 · 2027-01-04 → 2027-01-29 · Extension API + WASM sandbox + seo-basics · +9% → 79%

(stub)

## M10 · 2027-02-01 → 2027-02-26 · Caching L1+L2 + invalidation + backup · +7% → 86%

(stub)

## M11 · 2027-03-01 → 2027-03-26 · tantivy search · +6% → 92%

(stub)

## M12 · 2027-03-29 → 2027-04-23 · WP importer + PGO/BOLT + edge + 🚀 production · +8% → 100%

(stub)

---

## M1 / M2 — pending v2 rewrite

User-confirmed (2026-05-26): both M1 (in-flight) and M2 (pre-generated) will be rewritten under v2 contract after M3 pilot validation. Status `pending-v2-rewrite`.

---

## Status roll-up

| Month | Total days | Drafted | Adopted-pilot | Ratified | Executed |
|---|---:|---:|---:|---:|---:|
| M3 | 20 | 0 | **1** | 0 | 0 |
| M4 | 20 | 0 | 0 | 0 | 0 |
| M5 | 20 | 0 | 0 | 0 | 0 |
| M6 | 20 | 0 | 0 | 0 | 0 |
| M7 | 20 | 0 | 0 | 0 | 0 |
| M8 | 18 | 0 | 0 | 0 | 0 |
| M9 | 20 | 0 | 0 | 0 | 0 |
| M10 | 20 | 0 | 0 | 0 | 0 |
| M11 | 20 | 0 | 0 | 0 | 0 |
| M12 | 20 | 0 | 0 | 0 | 0 |
| **Total M3–M12** | **198** | **0** | **1** | **0** | **0** |
| M1 (v1 in-flight) | 20 | — | — | — | — |
| M2 (v1 pre-gen) | 20 | — | — | — | — |
| **Total all 240 days** | **240** | **0** | **1** | **0** | **0** |

### `adopted-pilot` status — definition

Custom status used **once**, during the 2026-05-26 governance Adoption Pass
(per `docs/governance/EXECUTION_PROTOCOL.md §14`). The pilot day's architect.md
and senior-dev.md were Council-reviewed at adoption time even though execution
date is 2026-07-20. Re-review is automatic at session-open on 2026-07-20;
status transitions to `ratified` after the live re-review confirms entering
state matches assumptions.
