# RFC-001 — CMS pages publish

- **Status:** Accepted · backfilled 2026-06-17
- **Date:** 2026-06-17
- **Owner:** AX•ARCHITECT
- **References:** ENTITY §7 (perf targets), §3.6 (sanitize on write), §11 (typed content blocks); [ADR-001](../adr/ADR-001-cms-pages-architecture.md); [VAL-001](../validations/VAL-001-cms-pages-tests.md)

## Context

WordPress and most contemporaries store post bodies as HTML blobs in
a single `post_content` column. AX•CMS stores structured content as a
typed `Block` enum (variants: `Heading`, `Paragraph`, `Image`,
`CodeBlock`) for three reasons that compound:

- **Editor-time invariants.** A typed `HeadingLevel` enum makes
  "heading level six" a compile error rather than a runtime branch.
  `PostSlug::try_new` is the gate; downstream code consumes only
  validated slugs.
- **Server-side rendering without HTML parsing on the read path.**
  ENTITY §7's 10–20 ms p95 cached-read budget cannot accommodate a
  parser. Match-arm dispatch on the block enum is O(1) per block;
  HTML parsing is allocation-heavy and parser-dependent.
- **Migration safety.** A new visual element ships as a new enum
  variant + new editor surface — never as ad-hoc HTML drift that
  later requires regex scraping to detect.

## Why

This is a Layer-1 (Correctness) and Layer-2 (Performance) decision
combined. Stored HTML blobs are the WordPress shape; they force the
read path to choose between:

1. Sanitize on read (re-runs ammonia per request — kills §7 perf
   target).
2. Sanitize on write but trust the stored blob forever (fragile —
   any historic XSS becomes a perpetual liability).

Typed blocks side-step the dilemma: the `Paragraph { html: String }`
variant *is* the sanitized HTML, written through `clean_html`
(see [SEC-001](../security/SEC-001-sanitization-pipeline.md)), and
the rendering path is `match arm + write!`.

## Decision

See [ADR-001](../adr/ADR-001-cms-pages-architecture.md).

## TLA layers

- **L1 Correctness** — `Block` derives `Deserialize` with
  `#[serde(tag = "type")]`; unknown variants are explicit JSON parse
  errors, not silent data loss. `PostStatus::can_transition_to` is a
  total function over the FSM; illegal transitions cannot compile.
- **L2 Performance** — rendering = match arm dispatch, zero parsing.
  `block_type()` returns `&'static str` for tracing labels — no
  per-render string allocation. The Post aggregate is `Clone` but
  rarely cloned in the hot path (Axum extracts by reference where
  possible).
- **L3 Scalability** — per-request render cost is bounded by block
  count. Cache key = `tenant + slug + capability_hash` (ENTITY §3.9.1).
  The capability hash ensures editor-visible drafts are not served to
  anonymous readers from a shared cache slot.
- **L4 Operability** — `block_type()` static-string label feeds
  metrics (`render.blocks_total{type="heading"}`). The FSM transition
  function returns `AppError::Conflict` with the rejected pair —
  operators see exactly why a publish failed.

## Consequences

- (+) Editor UX stays typed end-to-end. Drafts in `localStorage`
  cannot ship an unknown block variant.
- (+) Render path stays allocation-light; §7 cached-read budget is
  realistic.
- (+) Sanitization is gated at exactly one place (the write path),
  so an audit knows where to look ([SEC-001](../security/SEC-001-sanitization-pipeline.md)).
- (−) Adding a new variant is a rolling-deploy concern — old
  binaries return a parse error on a new tag. Mitigated via
  expand-only RFC discipline + the M3 W3 `EditableBlock` substrate
  decision (ADR-010, M3 W1 D1).
- (−) Backfilling WordPress imports requires a one-shot HTML → block
  parser (M9 wp-importer extension). Cost is acknowledged.

## References

- [ADR-001](../adr/ADR-001-cms-pages-architecture.md) — the choice
- [VAL-001](../validations/VAL-001-cms-pages-tests.md) — the
  verification trail
- [SEC-001](../security/SEC-001-sanitization-pipeline.md) — sanitize
  on write
- ENTITY §7 (perf targets), §3.6 (sanitize on write), §11 (typed
  content blocks)
