# MONTH-SKELETON M4 · 2026-08-17 → 2026-09-11 · Block library expansion + patterns + reusable blocks

> Seed for 2026-08-14 RETRO → 2026-08-17 bootstrap.

## Phase

P2 Content core (middle).

## Assumed entering state

- Media pipeline live; uploads + variants working
- Revisions accumulating; autosave protocol documented
- ADR-010 (editor strategy) decided; first block-editor scaffolding
  in `crates/presentation/src/admin/editor/` (if Leptos path) OR
  JSON textarea (fallback path)
- ~21% WP parity

## Goals

**G1 · 15 new Block variants**
Expand `nas2-domain::block::Block` enum from 4 to 19 variants:
List (ordered/unordered + items), Quote (text + cite + attribution),
Table (headers + rows), Cover (image + overlay + text), Group (children),
Columns (column-count + children), Spacer (height), Separator (style),
Buttons (button list + alignment), Gallery (image refs + columns),
Embed (provider + url + caption), Navigation (menu_id), Search,
Latest-posts (count + taxonomy filter). Each: serde shape +
in-module unit test + RFC-001 update section. Validation rules in
`Block::validate(&self) -> Result<(), AppError>`.

**G2 · Block patterns**
`Pattern` aggregate: `{ id, tenant_id, name, blocks: Vec<Block>,
category: PatternCategory }`. Patterns are reusable templates inserted
into posts; the inserted blocks are copies (no reference). Migration
`0005_patterns.sql`. CRUD endpoints under `/api/v1/patterns`.

**G3 · Reusable blocks**
`ReusableBlock` aggregate: `{ id, tenant_id, slug, blocks }`.
Unlike patterns, reusable blocks are *referenced* in posts via
`Block::Reusable { id }`. Updates propagate via cache invalidation
(L1 purge keyed by `tenant + reusable:id`). Migration `0006_reusable_blocks.sql`.

**G4 · Block validation pass at write boundary**
`PostRepository::insert/update` calls `Block::validate` on every
block; reject invalid posts with `AppError::Validation`. Test matrix:
each block variant gets a malformed-input test that asserts the
write is rejected with a specific message.

**G5 · OpenAPI surface (utoipa) + Swagger UI**
Annotate every handler from M1–M4 with `#[utoipa::path]`; mount
Swagger UI at `/api/docs` (admin-capability gated — `manage.site`).
RFC-006 documents the OpenAPI versioning policy (additive only;
breaking changes bump version segment).

## Weekly themes

| Week | Dates | Theme |
|---|---|---|
| M4 W1 | 08-17..08-21 | 15 new Block variants (4-5/day) |
| M4 W2 | 08-24..08-28 | Patterns + reusable blocks |
| M4 W3 | 08-31..09-04 | Block validation + write-boundary tests |
| M4 W4 | 09-07..09-11 | OpenAPI + Swagger + RETRO |

## ADRs needed

- *Optional* ADR-016 — block validation strictness (reject vs auto-fix)

## Migrations

- `0005_patterns.sql`
- `0006_reusable_blocks.sql`

## Exit criteria

1. `Block` enum has 19 variants; all roundtrip JSON
2. `POST /api/v1/posts` with an invalid block payload returns 400
   with the specific error message
3. Patterns CRUD endpoints live; pattern insertion endpoint copies
   blocks
4. Reusable block update propagates to all referencing posts within
   one cache-purge cycle
5. Swagger UI renders at `/api/docs` listing ≥ 15 endpoints
6. ~31% WP parity

## Carry-over seed for M5

- Taxonomies depend on `post_terms` join — design at start of M5
- OpenAPI surface seeds the M7 admin client generation
