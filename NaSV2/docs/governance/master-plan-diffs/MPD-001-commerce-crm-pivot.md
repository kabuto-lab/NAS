# MPD-001 · Commerce + CRM mission expansion

> **Status:** Ratified — operator-approved 2026-05-26.
> **Type:** Master-Plan Diff per `ROADMAP_ENGINE.md §4.2`.
> **Scope:** **additive** — does NOT supersede the WP-replacement framing; *extends* it.
> **Affects:** M2 onward. M1 (foundation) unchanged.
> **Source RETRO:** none — this is an out-of-cycle MPD triggered by operator instruction. See `ROADMAP_ENGINE.md §6` re-planning triggers ("operator declares re-planning").

---

## Motivation (operator-stated, 2026-05-26)

> "WordPress начинался как блоговая система. В 2026 она не актуальна особо.
> Нужна витрина магазина и CRM."
>
> *(Translation: WordPress began as a blogging system. In 2026 it is not
> especially relevant. What's needed is a storefront and CRM.)*

WordPress's 2026 reality:
- ~40 % of the web's CMS market by raw share, but
- blog-as-product framing is no longer where new SMB/SME pilots are won,
- Shopify (commerce) + HubSpot (CRM) own that mindshare,
- a Rust-native, multi-tenant, ENTITY §7-performance replacement targeting **content + commerce + CRM as one platform** is a more defensible product position than "another modern blog engine".

The existing AX•CMS foundation (hex layers, RLS, pool isolation, capability model, Leptos SSR + islands) was designed generic enough to host commerce + CRM features alongside content. This is **not a rewrite**; it is a scope-extension MPD.

---

## What changes in the platform

### Mission (ENTITY.md §0) — additive

**Before:** "next-generation CMS platform intended to replace WordPress at enterprise scale".

**After (effective M2 RETRO 2026-06-19):**
> Next-generation **content + commerce + CRM** runtime intended to replace the WordPress + Shopify + HubSpot SMB/SME stack with a single Rust-native, multi-tenant platform at enterprise scale and edge-native rendering.

ENTITY.md §0 itself is **not edited in this MPD** (operator did not explicitly authorize the §0 spine touch). The new framing lives:
- here (MPD-001),
- in `docs/governance/MISSION-V2-COMMERCE-CRM.md` (comprehensive rationale + architectural impact),
- and is read at every session-start via the updated `CLAUDE.md ## STOP` ritual.

§0 itself will be amended at the next ENTITY-spine-edit window with operator sign-off — Tracked.

### Stack (ENTITY.md §3) — **unchanged**

The CONSTITUTION §4 Immutables (Rust / Tokio / Postgres+RLS / PgBouncer transaction / pool isolation / Leptos / libvips / ammonia / capabilities / forward-only migrations / no-unsafe / single-binary / 12-month horizon / two-layer prompts) **all hold**. Commerce + CRM are domain additions, not platform rewrites.

### Domain — new aggregates (new modules under `crates/domain/`)

| Aggregate | Module | Purpose | First lands |
|---|---|---|---|
| `Product` | `commerce/product.rs` | sellable item: id, slug, title, description blocks (reuse `Block` enum!), price, currency, inventory | M2 W5+ |
| `ProductVariant` | `commerce/variant.rs` | size/color/etc; SKU; price delta | M2 W5+ |
| `Order` | `commerce/order.rs` | OrderId, customer, lines, status (FSM: Cart→Pending→Paid→Fulfilled→Refunded/Cancelled), totals | M3+ |
| `OrderLine` | `commerce/order_line.rs` | product_variant + qty + unit_price snapshot | M3+ |
| `Customer` | `crm/customer.rs` | id, tenant, email, name, addresses, lifecycle status | M2 W5+ |
| `Contact` | `crm/contact.rs` | id, tenant, type (lead/customer/partner), email, source | M3+ |
| `Lead` | `crm/lead.rs` | id, contact_id, status (FSM: New→Qualified→Disqualified/Converted), source-tracking | M4+ |

The existing `Post` / `Page` / `Site` aggregates **stay** and gain a sibling-namespace structure under `crates/domain/`:

```
crates/domain/src/
├── lib.rs
├── content/        ← existing: site.rs, post.rs, block.rs, media.rs
├── auth/           ← existing: capability.rs, role.rs, user.rs
├── commerce/       ← NEW: product.rs, variant.rs, order.rs, ...
├── crm/            ← NEW: customer.rs, contact.rs, lead.rs
└── shared/         ← cross-cutting (validate_slug stays here)
```

### Capabilities (ENTITY §14) — new namespaces

```rust
pub enum Capability {
    // existing — content namespace
    CmsPageRead, CmsPagePublish, CmsPageDraft,
    MediaUpload,
    ManageUsers, ManageSite,
    // new — commerce namespace
    CommerceProductRead, CommerceProductManage,
    CommerceOrderRead, CommerceOrderManage,
    CommerceCheckout,            // public storefront checkout
    // new — CRM namespace
    CrmContactRead, CrmContactManage,
    CrmLeadRead, CrmLeadManage,
}
```

This is +9 variants — manageable. `as_str()` strings: `commerce.product.read`, `commerce.product.manage`, `commerce.order.read`, `commerce.order.manage`, `commerce.checkout`, `crm.contact.read`, `crm.contact.manage`, `crm.lead.read`, `crm.lead.manage`.

### Application — new use cases

Mirror the existing `queries/` and (future) `commands/` structure:

- `queries/`: `GetProductBySlug`, `ListPublishedProducts`, `GetOrderById`, `GetCustomerByEmail`, `ListContactsForTenant`, `ListLeadsByStatus`, etc.
- `commands/`: `CreateProduct`, `UpdateProductInventory`, `PlaceOrder`, `MarkOrderPaid`, `MarkOrderFulfilled`, `UpsertContact`, `ConvertLeadToCustomer`, etc.

Each follows the same capability-gate pattern (`GetPublishedPageBySlug`-style).

### Ports — new repositories

`ProductRepository`, `OrderRepository`, `CustomerRepository`, `ContactRepository`, `LeadRepository` — all under `crates/application/src/ports/` with the same `Send + Sync + async-trait + dyn-safety` contract and `mockall::automock` for tests.

### HTTP surface — new endpoint families

```
/api/v1/products
/api/v1/products/:slug
/api/v1/orders
/api/v1/orders/:id
/api/v1/checkout      (public, capability: commerce.checkout)
/api/v1/customers
/api/v1/contacts
/api/v1/leads
```

All under the same router pattern (resolve_tenant middleware → capability gate → use case → JSON response).

### Migrations — new tables (M2+ schedule)

```
0009_products.sql                   M2 W5
0010_product_variants.sql           M2 W5
0011_rls_products.sql               M2 W5
0012_customers.sql                  M2 W6
0013_rls_customers.sql              M2 W6
0014_orders.sql                     M3+
0015_order_lines.sql                M3+
0016_rls_orders.sql                 M3+
0017_contacts.sql                   M3+
0018_leads.sql                      M4+
0019_rls_crm.sql                    M4+
```

### Edge cache key composition — updated

Cache key composition (`CapabilitySet::cache_key_hash` consumer) gains a `domain_namespace` mixin: `cache_key = tenant + slug + caps_hash + theme_hash + namespace`. Pages and products live in separate cache namespaces.

---

## What changes in the roadmap

### M1 (2026-05-25 → 2026-06-19) — **UNCHANGED**

M1 ships foundation: xtask gates, `nas2-common`, `nas2-domain` (content + auth), application ports, tenant middleware, first endpoint. **All of this is needed for commerce + CRM anyway** — no waste.

W4 D1..D5 execute the original master-plan content (matrix tests, capability-coverage real impl, planning backfill, ammonia clean_html, RETRO).

### M2 (2026-06-22 → 2026-07-17) — **partial pivot**

Original M2: Postgres repos + RLS + JWT + 5 REST CRUD endpoints for Posts.

Revised M2:
- **W5** (06-22..06-26): migrations 0002..0008 (sites/posts + RLS + users + roles + caps + assignments) AS PLANNED — these aren't blog-specific, they're foundation.
- **W6** (06-29..07-03): `PgPostRepository` + `with_tenant` AS PLANNED. ALSO begin `PgProductRepository` + `Product` aggregate scaffolding (in parallel — same pattern).
- **W7** (07-06..07-10): JWT + login AS PLANNED (auth is universal).
- **W8** (07-13..07-17): Original plan was 5 Posts CRUD endpoints. **Revised:** ship 3 Posts CRUD (read/list/create) + 3 Products CRUD endpoints + 1 Customer endpoint. RETRO on 2026-07-17 incorporates commerce metrics into M3 bootstrap.

WP-parity tracking: M2 exit was projected at 14% WP-parity. With the pivot, parity tracking adds a second axis:
- WP-parity at M2 exit: ~10% (was 14% — we sacrificed 2 Posts CRUD endpoints to seed commerce)
- Commerce-parity at M2 exit: ~5% (Product CRUD basic)
- CRM-parity at M2 exit: ~3% (Customer basic)

The 12-month parity arc target shifts: instead of "100% WP-core by M12", target is **"50% WP-core + 40% Shopify-core + 30% HubSpot-core by M12"** — broader breadth, equivalent depth.

### M3 (2026-07-20 → 2026-08-14) — Media + Revisions + ADR-010 + Commerce continuation

Original M3: Media pipeline (libvips) + ADR-010 (block-editor) + revisions.

Revised M3: **Media pipeline serves BOTH content AND product images** (same libvips workers — Council Sentinel & Forgemaster confirm: no fork; the variant matrix `thumb / medium / large / original` works for product photos identically). Revisions apply to Products too. ADR-010 (block-editor) extends to product-description blocks.

Added scope: Order aggregate + checkout endpoint (basic, no payment provider — that's M11+).

### M4–M12 — re-planned at successive RETROs

The existing `MONTH-SKELETON-{04..12}.md` files are **historical**. Each month's RETRO regenerates the NEXT month's daily prompts per `ROADMAP_ENGINE.md §4.1`. The Council protocol absorbs the pivot via this mechanism naturally — no need to rewrite the skeletons preemptively.

`WP-PLAN-12-MONTH.html` stays as **historical record** of the pre-pivot plan; a `WP-PLAN-12-MONTH-v2.html` (or rename to `MASTER-PLAN-12-MONTH-v2.html`) lands at M2 RETRO with the commerce + CRM threads woven in.

---

## Decision drivers (the 5 §3 Priority Ladder rungs)

| Rung | Verdict |
|---|---|
| 1 Correctness | ✓ — every new aggregate gets RLS + capability gate from day 1. The CMS pattern transfers. |
| 2 Operability | ✓ — same observability stack (Tempo, Pyroscope, Prometheus); same single-binary deploy. |
| 3 Scalability | ✓ — same pool isolation contract; commerce hot paths follow the same `cache_key_hash` discipline. |
| 4 Maintainability | ⚠ — wider surface = more tests, more docs, more ADRs. Historian must track. Acceptable for the value uplift. |
| 5 Performance | ✓ — §7 targets unchanged; commerce p95 ≤ same 10-20 ms cached-read. |

---

## Council review

| Entity | Verdict |
|---|---|
| **ORCHESTRATOR** | ✓ approve. Mission expansion aligns with §0 mission of "next-generation runtime, not traditional CMS". Forward-binds: every M2..M12 monthly RETRO must re-evaluate commerce / CRM scope. |
| **HISTORIAN** | ✓ consistent — no contradictions with ratified ADR-002..ADR-006. ADR-010 (block-editor) `Consequences` extends naturally to product blocks. Decision graph gains edges: `MPD-001 forward-binds ADR-010, RFC-009 (plugin SDK), MASTER-PLAN-v2`. |
| **FORGEMASTER** | ✓ approve. Commerce hot paths (storefront read) follow the same Arc<dyn Repository>+ static-dispatch use-case shape. No new dyn surface. Existing perf targets uphold. |
| **SENTINEL** | ⚠ approve-with-conditions. Commerce surface widens attack scope (cart manipulation, price tampering, inventory race conditions). Tier-3 Adversary must engage at every commerce endpoint introduction. Failure mode register gains commerce-specific entries (order corruption, inventory negative-stock, customer PII exposure). |
| **SIMPLIFIER** | ⚠ approve-with-conditions. The domain module split (`content/`, `auth/`, `commerce/`, `crm/`) is good — but watch for over-engineering. Don't preemptively build commerce types we won't sell against in M2-M3. |
| **ECONOMIST** | ⚠ approve-with-conditions. Engineer-week budget grows: was ~20 weeks Y1, now ~25-28. Operator should confirm. Per-tenant DB storage grows ~3× (commerce + CRM tables). Production tenant cost ~ $25-40/month (was $15-25/month). |
| **MIGRATOR** | ✓ approve. Plugin SDK shape (M9 ADR-014, RFC-009) gains commerce + CRM hook types. No breaking changes to existing ADRs. |
| **ECOSYSTEM** | ✓ approve. Wider plugin marketplace (commerce plugins are a $-genuine market). M9 SDK shape MUST include commerce hooks. |
| **PRODUCTOR** | ✓ approve. Demo milestone (M7) becomes "publish a post + sell a product + see customer in CRM". Stronger demo than "publish a post" alone. |
| **JUDGE** | not invoked — no Council disagreement. All Tier-2/3/4 approvals stand. |

---

## Action items binding from MPD-001

1. **`memory/project_next_day_plan.md`** — next AVTONOM session reads this; finds the pivot directive. (Written alongside this MPD.)
2. **M2 W4 RETRO (2026-06-19)** must produce `MASTER-PLAN-12-MONTH-v2.html` reflecting MPD-001.
3. **ENTITY.md §0 amendment** — pending operator-authorized spine touch. Until then, MPD-001 + `MISSION-V2-COMMERCE-CRM.md` are the canonical mission scope.
4. **Capability enum +9 variants** — lands first commerce/CRM endpoint day (M2 W8 D1 or earlier per RETRO replan).
5. **Domain module restructure** (`content/`, `auth/`, `commerce/`, `crm/`) — lands at M2 W5 D1 if Council approves the move at that session's T2 pass.
6. **Tier-3 Adversary watch** — auto-on for every commerce + CRM endpoint going forward (per `ENTITY_SYSTEM.md §14` activation matrix, public input boundary).

---

## Roll-forward to daily prompts

Per `EXECUTION_PROTOCOL.md §15` (mid-session amendment): existing M2-M12 daily prompts in `docs/session-plans/daily/` remain valid as **content-track** prompts. The commerce + CRM prompts will be **woven in** at each monthly bootstrap.

For M1 W4 (current week, 2026-06-15 → 2026-06-19): **no daily-prompt changes**. M1 W4 closes M1 foundation work and produces the M2 bootstrap document at 2026-06-19. That bootstrap will reflect MPD-001.

---

**Ratified:** 2026-05-26 by operator via direct session instruction.
**Drift detector D-9 (decision-graph) check:** consistent — no prior ADR contradicts MPD-001.
**Forward-binds:** `MISSION-V2-COMMERCE-CRM.md`, `MASTER-PLAN-12-MONTH-v2.html` (anticipated M2 W4), ENTITY.md §0 amendment (anticipated next ENTITY revision).
