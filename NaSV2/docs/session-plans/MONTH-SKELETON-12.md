# MONTH-SKELETON M12 · 2027-03-29 → 2027-04-23 · WP importer + PGO/BOLT + edge + production deploy

## Phase

P5 Migration + production. Final month of year-1 plan.

## Assumed entering state

- All P1–P4 deliverables live
- ~92% WP parity

## Goals

**G1 · WXR (WordPress eXtended RSS) importer**
`nas2-cli import wxr <file> --tenant <id> [--dry-run]`. Parses
WordPress XML export: posts + pages + comments + categories + tags +
attachments. Slug-collision resolver (`?` suffix on conflict).
Attachment URL collection (next goal handles rehosting). Dry-run
emits a per-import report (counts, conflicts, warnings).

**G2 · Image rehoster**
`nas2-cli import rehost <wxr-report-file>` downloads `<wp:attachment_url>`
to our media store, generates variants via M3 pipeline, rewrites URLs
in imported post bodies. Resumable via pgmq job queue.

**G3 · PGO + BOLT real impls**
`xtask pgo-build` real (was stub from M1):
 1. Build `release-pgo-gen` profile (instrumented)
 2. Run canonical benchmark workload (defined in `xtask`)
 3. `llvm-profdata merge` → `/tmp/pgo/merged.profdata`
 4. Build `release-pgo` profile with `-Cprofile-use`

`xtask bolt-optimize` real:
 5. `llvm-bolt` post-link reorder using PGO profile
 6. Output `target/bolt/nas2-server`

Nightly `bench-runner` (already real) compares against
`docs/perf/baseline.json` — regress > 5% → CI issue.

**G4 · Edge integration (Cloudflare Workers)**
`crates/edge-adapter/src/cloudflare.rs` — `wrangler.toml` for edge
deploy. Worker responsibilities:
 - Static delivery from R2 (media, theme assets)
 - Tenant routing from `Host` → upstream pinned by region
 - Image variant negotiation by `Accept` / `DPR`
 - Edge cache (M10 ADR-015 hook here)

**G5 · Production deploy + runbook**
`fly.toml` for Fly.io Machines (preferred per ENTITY §5.2). Staged
rollout: canary → 10% → 50% → 100% via `fly deploy --strategy=canary`.
Health checks `/health/live` + `/health/ready` + `/health/pool`.
On-call runbook: `docs/ops/runbook.md` — common alerts + steps.

## Weekly themes

| Week | Dates | Theme |
|---|---|---|
| M12 W1 | 03-29..04-02 | WXR parser + dry-run + report |
| M12 W2 | 04-05..04-09 | Image rehoster + resumable jobs |
| M12 W3 | 04-12..04-16 | PGO + BOLT real + bench-runner CI gate |
| M12 W4 | 04-19..04-23 | Edge worker + production deploy + RETRO |

## ADRs needed

- ADR-024 — WXR import strictness (strict reject vs lossy with report)
- ADR-025 — production rollout strategy

## Migrations

- `0016_import_jobs.sql` — import state tracking
- `0017_idempotency_keys.sql` — idempotency for rehost jobs

## Exit criteria

1. Real customer WP site imported and served from production deploy
2. `cargo xtask pgo-build && cargo xtask bolt-optimize` produces a
   measurably faster binary (≥ 15% on canonical bench)
3. Edge worker serves cached pages with p95 ≤ 5 ms (edge hit)
4. Production deploy procedure documented; rollback tested in staging
5. ~**100% WP-equivalent core parity**
6. **End of year-1 plan.**

## Year-end deliverable

A self-contained release artifact:
 - Single binary `nas2-server` (PGO + BOLT optimized)
 - Bundled `themes/minimal/`
 - First-party `extensions/seo-basics/`
 - Edge worker for Cloudflare
 - WP importer CLI
 - Production runbook
 - Year-end RETRO documenting state of every M1–M12 goal
 - Year-2 bootstrap (`avtonom-year-2-bootstrap.md`) seeded by the
   year-end RETRO

## Carry-over seed for Year 2

- E-commerce (cart + product CPT + payment gateway)
- Forms (block-based form builder)
- Real-time collaboration (yrs CRDT)
- i18n full implementation
- Plugin marketplace signing infrastructure
- Static export mode
