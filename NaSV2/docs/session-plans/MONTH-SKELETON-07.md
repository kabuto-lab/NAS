# MONTH-SKELETON M7 · 2026-11-09 → 2026-12-04 · Admin panel: post list + post editor (block UI MVP)

## Phase

P3 Admin + themes (middle). **First demo-worthy milestone month.**

## Assumed entering state

- Comments + moderation live
- Admin shell + dashboard + nav skeleton from M5
- ADR-010 (editor strategy) from M3 dictates frontend approach
- ~48% WP parity

## Goals

**G1 · Admin route guards + nav rendering**
Per-route capability guards in `nas2-presentation::admin::router`.
Nav menu renders only items the current `CapabilitySet` permits. JWT
caps source from M2 + cookie session. Breadcrumbs via `Path<…>`
extractor sequence.

**G2 · Post list view**
Leptos island `PostList`: paginated table, filter by
status/author/term, sortable. Bulk-select + bulk-status-change. Server-
rendered first page; client-side hydration for filters. Optimistic
status changes with rollback on error.

**G3 · Block editor MVP**
The big one. Path A (Leptos): edit canvas → block insert → block
reorder (drag/drop or up/down) → block delete → block edit (per
variant: text input for Heading, rich-text for Paragraph, file picker
for Image). Save draft every 30 s via M3 autosave path. Path B (JSON
fallback): textarea + JSON schema validation + Markdown preview pane.
Choice dictated by ADR-010.

**G4 · Media browser island**
Grid view with filtering (type/date/tenant). Upload directly from
editor (calls M3 upload endpoint). Alt-text inline edit + save.
Server-rendered thumbnails via M3 variant `thumb_320`.

**G5 · User & role manager**
List users + assign roles + capability matrix view (read-only; role
editing is for super-admin). Endpoints already exist from M2/M5; M7
adds the UI layer.

## Weekly themes

| Week | Dates | Theme |
|---|---|---|
| M7 W1 | 11-09..11-13 | Route guards + nav + post list |
| M7 W2 | 11-16..11-20 | Block editor canvas + insert/reorder/delete |
| M7 W3 | 11-23..11-27 | Block editor per-variant edit + autosave |
| M7 W4 | 11-30..12-04 | Media browser + user manager + RETRO |

## ADRs needed

- ADR-019 — Leptos island state-management pattern (signal-based vs
  context vs leptos_router state) — picked once, applied everywhere

## Exit criteria

1. Operator logs in at `/admin/login`, navigates to Posts, opens a
   draft, edits blocks, saves draft, publishes — entire flow works
2. Media browser usable; alt-text editable
3. Role view renders without "loading…" stall (server-rendered)
4. Test: `cargo test -p nas2-presentation --tests` integration tests
   cover admin happy path
5. ~60% WP parity — **first demo milestone**

## Carry-over seed for M8

- Block editor talks JSON to the API; themes API in M8 dictates how
  rendered output composes
- Media browser carries forward into theme manager UI
