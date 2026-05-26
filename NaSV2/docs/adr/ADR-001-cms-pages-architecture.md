# ADR-001 — CMS pages architecture (Post + Block + FSM)

- **Status:** Accepted · backfilled 2026-06-17
- **Date:** 2026-06-17
- **Owner:** AX•ARCHITECT
- **References:** [RFC-001](../rfc/RFC-001-cms-pages-publish.md); ENTITY §7, §11, §3.6; [VAL-001](../validations/VAL-001-cms-pages-tests.md)

## Context

RFC-001 establishes *why* AX•CMS rejects the WordPress HTML-blob
shape. ADR-001 documents the *how* — the concrete aggregate, its
sub-types, the state machine, and the alternative considered.

## Decision

### Aggregate shape

```rust
pub struct Post {
    pub id:            PostId,
    pub site_id:       SiteId,
    pub tenant_id:     TenantId,
    pub slug:          PostSlug,
    pub title:         String,
    pub status:        PostStatus,
    pub published_at:  Option<DateTime<Utc>>,
    pub blocks:        Vec<Block>,
    pub custom_type:   Option<CustomPostType>,
}
```

`Post` lives in `crates/domain/src/post.rs`. It is the **only**
aggregate root for the CMS-content sub-domain; `Block` is a value
object owned by the `Post::blocks` vector.

### Block enum (tagged serde repr)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Heading    { level: HeadingLevel, text: String },
    Paragraph  { html: String },
    Image      { src: String, alt: String, caption: Option<String> },
    CodeBlock  { lang: Option<String>, code: String },
}
```

Wire form: `{"type":"heading","level":"h2","text":"…"}`. Tagged so
that grep over `tracing` logs finds blocks by type without ad-hoc
JSON path queries.

### FSM at aggregate level

```text
Draft ─────► Scheduled ─────► Published ─────► Archived
  │                ▲              ▲                │
  └────────────────┴──────────────┘                │
                                                   ▼
                                            (terminal)
```

`PostStatus::can_transition_to(self, target)` is a total function
(`const fn`, exhaustive `match`). The aggregate-level
`Post::transition_to` enforces it; `Conflict` errors carry the
rejected pair so operators read the cause directly.

`published_at` is set on the first transition into `Published`
and *preserved* across `Published → Archived` (soft-delete model).
This is a deliberate invariant — un-archiving via a future
`Restore` command preserves the audit trail.

### Slug as a newtype

`PostSlug::try_new` validates: lowercase ASCII / digits / hyphen,
no leading/trailing hyphen, ≥ 1 char, ≤ 200 chars. Validated at
the boundary; downstream code consumes only valid slugs (so the
404 path for malformed slugs is a 400, not a 404).

## Alternatives considered

### Alternative A — Single `Post::content: String` HTML blob

Rejected. Forces sanitize-on-read (ENTITY §3.6 violation), forces
HTML parsing on the render path (ENTITY §7 violation), forces
fragile regex-based diffing for analytics ("how many H2s did
publish-2026-Q3 contain?").

### Alternative B — Single `Post::content: serde_json::Value` opaque tree

Rejected. Loses typing entirely; FSM at the editor would have to
re-validate per node on every save. The cost saved at parse time
moves to per-write validation time, and the IDE no longer auto-
completes block shapes.

### Alternative C — Separate `Block` table joined per render

Rejected. Adds an N+1 risk that contradicts ENTITY §9.6.
`Vec<Block>` lives inside the JSONB column and the §7 cached-read
budget is realistic only when blocks travel with the post.

## TLA layers

- **L1 Correctness** — typed FSM + total transition function;
  illegal transitions cannot compile.
- **L2 Performance** — render = match-arm dispatch on enum
  variants. JSONB column on the DB side, deserialized via `simd-
  json` (ENTITY §3.11) for the hot path.
- **L3 Scalability** — aggregate fits in a single row; no joins
  on the read path. Cache key (`tenant + slug + capability_hash`)
  achieves §3.9.1 hit-ratio targets.
- **L4 Operability** — `block_type()` static-string tag is the
  metric label; FSM conflicts log with the exact rejected pair.

## Consequences

- (+) Editor / wire / DB / render share a typed contract.
- (+) FSM violations are explicit conflict errors with context.
- (−) Adding a `Block` variant is a coordinated change (editor +
  domain + renderer + serde). Mitigated by `#[serde(other)]`
  catch-all in the future (currently absent — chosen for explicit
  errors).

## References

- [RFC-001](../rfc/RFC-001-cms-pages-publish.md) — strategic intent
- [VAL-001](../validations/VAL-001-cms-pages-tests.md) — verification
- ENTITY §7, §9.6 (no N+1), §11 (typed blocks)
