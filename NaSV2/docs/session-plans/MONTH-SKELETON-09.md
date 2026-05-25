# MONTH-SKELETON M9 · 2027-01-04 → 2027-01-29 · Extension API + WASM sandbox + first plugin (SEO basics)

## Phase

P4 Extensibility + perf (start).

## Assumed entering state

- Themes API + first theme + admin theme manager
- ~70% WP parity

## Goals

**G1 · ADR-014: WASM sandbox vs compile-time linkage**
Decision: ship both paths.
 - WASM (default) — `wasmtime` + WASI Snapshot 2; per-plugin capability
   tokens via host-imported `caps_require(cap)` host function.
 - Compile-time linkage — for first-party extensions only; `inventory`
   registry; faster cold start.

Reasoning: customer-installable plugins MUST be sandboxed (security);
first-party plugins (SEO basics, analytics) ship with the binary for
cold-start performance. WP doesn't sandbox — we do better.

**G2 · Hook registry**
Typed action/filter equivalents. Action = `Hook<Args, ()>` (fire-and-
forget side effect). Filter = `Hook<Args, Output>` (chained transform).
Deterministic ordering via per-plugin priority. Capability-aware
invocation: hook payload includes `&CapabilitySet`.

**G3 · Plugin lifecycle**
Install → activate → deactivate → uninstall. Sandbox per plugin
(WASM instance reused per request — pooled). Settings API:
`Setting { plugin_id, key, value: serde_json::Value, schema:
SchemaRef }`. Settings UI auto-generated from schema.

**G4 · First plugin: `extensions/seo-basics/`**
Capabilities:
 - `<title>` and `<meta name="description">` injection per Post
 - OpenGraph + Twitter card meta
 - `sitemap.xml` generation (cached, regenerated on post-publish)
 - `robots.txt` generation
 - Canonical URL injection
Ships as both a WASM build AND a compile-time path for demo.

**G5 · Plugin manager UI**
`/admin/plugins`: list installed plugins, activate/deactivate,
settings UI per plugin (auto-generated from setting schema), upload
new plugin archive (capability `plugins.install`).

## Weekly themes

| Week | Dates | Theme |
|---|---|---|
| M9 W1 | 01-04..01-08 | ADR-014 + wasmtime wiring + capability tokens |
| M9 W2 | 01-11..01-15 | Hook registry + plugin lifecycle |
| M9 W3 | 01-18..01-22 | First plugin: SEO basics (sitemap, OG tags) |
| M9 W4 | 01-25..01-29 | Plugin manager UI + RETRO |

## ADRs needed

- **ADR-014** — WASM sandbox model
- **ADR-021** — hook payload contract (serialization + capability gating)

## Exit criteria

1. `extensions/seo-basics/` ships in tree as both WASM + native builds
2. Activated plugin injects `<title>` into rendered pages
3. `GET /sitemap.xml` returns valid Sitemap protocol XML
4. Plugin manager admin page allows toggle + settings edit
5. Bench: WASM plugin invocation cold start < 5 ms p95
   (if > 5 ms, ADR-014 fallback kicks in for that hook)
6. ~79% WP parity

## Carry-over seed for M10

- Hook invocation is on the hot read path → M10 cache must consider
  hook-driven output variance in cache keys
- Plugin lifecycle events emit on NATS — fits M10's invalidation
  fan-out
