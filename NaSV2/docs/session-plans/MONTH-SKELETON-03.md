# MONTH-SKELETON M3 · 2026-07-20 → 2026-08-14 · Media pipeline + revisions + editor strategy ADR

> Seed for the 2026-07-17 RETRO → 2026-07-20 bootstrap. Detailed
> daily prompts auto-generate on bootstrap day.

## Phase

P2 Content core (start) — see `MASTER-ROADMAP-2026-2027.md`.

## Assumed entering state (from M2 exit)

- `PgPostRepository` + RLS policy SQL applied per ENTITY §3.3 / §15
- `JwtVerifier` middleware live; caps from JWT
- 5 REST endpoints for Posts (GET single, GET list, POST, PATCH, DELETE)
- nas2-application has `commands::` module with CreatePost / UpdatePost
- ADR-009 (CSRF/nonce surface) written; impl: SameSite-strict cookie
- ~14% WP parity

## Goals

**G1 · ADR-010: block editor frontend strategy**
First architecturally-binding decision of P2. Compares: (a) port
Gutenberg via wasm-bindgen (huge), (b) build Leptos block editor
from scratch (smaller, fewer blocks), (c) read-only first + JSON
textarea admin (ships fastest, technical debt against M7). Decision
gates how M4 block library is consumed.

**G2 · Media model (real)**
Replace M1's placeholder. `Media` aggregate with `mime: Mime`,
`bytes_size: u64`, `checksum_sha256: [u8; 32]`, `original_filename:
String`, `variants: Vec<MediaVariant>`, `alt: String`, `caption:
Option<String>`. `MediaVariant` carries derived sizes per theme
manifest (placeholder until M8). `MediaRepository` impl against
PostgreSQL.

**G3 · libvips workers**
`nas2-image-pipeline` real impl. pgmq queue `ax_image_jobs` consumer
running under `TaskCategory::Image` supervisor. libvips bindings via
the `libvips` crate (already in workspace deps). Variant matrix
hardcoded for M3 (thumb 320×, medium 768×, large 1280×, original).
Variant generation idempotent; resume on crash via pgmq message
re-delivery.

**G4 · Upload endpoint + storage**
`POST /api/v1/media` — multipart/form-data, capability
`media.upload`. `infer` crate for MIME sniffing (reject if header MIME
mismatches sniffed). Virus-scan hook (`trait MediaScanner`) with
default no-op impl; production wires clamav or similar. Storage
adapter `StorageAdapter` trait + local filesystem impl + S3 impl
(both behind features); local-fs default for dev.

**G5 · Post revisions + autosaves**
`revisions` table: snapshot of Post body at every save. New aggregate
`PostRevision { id, post_id, created_at, author_id, snapshot:
Vec<Block>, summary: String }`. Endpoints: `GET /pages/:slug/revisions`,
`GET /pages/:slug/revisions/:id`, `POST /pages/:slug/revisions/:id/revert`.
Autosave protocol: every 30 s the editor PATCHes with `?autosave=true`,
which creates a revision but does NOT bump `posts.updated_at`. RFC-005
documents the protocol.

## Weekly themes

| Week | Dates | Theme |
|---|---|---|
| M3 W1 | 07-20..07-24 | ADR-010 (editor strategy) + Media model |
| M3 W2 | 07-27..07-31 | libvips worker + variant matrix |
| M3 W3 | 08-03..08-07 | Upload endpoint + storage adapter |
| M3 W4 | 08-10..08-14 | Revisions + autosave + RETRO |

## ADRs needed this month

- **ADR-010** — block editor frontend strategy (G1)
- **ADR-011** — media storage adapter contract (S3 vs local-fs vs
  pluggable) (G4)
- *Optional* ADR-012 — virus-scan hook contract (G4)

## Migrations

- `0002_media.sql` — media + media_variants tables
- `0003_revisions.sql` — revisions table
- `0004_rls_media.sql` — RLS policies for media + media_variants

Each with matching `ROLLBACK-NNN-*.md`.

## Exit criteria

1. `POST /api/v1/media` returns 201 with media id; variants generated
   asynchronously and `GET /api/v1/media/:id` shows them when ready
2. `xtask architecture-check / magic-check / capability-coverage /
   check-planning-refs` all green
3. ≥ 3 new integration tests against testcontainers postgres + libvips
4. Revisions accumulating; revert endpoint restores; autosave silent
5. ADR-010 written and committed; M4 plan reflects the choice

## Carry-over seed for M4 bootstrap

- Block library expansion (G1 of M4) depends on ADR-010 decision —
  bootstrap reads ADR-010 first
- Variant matrix is hardcoded in M3; M8 theme installer parameterizes
  it
