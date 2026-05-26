# Mission V2 — Content + Commerce + CRM

> **Status:** binding · v2.0 · 2026-05-26
> **Type:** mission-expansion directive (extends ENTITY.md §0; does NOT supersede).
> **Originating MPD:** [`MPD-001`](master-plan-diffs/MPD-001-commerce-crm-pivot.md).
> **Architectural changes:** additive only — see `MPD-001 §What changes in the platform`.

---

## §0 · The 2026 reality the platform serves

WordPress is the world's dominant CMS by share (~40 % of all websites). It is also the world's dominant **blog engine**. Those two facts are *both* true and *uncorrelated* with where new SMB/SME platform pilots are won in 2026.

The 2026 SMB/SME stack is roughly:
- **Shopify** — storefront + product catalog + checkout
- **HubSpot** — CRM + lead nurture + email marketing
- **Webflow / WordPress** — content / brochure / blog
- **Stripe / payments** — transactional infrastructure
- **Mailchimp / ConvertKit** — broadcast email
- **Intercom / Drift** — live chat / sales conversations

A new product that wants to *displace* this stack does not win by being "yet another WordPress". It wins by being **one Rust-native, multi-tenant, edge-native, capability-gated platform that does Content + Commerce + CRM cleanly enough that pilots stop juggling 5 SaaS subscriptions.**

That is the AX•CMS V2 mission.

---

## §1 · What "additive" means precisely

This is **not a rewrite**. The hex layers, the CONSTITUTION §4 Immutables (Rust / Tokio / Postgres + RLS / PgBouncer txn-mode / pool isolation / Leptos SSR + islands / libvips / ammonia / capabilities / forward-only migrations), and the ENTITY.md §3 stack **all remain binding**.

What changes:
- **Domain breadth** — new aggregates: `Product` · `ProductVariant` · `Order` · `OrderLine` · `Customer` · `Contact` · `Lead`.
- **Domain module layout** — modules grouped under `content/` · `auth/` · `commerce/` · `crm/` namespaces (M2 W5).
- **Capability namespace** — `commerce.*` and `crm.*` join the existing `cms.*`.
- **HTTP surface** — `/api/v1/{products,orders,checkout,customers,contacts,leads}` joins `/api/v1/pages`.
- **Migration sequence** — schema additions interleaved with the original migration plan.
- **Cache-key composition** — gains a `domain_namespace` mixin so content and commerce caches don't collide.
- **Plugin SDK shape (M9, RFC-009)** — gains commerce + CRM hook types.
- **Master plan** — `WP-PLAN-12-MONTH.html` → `MASTER-PLAN-12-MONTH-v2.html` at M2 RETRO.

What does NOT change:
- Rust 1.85+ stable as the only language for production code
- Tokio + Axum + Hyper stack
- PostgreSQL 17+ with RLS as the system of record
- PgBouncer in transaction mode with three-pool isolation
- Leptos SSR + islands as the frontend substrate (no React, no full SPA)
- ammonia sanitize-on-write
- libvips for image pipeline (no `image` crate in hot path)
- §7 performance targets (10-20 ms p95 cached, 10 K+ req/s/core)
- The Council protocol (14 minds; CONSTITUTION; EXECUTION_PROTOCOL)
- The two-layer daily-prompt format (architect.md + senior-dev.md)
- Forward-only migrations
- `#![forbid(unsafe_code)]` workspace-wide

---

## §2 · Why commerce and CRM, not (just) commerce, not (just) CRM

**Commerce alone** = Shopify-clone. The market is saturated and well-served; differentiation is hard.

**CRM alone** = HubSpot-clone. Same story, slightly different cohort.

**Content + Commerce + CRM under one runtime, one tenancy contract, one capability model, one RLS boundary, one observability stack** = a defensible product. The integration story sells:

- A product's content (description, gallery, SEO meta) lives in the same aggregate space as a page's content (`Block` enum reused).
- A customer's CRM record auto-links to their order history (same tenant, same RLS, JOIN, not webhook-glued).
- A lead-nurture sequence that triggers on "viewed product X 3 times" works because everything is in one database, queryable with normal SQL.
- A blog post that converts to a customer is one capability-gated `INSERT INTO contacts` away.

