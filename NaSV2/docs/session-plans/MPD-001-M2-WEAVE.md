# MPD-001 · M2 WEAVE OVERLAY — Commerce + CRM threads (additive)

> **Status:** Ratified weave plan — operator-approved indirectly via
> `memory/project_next_day_plan.md §3 M2 forecast` and direct re-trigger
> of `avtonom-month-bootstrap-2026-07.md` on 2026-05-26 (second AVTONOM
> call of the day).
> **Source mission:** `docs/governance/master-plan-diffs/MPD-001-commerce-crm-pivot.md`
> ratified 2026-05-26.
> **Source plan:** `ROADMAP-2026-06.md` + `WEEK-{05..08}.md` + `daily/2026-{06-22..07-17}.md`
> authored 2026-05-25 (one day pre-MPD).
> **Type:** Additive overlay — does NOT modify content-track P1..PN of
> any daily; ADDs `## MPD-001 WEAVE ADD-ON` sections with bounded
> time budgets per day. Each ADD-ON is OPTIONAL and may be deferred to
> next-day `CARRY-OVER` if the core P1..PN runs over.
>
> **Why this file exists separately:** content-track dailies were already
> reviewed and committed by `d42a55a` ("next-month bootstrap (2026-07)
> generated"). Re-writing them would destroy reviewed content. The
> overlay model preserves that review work while honouring MPD-001 PF1.0
> instruction in the bootstrap prompt.

---

## 1 · Overlay summary

The M2 content track (G1 RLS → G2 PgPostRepository → G3 JWT → G4 Posts
CRUD → G5 CSRF + revisions) is UNCHANGED. On top of it, four bounded
ADD-ON slots land commerce + CRM as **pure-domain seeds** + design
docs — no endpoints, no repo impls, no migrations.

| Day | Slot | Budget | Output | Cumulative |
|---|---|---:|---|---|
| 2026-06-29 (W6 D1) | post-P1 | 30 min | Domain module restructure (`content/`, `auth/`, `commerce/`, `crm/`, `shared/` subdirs) | 0.5 h |
| 2026-07-14 (W8 D2) | post-P1 | 1.5 h | `commerce::product::Product` aggregate seed + 3 unit tests | 2.0 h |
| 2026-07-15 (W8 D3) | post-P1 | 1.0 h | `crm::customer::Customer` aggregate seed + 2 unit tests | 3.0 h |
| 2026-07-16 (W8 D4) | post-P1 | 1.0 h | RFC-007 commerce-foundation + RFC-008 crm-foundation + capability enum +4 variants | 4.0 h |
| 2026-07-17 (W8 D5) | RETRO §x | 30 min | RETRO-2026-07 includes "commerce + CRM thread progress" subsection + M3 W1 forecast | 4.5 h |

**Total ADD-ON budget:** 4.5 h on top of M2's 160 h capacity (~2.8 %).
Within the 5 % docs allocation per `AUDIT-2026-06-22.md §A12`.

**Carry-over discipline:** if a content-track day overruns its 4 h,
the ADD-ON is the FIRST thing deferred (not the inverse). The content
track remains the master plan.

---

## 2 · Domain module restructure (W6 D1)

```
crates/domain/src/
├── lib.rs                # re-exports ALL existing public types from
│                         # their new locations (zero call-site breakage)
├── content/
│   ├── mod.rs
│   ├── site.rs           # moved
│   ├── post.rs           # moved
│   ├── block.rs          # moved (also used by commerce::product!)
│   └── media.rs          # moved
├── auth/
│   ├── mod.rs
│   ├── capability.rs     # moved
│   ├── role.rs           # moved
│   └── user.rs           # moved
├── commerce/
│   └── mod.rs            # NEW · empty marker
├── crm/
│   └── mod.rs            # NEW · empty marker
└── shared/
    ├── mod.rs            # NEW
    └── slug.rs           # moved validate_slug here (cross-cutting)
```

**Rules:**
- `cargo check --workspace` MUST remain green after restructure.
- `lib.rs` re-exports keep ALL external call-sites compiling
  unchanged: `pub use content::post::*; pub use auth::role::*; …`
- No code logic changes. Pure mechanical move + re-export.
- 30-minute budget. If hits 45 min → REVERT (`git checkout -- .` on the
  affected files), log, defer to D2 CARRY-OVER.

**Single mini-commit:**
`refactor(ax/m2-w6d1,MPD-001): domain module restructure into content/auth/commerce/crm/shared`

---

## 3 · Aggregate seeds (W8 D2 + D3)

### 3.1 · `commerce::product::Product` (W8 D2, 2026-07-14)

```rust
// crates/domain/src/commerce/product.rs
pub struct Product {
    id:          ProductId,           // newtype in nas2-common
    tenant_id:   TenantId,
    site_id:     SiteId,
    slug:        ProductSlug,         // newtype in nas2-common
    title:       String,
    description: Vec<Block>,          // REUSE Block from content/
    price_cents: i64,                 // bigint; non-negative invariant
    currency:    Currency,            // enum: ISO 4217 subset
    sku:         Option<Sku>,         // newtype in nas2-common
    status:      ProductStatus,       // Draft/Active/Archived FSM
}
```

**Invariants enforced in `Product::new(...)`:**
- `price_cents >= 0` (no negative prices; refunds are Order-level)
- `description` length `>= 1` (must have at least one Block)
- `slug` validated via `shared::slug::validate_slug` (already covers
  ASCII + length 1..200)
- `currency` defaults to `Currency::Usd` if input invalid →
  `AppError::Validation`

**FSM `transition_to(new: ProductStatus)`:**
- `Draft → Active` (publish)
- `Active → Archived` (soft-delete)
- `Draft → Archived` (discard)
- All other transitions → `AppError::Conflict` (parallel to
  `Post::transition_to`)

**Tests (3 minimum, in module):**
- `test_new_validates_slug`
- `test_new_validates_price_non_negative`
- `test_transition_to_active_from_archived_returns_conflict`

**No repo. No migration. No endpoint.** All M3+.

**Single mini-commit:**
`feat(ax/m2-w8d2,MPD-001): commerce::Product aggregate seed`

### 3.2 · `crm::customer::Customer` (W8 D3, 2026-07-15)

```rust
// crates/domain/src/crm/customer.rs
pub struct Customer {
    id:               CustomerId,        // newtype in nas2-common
    tenant_id:        TenantId,
    email:            Email,             // REUSE VO from auth/
    display_name:     String,
    lifecycle_status: LifecycleStatus,   // Lead/Customer/Churned FSM
    created_at:       DateTime<Utc>,
}
```

**Invariants enforced in `Customer::new(...)`:**
- `email` is a validated `Email` VO (RFC 5322 subset, already shipped
  in `auth/user.rs`)
- `display_name` length `>= 1` (non-empty after trim)
- `lifecycle_status` defaults to `LifecycleStatus::Lead`

**FSM `transition_to(new: LifecycleStatus)`:**
- `Lead → Customer` (conversion)
- `Customer → Churned` (lifecycle end)
- `Lead → Churned` (dropped before conversion)
- No backward transitions in M2 (revisit if business demands)

**Tests (2 minimum, in module):**
- `test_new_validates_display_name_non_empty`
- `test_lifecycle_default_is_lead`

**No repo. No migration. No endpoint.** All M3+.

**Single mini-commit:**
`feat(ax/m2-w8d3,MPD-001): crm::Customer aggregate seed`

---

## 4 · Foundation RFCs (W8 D4)

### 4.1 · `docs/rfc/RFC-007-commerce-foundation.md`

~80 LOC. Standard RFC structure: Status, Context, Decision, TLA
layers, Consequences, References.

**Covers:**
- Motivation: MPD-001 §"Motivation"; Shopify/HubSpot context.
- Aggregates seeded (Product, M2) + planned (ProductVariant, Order,
  OrderLine — M3+).
- Capability namespace: `commerce.product.{read,manage}`,
  `commerce.order.{read,manage}`, `commerce.checkout`.
- Postgres tables — schema sketch only (NOT migration):
  `products(id, tenant_id, site_id, slug, title, description JSONB,
  price_cents BIGINT, currency TEXT, sku TEXT NULL, status TEXT,
  created_at, updated_at)`;
  `product_variants(id, product_id, sku, price_delta_cents, attrs JSONB)`;
  `orders(id, tenant_id, customer_id, status, currency, total_cents,
  ...)`; `order_lines(id, order_id, variant_id, qty, unit_price_cents,
  snapshot JSONB)`.
- Non-goals M2: no Order, no checkout, no payments adapter.
- Forward link: M3 W1 D1..D3 expects ProductRepository + 3 endpoints
  (GET/LIST/POST). M3 W2 expects Order aggregate.

### 4.2 · `docs/rfc/RFC-008-crm-foundation.md`

~80 LOC. Same structure.

**Covers:**
- Motivation: MPD-001 §"Motivation"; HubSpot context.
- Aggregates seeded (Customer, M2) + planned (Contact, Lead — M3/M4).
- Capability namespace: `crm.contact.{read,manage}`,
  `crm.lead.{read,manage}`.
- Postgres tables — schema sketch only:
  `customers(id, tenant_id, email, display_name, lifecycle_status,
  created_at)`; `contacts(id, tenant_id, customer_id NULL, kind,
  email, source, ...)`; `leads(id, contact_id, status, source, ...)`.
- Non-goals M2: no Contact, no Lead, no marketing automation.
- Forward link: M3 W3 D1..D2 expects CustomerRepository + first
  upsert endpoint. M4 W1 expects Contact + Lead aggregates.

### 4.3 · Capability enum extension

In `crates/domain/src/auth/capability.rs` (post-restructure path),
add 4 variants:

```rust
pub enum Capability {
    // ... existing variants ...
    CommerceProductRead,
    CommerceProductManage,
    CrmContactRead,
    CrmContactManage,
}
```

Extend `as_str()`:
- `commerce.product.read`
- `commerce.product.manage`
- `crm.contact.read`
- `crm.contact.manage`

4 unit tests (one per variant). No handler uses these yet —
pure enum surface for M3+ binding.

**Single mini-commit (covers RFCs + enum):**
`docs(ax/m2-w8d4,MPD-001,RFC-007,RFC-008): commerce + CRM foundation RFCs + capability enum extension`

---

## 5 · Exit criteria addendum

In addition to `ROADMAP-2026-06.md §B6` items 1..14:

15. `crates/domain/src/{content,auth,commerce,crm,shared}/` subdirs
    exist; ALL existing aggregates re-located; `lib.rs` re-exports
    preserve ALL call-sites; `cargo check --workspace --all-targets`
    green; `cargo test --workspace --lib --no-fail-fast` green.
16. `commerce::product::Product` shipped with ≥ 3 unit tests passing.
17. `crm::customer::Customer` shipped with ≥ 2 unit tests passing.
18. RFC-007 + RFC-008 committed; `xtask check-planning-refs` green.
19. Capability enum extends by 4 variants (CommerceProductRead,
    CommerceProductManage, CrmContactRead, CrmContactManage) with
    unit tests covering `as_str()` strings.

---

## 6 · Anti-goals (commerce + CRM specific, M2)

- ❌ No `Order` aggregate (M3 W2+)
- ❌ No `Lead` aggregate (M4+)
- ❌ No `Contact` aggregate (M3 W3+)
- ❌ No `ProductVariant` aggregate (M3 W1+)
- ❌ No commerce migrations (M3 W1+)
- ❌ No commerce endpoints (M3 W1+)
- ❌ No checkout flow / payments (M3 W4+)
- ❌ No new Cargo.toml deps (current domain crate deps suffice for
  in-memory aggregates; `chrono::DateTime<Utc>` already a workspace dep)

---

## 7 · Capacity impact

| Bucket | M2 budget | Pre-weave | With weave | Delta |
|---|---:|---:|---:|---:|
| P0 verification | 48 h | 48 h | 48 h | 0 |
| Feature work | 80 h | 80 h | 80 h | 0 |
| Refactor / debt | 24 h | 24 h | 24 h | 0 |
| Docs | 8 h | 8 h | **12.5 h** | +4.5 h |
| Slack | 5 d | 5 d | 5 d (no change) | 0 |
| **Total** | 160 h | 160 h | 164.5 h | +2.8 % |

The +4.5 h fits inside per-day slack and ADD-ONs are deferrable.
No goal slips planned.

---

## 8 · Forward link to M3

The next-month bootstrap (`avtonom-month-bootstrap-2026-08.md`,
generated by RETRO-2026-07 in W8 D5) should:

1. Convert seed `Product` → `PgProductRepository` (M3 W1 D1)
2. Migration `0009_products.sql` + `0010_rls_products.sql` (M3 W1 D2..D3)
3. Endpoints `GET /api/v1/products` (list), `GET /api/v1/products/:slug`,
   `POST /api/v1/products` (M3 W1 D4..D5 + W2 D1)
4. `PgCustomerRepository` + `POST /api/v1/customers` upsert (M3 W3)
5. `ProductVariant` + `Order` aggregates (M3 W2..W4)

The seeds and RFCs from M2 give M3 the design + tests + schema sketch
to execute deterministically. No further design churn expected for
the M3 commerce/CRM thread.

---

## 9 · References

- `docs/governance/MISSION-V2-COMMERCE-CRM.md` — mission rationale
- `docs/governance/master-plan-diffs/MPD-001-commerce-crm-pivot.md` — operational diff
- `memory/project_next_day_plan.md §3 M2 forecast` — pre-ratified plan
- `docs/session-plans/ROADMAP-2026-06.md` — content-track master
- `docs/session-plans/RETRO-2026-06.md §7` — M2 seed goals
- `docs/session-plans/AUDIT-2026-06-22.md §A12` — capacity model
- `ENTITY.md §3` — stack (unchanged by MPD-001)
- `docs/governance/CONSTITUTION.md §4` — 14 Immutables (unchanged)

---

**End of weave overlay.** Each affected daily prompt
(`daily/2026-06-29.md`, `2026-07-14.md`, `2026-07-15.md`,
`2026-07-16.md`) contains a `## MPD-001 WEAVE ADD-ON` section
pointing back at this document for design rationale.
