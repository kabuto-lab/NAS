# PLAN-001 — Image pipeline · libvips workers

- **Status:** Draft (implementation pending)
- **Date:** 2026-05-25
- **Owner:** AX•ARCHITECT
- **References:** ADR-004, ENTITY §3.7, §9.2

## Objective

Wire a non-blocking image variant pipeline so request handlers never call
`image` crate transforms synchronously. Uploads stay on `http_pool`, the
heavy work runs on `worker_pool` (ADR-003), and the hot read path serves
pre-rendered variants from S3/MinIO.

## Files (spine markers explicit per CLAUDE.md §12)

| # | File | Spine? | Action |
|---|------|--------|--------|
| 1 | `crates/image-pipeline/Cargo.toml` | non-spine | enable `libvips` workspace dep behind a `vips` feature; add `pgmq` client (raw `sqlx`) |
| 2 | `crates/image-pipeline/src/lib.rs` | non-spine | replace stub with module tree (`producer`, `worker`, `variants`, `storage`) |
| 3 | `crates/image-pipeline/src/producer.rs` | non-spine | `enqueue_variant_job(tenant_id, asset_id, mime, bytes_ref)` |
| 4 | `crates/image-pipeline/src/worker.rs` | non-spine | pgmq consumer loop bound to `worker_pool`; idempotency on `(tenant_id, asset_id, variant)` |
| 5 | `crates/image-pipeline/src/variants.rs` | non-spine | theme-configured set (`thumb`, `card`, `hero`, `original`) |
| 6 | `crates/image-pipeline/src/storage.rs` | non-spine | S3 PUT via `aws-sdk-s3` (already in workspace deps) |
| 7 | `crates/infrastructure/src/persistence/asset_variants.rs` | non-spine | repo storing variant URLs keyed by `(tenant_id, asset_id, variant)` |
| 8 | `migrations/NNNN_asset_variants.sql` | non-spine (new) | table + composite index `(tenant_id, asset_id, variant)` |
| 9 | `apps/server/src/main.rs` | **spine** | wire `worker_pool` into a `TaskSupervisor` category; one-line spawn of worker loop |

Step 9 is a mini-edit (spine §12); everything else is non-spine and shippable
without an explicit ok in SEMIAUTO.

## Rollout

1. Implement #1–#7 with unit tests (mocked S3 via `aws-smithy-mocks`).
2. Apply migration; backfill is empty (new table).
3. Smoke benchmark: 100 uploads × 4 variants → p95 worker turnaround < 500 ms
   on a 4-core dev box.
4. Enable producer path in upload handler behind a feature flag
   (`image_pipeline.v2 = true`). Flag default = off in prod for one week.
5. Flip default to on after one week of clean Sentry.

## Validation

VAL spec to be written alongside producer impl (placeholder:
`docs/validations/VAL-004-image-pipeline-libvips.md`).

## Risks

- libvips system dep — must be available on the VPS (apt: `libvips-dev`).
  `Dockerfile` / `vps:after-pull` should `apt-get install libvips-dev` before
  `cargo build --release`.
- pgmq schema not yet created — depends on PLAN-???-queue-pgmq.
