# MONTH-SKELETON M5 · 2026-09-14 → 2026-10-09 · Taxonomies + term assignment + admin scaffolding

## Phase

P2 Content core (end) → P3 Admin + themes (start). Bridge month.

## Assumed entering state

- 19 block variants + patterns + reusable blocks shipped
- OpenAPI + Swagger admin-gated
- ~31% WP parity

## Goals

**G1 · Taxonomy domain + repo**
`Taxonomy { id, tenant_id, slug, label, hierarchical: bool, public: bool }`
+ `Term { id, taxonomy_id, parent_id: Option<TermId>, slug, name,
description }`. `TaxonomyRepository` + `TermRepository`. Hierarchical
traversal: `descendants_of(term)`, `ancestors_of(term)`. Migration
`0007_taxonomies.sql` + `0008_terms.sql` + `0009_rls_taxonomies.sql`.

**G2 · Post-term assignment**
`post_terms` join table; `PostRepository::assign_terms / terms_for_post`;
invariant: term belongs to same tenant as post. Migration
`0010_post_terms.sql`. Bulk assign endpoint `PATCH /api/v1/posts/:slug/terms`.

**G3 · Custom taxonomies**
Tenants register taxonomies via admin or seed. Pre-seeded per tenant:
WP-compat `category` (hierarchical) + `tag` (flat). Tenant isolation
strict — taxonomies created per-tenant.

**G4 · Term archive endpoint**
`GET /api/v1/taxonomies/:tax/terms/:slug/posts` — paginated list of
published posts in a term; capability `cms.page.read`; supports nested
term filter (children included by default; `?strict=true` for exact).

**G5 · Admin scaffolding kickoff**
Leptos islands skeleton in `crates/presentation/src/admin/`. Login
form (POST `/admin/login` returning JWT cookie). Dashboard placeholder
listing recent posts. Capability-aware nav menu. NO content editing
yet — that's M7. CSRF surface from M2 carried through.

## Weekly themes

| Week | Dates | Theme |
|---|---|---|
| M5 W1 | 09-14..09-18 | Taxonomy + Term aggregates + repos |
| M5 W2 | 09-21..09-25 | post_terms + assignment endpoints |
| M5 W3 | 09-28..10-02 | Custom taxonomies + term archive endpoint |
| M5 W4 | 10-05..10-09 | Admin shell scaffolding + login + RETRO |

## ADRs needed

- ADR-017 — taxonomy hierarchy depth limits (default: 6 levels)
- ADR-018 — admin URL prefix (`/admin/` vs `/wp-admin/` for WP-compat)

## Migrations

- `0007_taxonomies.sql`
- `0008_terms.sql`
- `0009_rls_taxonomies.sql`
- `0010_post_terms.sql`

## Exit criteria

1. Categories + Tags pre-seeded per tenant; both editable via API
2. Posts assignable to multiple terms across multiple taxonomies
3. Term-archive endpoint paginated and capability-gated
4. Admin shell renders login → dashboard with capability-aware nav
5. ~40% WP parity

## Carry-over seed for M6

- Admin shell is the foundation for M7's editor — invest in routing +
  state plumbing carefully
- Comments depend on `Post`; tenant isolation contracts in M5 carry to
  M6 verbatim