The competitive moat is **integration that the SaaS-stack incumbents physically cannot match without rebuilding their architectures**.

---

## §3 · How this maps to the 12-month Y1 horizon

| Month | Original theme | Revised theme |
|---|---|---|
| **M1** | xtask gates + nas2-common + first GET | unchanged — foundation is shared |
| **M2** | Postgres repos + JWT + 5 Posts CRUD | Postgres repos + JWT + 3 Posts CRUD + 3 Products CRUD + 1 Customer CRUD |
| **M3** | Media pipeline + revisions + ADR-010 | Media pipeline (shared by content+products) + revisions + ADR-010 + Order aggregate + checkout endpoint scaffold |
| **M4** | Block library (15 variants) + Patterns + Reusable + OpenAPI | Block library + ProductBlock variant + Patterns + Reusable + OpenAPI |
| **M5** | Taxonomies + admin shell | Taxonomies (extended to product categories) + admin shell (products + customers views) |
| **M6** | Comments + moderation + spam | Comments + moderation + **product reviews** (same moderation surface) + Lead aggregate |
| **M7** | Admin panel + editor MVP + media browser + 🏆 demo | Admin panel + editor MVP + media browser + **storefront preview** + **CRM contact list** + 🏆 demo |
| **M8** | Themes API + bundled minimal theme | Themes API + minimal-content theme + **minimal-storefront theme** |
| **M9** | Extension API + WASM sandbox + seo-basics plugin | Extension API + WASM sandbox + seo-basics plugin + **stripe-payments plugin** + commerce/CRM hook surfaces |
| **M10** | Caching L1+L2 + invalidation + backup | Caching L1+L2 (gains product-namespace cache keys) + invalidation + backup |
| **M11** | Search (tantivy) | Search across content + products + customers (one tantivy index per tenant, multi-doc-type schema) |
| **M12** | WP importer + PGO + edge + production | WP importer + Shopify CSV importer + PGO + edge + production |

WP-parity stops being the singular success metric. Replaced by a 3-axis triangle: **WP-core / Shopify-core / HubSpot-core** percentages, target geometry by M12 = roughly equal coverage at ~50-30-30 % weight.

---

## §4 · Capability model expansion

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    // === content (existing — unchanged) ===
    CmsPageRead,         // "cms.page.read"
    CmsPagePublish,      // "cms.page.publish"
    CmsPageDraft,        // "cms.page.draft"
    MediaUpload,         // "media.upload"
    ManageUsers,         // "manage.users"
    ManageSite,          // "manage.site"

    // === commerce (NEW M2+) ===
    CommerceProductRead,    // "commerce.product.read"
    CommerceProductManage,  // "commerce.product.manage"
    CommerceOrderRead,      // "commerce.order.read"
    CommerceOrderManage,    // "commerce.order.manage"
    CommerceCheckout,       // "commerce.checkout"     (public-facing)

    // === CRM (NEW M2+) ===
    CrmContactRead,         // "crm.contact.read"
    CrmContactManage,       // "crm.contact.manage"
    CrmLeadRead,            // "crm.lead.read"
    CrmLeadManage,          // "crm.lead.manage"
}
```

`CapabilitySet`, `cache_key_hash`, BTreeSet ordering — unchanged. The expansion is purely a variant addition; no shape change.

---

## §5 · Domain module restructure (lands M2 W5 D1)

**Current** (after M1 W2):
```
crates/domain/src/
├── lib.rs
├── block.rs
├── capability.rs
├── media.rs
├── post.rs
├── role.rs
├── site.rs
└── user.rs
```

**Revised** (lands M2 W5 D1):
```
crates/domain/src/
├── lib.rs                     (re-exports stay flat: pub use content::*; pub use auth::*; ...)
├── content/
│   ├── mod.rs
│   ├── block.rs
│   ├── media.rs
│   ├── post.rs
│   └── site.rs
├── auth/
│   ├── mod.rs
│   ├── capability.rs
│   ├── role.rs
│   └── user.rs
├── commerce/
│   ├── mod.rs
│   ├── product.rs             (NEW — lands M2 W6 D1)
│   ├── variant.rs             (NEW — M2 W6 D2)
│   ├── order.rs               (NEW — M3+)
│   └── order_line.rs          (NEW — M3+)
├── crm/
│   ├── mod.rs
│   ├── customer.rs            (NEW — M2 W8 D2)
│   ├── contact.rs             (NEW — M3+)
│   └── lead.rs                (NEW — M4+)
└── shared/
    ├── mod.rs
    └── slug.rs                (existing validate_slug helper migrates here;
                                 Post + Product both consume)
