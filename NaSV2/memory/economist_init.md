---
name: economist-init
description: Economist initial state at 2026-05-26 Adoption Pass — capacity assumptions, cost-curve register, scope-down candidates
metadata:
  type: project
---

# ECONOMIST — Init Dossier (2026-05-26 Adoption Pass)

## Capacity assumptions (Y1 horizon)

| Concern | Assumption | Source / verification |
|---|---|---|
| Tenants per instance | up to 1 K warm (RSS budget: 1 MB/tenant × 1 K + 50 MB idle = ~1.05 GB) | ENTITY §7 |
| Concurrent requests | 10 K/sec/core on cached read; 1 instance × 8 cores = 80 K req/s | ENTITY §7 |
| L1 cache (moka) size | 1 GB per instance, ~ 100 K entries | ADR-006 |
| L2 cache (Dragonfly) | shared cluster, ~ 10 GB hot working set | ADR-006 |
| Pool connections | 25 http + 10 worker + 5 admin = 40 / instance | ENTITY §3.4.2 |
| pgmq throughput | ~ 1 K msg/s/queue (transactional) | ADR-005 |
| NATS JetStream | ~ 100 K msg/s (volume) | ADR-005 |
| Edge cache hit ratio (L3) | target ≥ 80 % at steady state | RFC-002 |

## Cost-curve register

| Subsystem | Per-tenant cost curve | Concern? |
|---|---|---|
| Pages + revisions (Postgres) | O(N posts × avg revisions) | bounded by retention policy (M10 ADR-022 reserved) |
| Media (S3 / local-fs) | O(N media × avg variants) | bounded by variant matrix; M3 W3 |
| Media variant generation (libvips CPU) | O(N uploads × variant count) | worker-bound; backpressure via pgmq vt |
| L1 cache (moka) | O(active tenants × active slugs) | bounded by moka size budget |
| L2 cache (Dragonfly) | O(active slugs × tenant count) | cluster-bound; horizontal scale |
| Search index (tantivy, M11) | O(N posts × N tenants) **per-tenant index** | watch — M11 architect must confirm storage budget |
| Plugin instances (WASM, M9) | O(N plugins × tenant count × active sessions) | bench at M9 W1 D5 (cold-start ≤ 5 ms) |

## Engineer-week ledger (rolling estimate)

| Month | Estimated engineer-weeks | Cumulative |
|---|---|---|
| M1 | 1.0 (4 h/day × 5 days × 4 weeks ≈ 80 h ≈ 2 weeks senior) | 1.0 |
| M2 | 1.5 | 2.5 |
| M3 | 2.0 (editor scaffold + media pipeline) | 4.5 |
| M4 | 1.5 (15 variants + patterns) | 6.0 |
| M5 | 1.5 (taxonomies + admin scaffolding) | 7.5 |
| M6 | 1.5 (comments + spam) | 9.0 |
| M7 | 2.0 (block editor MVP + media browser + demo) | 11.0 |
| M8 | 1.5 (themes + minimal theme; partial year-end) | 12.5 |
| M9 | 2.0 (WASM sandbox + first plugin) | 14.5 |
| M10 | 1.5 (cache layers + backup) | 16.0 |
| M11 | 1.5 (search + reindex) | 17.5 |
| M12 | 2.0 (importer + PGO + edge + deploy) | 19.5 |
| **Y1 total** | **~20 engineer-weeks** | — |

Assumes single-senior engineer + AVTONOM. Drift > 20 % triggers MPD per `ROADMAP_ENGINE.md §6`.

## Infra cost (steady-state estimate)

Per single-tenant pilot, monthly:

| Component | Cost (USD/month) |
|---|---|
| 1× Fly.io / Hetzner app instance (8 vCPU / 16 GB) | ~ $50-80 |
| Postgres 17 (managed, ~ 50 GB) | ~ $50-100 |
| Dragonfly cluster (2 nodes, 4 GB) | ~ $40 |
| NATS JetStream (managed Synadia / self-host) | ~ $20-50 |
| S3 / R2 (media, ~ 100 GB) | ~ $5 |
| Cloudflare (Pro plan + Workers + R2) | ~ $25 |
| Tempo + Pyroscope + Loki (self-host or managed) | ~ $50 |
| **Total / single-tenant pilot** | **~ $240-350/month** |

Per-tenant marginal: dominated by media storage + cache slice. ~ $5-15/tenant/month at steady state.

## Scope-down candidates (kept in reserve)

| Feature | Originally planned in | Scope-down option |
|---|---|---|
| WP-importer (WXR) | M12 W1 | Defer to Year-2 if M12 weeks 1-2 slip; production deploys without WP-import support, customers migrate manually |
| WASM sandbox for plugins | M9 W1-W2 | Compile-time linked first-party plugins only; defer WASM to Year-2 (significant scope, low Y1 user value if marketplace not ready) |
| BOLT post-link optimization | M12 W3 | PGO only; BOLT in Year-2 if measurable win |
| Edge worker (Cloudflare) | M12 W4 | Origin-only deploy; edge in Year-2 |

None of these are pre-decided — they are *options* the Council holds in reserve for re-planning triggers (`ROADMAP_ENGINE.md §6`).

## Constitutional position

The Economist refuses "infinitely scalable" claims without per-unit math (`CONSTITUTION.md §5 F-7`). Every architect's "scales horizontally" requires naming the bottleneck that lifts.
