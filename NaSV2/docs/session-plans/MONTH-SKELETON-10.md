# MONTH-SKELETON M10 · 2027-02-01 → 2027-02-26 · Caching L1+L2 + invalidation fan-out + backup story

## Phase

P4 Extensibility + perf (middle).

## Assumed entering state

- Extension API + first plugin
- ~79% WP parity

## Goals

**G1 · moka L1 page cache wiring**
Per-instance `PageCache` keyed by
`(tenant_id, slug, capability_hash, theme_hash, plugin_set_hash)`.
TTL 5 minutes default + per-tenant override + size budget (LRU
eviction). `clear_for_tenant` + `clear_for_slug` operations exposed
for invalidation.

**G2 · Dragonfly L2 wiring (via fred)**
Cluster cache shared across instances. Same key shape as L1.
Fail-open on Dragonfly unavailability (request hits DB; trace metric
emitted). TTL 30 minutes default.

**G3 · Invalidation fan-out via NATS**
On any write that affects cached output (post publish, term rename,
plugin toggle, theme change), emit a `CacheInvalidate { kind, target }`
event on NATS subject `ax.cache.invalidate`. Each instance subscribes
and drops matching L1 entries. Dragonfly's L2 entries deleted by the
emitter directly. Idempotent.

**G4 · ADR-015: edge cache invalidation**
Cloudflare cache-purge API integration (RFC token + zone id);
fired alongside L1/L2 purge for tagged URLs.
For Fastly + others — pluggable `EdgePurger` trait. ADR documents
which edges are first-party-supported.

**G5 · Backup story**
`nas2-cli backup --tenant <id> --out <dir>` produces:
 - `dump.sql` — per-tenant SQL via `pg_dump` with row filters
 - `media/` — S3 sync of tenant's media (rsync-style + checksum)
 - `manifest.toml` — backup metadata + integrity hash chain
Restore: `nas2-cli restore --in <dir>` — verifies integrity then
applies. Tested in CI nightly against a throwaway tenant.

## Weekly themes

| Week | Dates | Theme |
|---|---|---|
| M10 W1 | 02-01..02-05 | moka L1 + cache-key composer |
| M10 W2 | 02-08..02-12 | Dragonfly L2 + fred wiring |
| M10 W3 | 02-15..02-19 | NATS invalidation + ADR-015 + edge purge |
| M10 W4 | 02-22..02-26 | Backup CLI + restore test + RETRO |

## ADRs needed

- **ADR-015** — edge cache invalidation mechanism
- **ADR-022** — backup integrity + restore contract

## Migrations

- `0014_cache_metadata.sql` — opt-in: per-tenant cache config

## Exit criteria

1. Cached read p95 ≤ 20 ms (ENTITY §7 target — finally measurable)
2. Throughput ≥ 10 K req/s/core on cached path
3. Post publish triggers cache purge across all instances within 1 s
4. `nas2-cli backup` + `restore` round-trips a tenant in CI nightly
5. Edge purge fires on configured tenants
6. ~86% WP parity

## Carry-over seed for M11

- Search indexing (M11 G2) emits on the same NATS channel pattern
- Cache key composer is reused by search-result caching