```

`use nas2_domain::Post;` and `use nas2_domain::Product;` continue to work via flat re-exports in `lib.rs` — call sites don't change.

---

## §6 · Operational impact

| Concern | Impact |
|---|---|
| Single-binary deployment | unchanged |
| Pool budgets (25/10/5) | unchanged at M2 scale; M11 search may need a 4th `search_pool` |
| Observability cardinality | +1 dimension on metric labels: `domain_namespace=content|commerce|crm` |
| Backup story | unchanged (`pg_dump` per-tenant covers all tables) |
| RLS policies | +N policies (one per new tenant-scoped table); Sentinel must verify each |
| Migration count | original M12 finishes at migration 0034; revised projection 0042 |

---

## §7 · Risk register added by MPD-001

| # | Risk | Mitigation owner |
|---|---|---|
| R-MPD-1 | Scope creep — commerce features eat content polish time | Orchestrator at monthly RETRO; Simplifier counterproposals |
| R-MPD-2 | Two product personas (content publisher vs e-commerce operator) pull UX in opposite directions | Productor at M7 demo milestone — refuse to ship the demo if either persona is broken |
| R-MPD-3 | Inventory race conditions on `Order::place` | Sentinel + Chaos at every Order endpoint introduction; M3 OrderRepository contract specifies row-level locking + advisory locks for inventory decrements |
| R-MPD-4 | PCI / payment data scope creep | M9 stripe-payments plugin uses Stripe-hosted checkout → tokenized; AX•CMS never stores PAN. ADR required at M9. |
| R-MPD-5 | CRM PII exposure (GDPR/CCPA) | Sentinel at every CRM endpoint; export/delete-by-customer-id endpoints from M4+ |
| R-MPD-6 | Migration sequence interleaving creates apply-order ambiguity | All migrations stay numerically monotonic; existing 0001..0008 stays Posts-focused; commerce migrations start at 0009 |

These join the existing 7 risks in `MASTER-ROADMAP-2026-2027.md §risk-register`.

---

## §8 · The next-AVTONOM-session directive

The next AVTONOM session opens with the load order from `CLAUDE.md ## STOP` (4 steps + memory). After T0 ritual, it:

1. **Reads `memory/project_next_day_plan.md`** — that file points at THIS file (MISSION-V2) + MPD-001.
2. **Reads MPD-001 + MISSION-V2** — strategic frame loaded.
3. **Proceeds with the daily prompt** for the current real-world date. For M1 W4 (2026-06-15 → 2026-06-19), the daily prompts are *foundation work* (xtask gates, planning backfill, ammonia, RETRO) — unaffected by the pivot.
4. **At the M1 W4 D5 RETRO (2026-06-19)**, the Orchestrator + Migrator + Ecosystem produce the `MASTER-PLAN-12-MONTH-v2.html` and regenerate the M2 daily prompts to weave commerce + CRM into M2-M3.

That is the cleanest way for the existing momentum to absorb the pivot without losing M1 W4.

---

## §9 · ENTITY.md §0 amendment — pending

The full ENTITY.md §0 mission statement is *not amended in this MPD* because the operator (2026-05-26) authorized only spine touches on **CLAUDE.md** (full ritual) and **ENTITY.md §22.0** (bootstrap order). §0 amendment is queued for the next operator-authorized ENTITY-spine-touch window.

Until §0 is amended, MISSION-V2 (this file) + MPD-001 are the **canonical mission statement** for any decision past 2026-05-26. They are loaded at every session-start via the updated `CLAUDE.md` ritual.

---

**End of MISSION-V2-COMMERCE-CRM.**
Read next: [`MPD-001`](master-plan-diffs/MPD-001-commerce-crm-pivot.md) for the operational diff. Read after that: `memory/project_next_day_plan.md` for the immediate next-session directive.
