# MONTH-SKELETON M6 · 2026-10-12 → 2026-11-06 · Comments + moderation + spam scaffolding

## Phase

P3 Admin + themes (start).

## Assumed entering state

- Taxonomies live; post-term assignment working
- Admin shell with login + dashboard scaffolding
- ~40% WP parity

## Goals

**G1 · ADR-011: comment hierarchy storage**
Options: (a) adjacency list (parent_id only — simple, expensive
descendant queries), (b) materialized path (single string column —
fast traversal, harder writes), (c) closure table (extra join table
— flexible, more storage). Pick based on expected nesting depth +
read pattern. Default recommendation: adjacency list + computed
`thread_root_id` for pagination batching.

**G2 · Comment model + repo + RLS**
`Comment { id, post_id, tenant_id, parent_id, author_name, author_email,
author_url, body_html, status, created_at, ip_hash, user_agent_hash }`
+ `CommentStatus { Pending, Approved, Spam, Trash }` FSM. `body_html`
is sanitized-on-write per ENTITY §3.6 with a *stricter* ammonia
allow-list than post bodies (no img, no a-with-target). RLS policy
in migration `0012_rls_comments.sql`.

**G3 · Moderation endpoints**
`GET /api/v1/comments?status=pending` (admin), `PATCH /api/v1/comments/
:id/status` (admin transitions), `DELETE /api/v1/comments/:id` (admin —
soft via `Trash`). Capability `comments.moderate`. Admin moderation
list UI (Leptos island).

**G4 · ADR-012 + spam scoring scaffold**
`trait SpamScorer { async fn score(&self, c: &Comment) -> Score; }`
in application layer. Default `HeuristicSpamScorer` impl: URL count >
3 → +0.4; body length < 20 → +0.2; honeypot field set → 1.0;
per-IP throttle exceeded → 0.5. Score ≥ 0.7 → `CommentStatus::Spam`
on submit. ADR-012 lists candidates for production scorer (cleantalk,
Bayesian, ML — defer to Year 2).

**G5 · Public submit endpoint + rate limit**
`POST /api/v1/posts/:slug/comments` (no auth required if site allows
anon comments; capability `cms.comment.write` if logged in).
`tower-governor` rate limit per IP (10/min) + per-tenant (1000/min).
Honeypot hidden field expected in payload (rejected if present).

## Weekly themes

| Week | Dates | Theme |
|---|---|---|
| M6 W1 | 10-12..10-16 | ADR-011 + Comment model + repo |
| M6 W2 | 10-19..10-23 | Moderation endpoints + admin moderation island |
| M6 W3 | 10-26..10-30 | Spam scorer + ADR-012 |
| M6 W4 | 11-02..11-06 | Public submit + rate limit + RETRO |

## ADRs needed

- **ADR-011** — comment hierarchy storage
- **ADR-012** — spam scoring strategy

## Migrations

- `0011_comments.sql`
- `0012_rls_comments.sql`

## Exit criteria

1. Comment thread on a published post renders at
   `GET /api/v1/posts/:slug/comments` (paginated by thread)
2. Public submit returns 201 + comment id; status auto-Pending or
   auto-Spam depending on heuristic
3. Moderator can transition Pending → Approved / Spam / Trash via API
4. Rate limit returns 429 with `Retry-After` header
5. ~48% WP parity

## Carry-over seed for M7

- Admin moderation island foundation reused by M7 post editor
- ADR-011 storage choice affects M7 thread-rendering performance
- Comments stress-test cache invalidation (impacts M10 design)
