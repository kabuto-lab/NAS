# PLAN-002 — Search engine · embedded Tantivy

- **Status:** Draft (implementation pending)
- **Date:** 2026-05-25
- **Owner:** AX•ARCHITECT
- **References:** ENTITY §3.10 (read-path), Tantivy 0.22

## Objective

Provide sub-millisecond full-text search for pages, posts, and media labels
without standing up a separate Elasticsearch/Meilisearch cluster. Tantivy is
embedded in `nas2-server`, the index lives on local disk per instance, and
PgBouncer's `worker_pool` drives offline index rebuilds.

## Files

| # | File | Spine? | Action |
|---|------|--------|--------|
| 1 | `crates/search-engine/Cargo.toml` | non-spine | enable workspace `tantivy` dep behind a `search` feature |
| 2 | `crates/search-engine/src/lib.rs` | non-spine | replace stub with module tree (`schema`, `indexer`, `query`, `tenant_scoped`) |
| 3 | `crates/search-engine/src/schema.rs` | non-spine | `tenant_id` (fast field), `kind`, `title`, `body`, `slug`, `updated_at` |
| 4 | `crates/search-engine/src/indexer.rs` | non-spine | full-rebuild + incremental add/replace; checkpoint to disk |
| 5 | `crates/search-engine/src/query.rs` | non-spine | term + phrase + boolean query API, `TenantScoped` filter on every search |
| 6 | `crates/search-engine/src/tenant_scoped.rs` | non-spine | `BooleanQuery::and(Term::exact(tenant_id) + user_query)` — never callable without `TenantId` |
| 7 | `apps/server/src/main.rs` | **spine** | start an indexer task on `worker_pool` `TaskSupervisor` and a shared `Searcher` `ArcSwap` for hot reads |
| 8 | `apps/server/src/router.rs` (new) | non-spine | `GET /search?q=…` handler |

## Multi-tenancy

Every `Searcher::search(query)` requires a `TenantId` argument and prepends
the `tenant_id` term filter inside `tenant_scoped.rs`. Tenant isolation is
**compile-time enforced**: there is no `search(query)` without `TenantId`.

## Rollout

1. Implement schema + indexer with `criterion` bench at 1 M documents.
2. Wire indexer behind `worker_pool` (PLAN-003 must land first).
3. Add `/search` route; measure cold + warm latency.
4. Capture baseline numbers via `cargo xtask bench-runner` (P4 X2).

## Risks

- Index corruption on crash — Tantivy guarantees atomic commits; mitigate
  with periodic snapshot to S3 (next PLAN).
- Disk pressure — 1 M docs ≈ 200 MB; budget 1 GB per instance.
- Reindex storm on schema bump — incremental migration plan required.
