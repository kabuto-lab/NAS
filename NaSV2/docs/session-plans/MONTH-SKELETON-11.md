# MONTH-SKELETON M11 · 2027-03-01 → 2027-03-26 · Search (tantivy) + reindex pipeline

## Phase

P4 Extensibility + perf (end).

## Assumed entering state

- Caching layers (L1+L2+edge) live; invalidation fan-out working
- Backup story present
- ~86% WP parity

## Goals

**G1 · tantivy index per tenant**
`nas2-search-engine` real impl. Per-tenant index directory under
`data/tenants/<id>/search/`. Schema for Post: title (text), body
(text — flattened from blocks), taxonomy_terms (facet), author_id
(stored u64), status (facet), created_at (date). Read/write through
`SearchIndex` trait.

**G2 · Reindex pipeline**
pgmq queue `ax_search_reindex` (from M1's bootstrap migration —
already created!). Worker in `TaskCategory::SearchIndex` supervisor
consumes messages. Emit on Post insert/update/delete (transactional
outbox pattern). Full reindex CLI: `nas2-cli search reindex --tenant
<id>`.

**G3 · Public search endpoint**
`GET /api/v1/search?q=<query>&tenant=<>` — full-text + faceted by
taxonomy. Capability `cms.page.read`. Paginated. Highlights via
tantivy snippet generator. Cache via M10 layers keyed by
`(tenant + query + page + capability_hash)`.

**G4 · Admin search bar**
Live search island in admin shell — cross-content-type (posts +
comments + media). Debounced + abort-on-keystroke. Uses
`GET /api/v1/search?scope=admin&types=...`.

**G5 · Search performance bench**
`docs/perf/search-baseline.json` — bench: 10 K posts × 100 queries;
target p95 ≤ 50 ms; documented in BENCH-001. PGO build wins ≥ 15%
over LTO-thin (measured).

## Weekly themes

| Week | Dates | Theme |
|---|---|---|
| M11 W1 | 03-01..03-05 | tantivy schema + SearchIndex trait |
| M11 W2 | 03-08..03-12 | Reindex pipeline + transactional outbox |
| M11 W3 | 03-15..03-19 | Public + admin search endpoints |
| M11 W4 | 03-22..03-26 | Bench + RETRO |

## ADRs needed

- ADR-023 — search index storage layout (per-tenant dirs vs single
  index with tenant facet; default: per-tenant for isolation)

## Migrations

- `0015_search_outbox.sql` — transactional outbox table

## Exit criteria

1. Post insert/update → search result updated within 5 s
2. `GET /api/v1/search?q=...` returns hits + highlights + facets
3. Admin search bar live-queries with < 200 ms perceived latency
4. Bench p95 ≤ 50 ms on 10 K-post corpus
5. ~92% WP parity

## Carry-over seed for M12

- Bench setup for M11 carries into M12's PGO measurement
- Per-tenant search dirs need backup integration (extend M10's CLI)
