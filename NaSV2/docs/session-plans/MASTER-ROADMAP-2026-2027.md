# MASTER ROADMAP — AX•CMS WordPress-on-Rust · 12 months · 2026-05 → 2027-04

> Source of truth for the multi-month arc. Derived from
> `GAP-ANALYSIS-WP-PARITY.md`. Each month's bootstrap (`avtonom-
> month-bootstrap-YYYY-MM.md`) seeds its AUDIT/ROADMAP from the
> matching month entry below.
>
> **Living document.** Each month's RETRO may shift cards in the
> immediately upcoming month; later months are seeds, not contracts.

---

## Phase view

| Phase | Months | Theme | Headline deliverable |
|---:|---|---|---|
| **P1 — Foundation** | M1–M2 | Domain, ports, repos, auth, RLS | First production-shaped read+write path with multi-tenant isolation |
| **P2 — Content core** | M3–M5 | Media pipeline, block library, taxonomies | Editor backend can express full WP content shape |
| **P3 — Admin + themes** | M6–M8 | Comments, admin panel, themes API | First end-to-end admin workflow: author logs in → writes post → applies theme → publishes |
| **P4 — Extensibility + perf** | M9–M11 | Extension API, caching, search | Customizable + cache-warm at p95 ≤ 50 ms cached read |
| **P5 — Migration + production** | M12 | WP importer + PGO/BOLT + edge | First customer site migrated and served from production deployment |

---

## Calendar (assuming 5 working days/week, no Q3 vacation)

| Month | Window | Working days |
|---:|---|---:|
| M1 | 2026-05-25 → 2026-06-19 | 20 |
| M2 | 2026-06-22 → 2026-07-17 | 20 |
| M3 | 2026-07-20 → 2026-08-14 | 20 |
| M4 | 2026-08-17 → 2026-09-11 | 20 |
| M5 | 2026-09-14 → 2026-10-09 | 20 |
| M6 | 2026-10-12 → 2026-11-06 | 20 |
| M7 | 2026-11-09 → 2026-12-04 | 20 |
| M8 | 2026-12-07 → 2026-12-31 | ~18 (year-end) |
| M9 | 2027-01-04 → 2027-01-29 | 20 |
| M10 | 2027-02-01 → 2027-02-26 | 20 |
| M11 | 2027-03-01 → 2027-03-26 | 20 |
| M12 | 2027-03-29 → 2027-04-23 | 20 |

Total: ~238 working days = ~12 calendar months.

---

## M1 · 2026-05-25 → 2026-06-19 · **Foundation: gates + domain core + first handler**
*(In progress — see `ROADMAP-2026-05.md` + `daily/2026-05-25..06-19.md`)*

- G1 xtask hygiene gates (magic-check + check-planning-refs real)
- G2 `nas2-common` + `nas2-domain` bootstrap
- G3 `application::ports` + tenant middleware
- G4 First `GET /api/v1/pages/:slug` handler + capability gate
- G5 `capability-coverage` real + planning backfill + first perf
  baseline + ammonia sanitize-on-write

**Exit**: 1 read endpoint working end-to-end (mock repo); xtask gates
real; ammonia wired; planning trail orphans ≤ 2.

---

## M2 · 2026-06-22 → 2026-07-17 · **Postgres repos + JWT + RLS policy SQL + CRUD endpoints**
*(See `ROADMAP-2026-06.md` + `daily/2026-06-22..07-17.md`)*

- **G1** First applied migration with **RLS policy SQL** for tenant-
  scoped tables (sites, posts, users, roles, role_assignments).
  Includes the `app.tenant_id` session-var contract.
- **G2** `PgPostRepository` impl against sqlx + `set_config` single-RTT
  pattern per ENTITY §3.5 + integration tests via testcontainers.
- **G3** `JwtVerifier` + `extract_caps_from_jwt` middleware; replaces
  the W4 D1 thread-local stub. Bearer token + cookie-jar paths.
- **G4** REST CRUD for Posts: `GET /pages/:slug` (real DB now),
  `GET /pages` list (paginated), `POST /pages` (admin), `PATCH
  /pages/:slug` (admin), `DELETE /pages/:slug` (admin); each with
  capability-coverage marker and oneshot tests.
- **G5** CSRF/nonce surface for admin POST/PATCH/DELETE; structural
  ADR for revisions/autosaves (deferred impl, ADR only).

**Exit**: real PostgreSQL + RLS-enforced multi-tenant CRUD for posts
visible at `https://<tenant-host>/api/v1/pages/...`; cumulative WP
parity ~14%.

---

## M3 · 2026-07-20 → 2026-08-14 · **Media pipeline + revisions + editor strategy ADR**

- **G1** ADR-010 **block editor frontend strategy** (G-S1 from gap
  analysis). Decision drives M6+M7 admin work.
- **G2** Media model: `Media` real aggregate (mime, checksum, sizes
  manifest, alt) + `MediaRepository` real methods (upload, find,
  derive_variant, delete).
- **G3** libvips workers: pgmq consumer in `nas2-image-pipeline`,
  TaskCategory::Image supervisor wired in `apps/server`; variants
  defined per theme manifest (placeholder until M8).
- **G4** Upload endpoint `POST /api/v1/media` (multipart, capability-
  gated, MIME sniffing via `infer` crate, virus-scan hook stub).
- **G5** Post revisions: `revisions` table + `RevisionRepository`,
  autosave every 30s contract (server-driven), `GET /pages/:slug/revisions`,
  revert endpoint.

**Exit**: image upload → variant generation → CDN URL works end-to-
end; revisions browsable; ADR-010 picked. ~21% parity.

---

## M4 · 2026-08-17 → 2026-09-11 · **Block library expansion + patterns + reusable blocks**

- **G1** 15 more block variants (List, Quote, Table, Code, Cover,
  Group, Columns, Spacer, Separator, Buttons, Gallery, Embed,
  Navigation, Search, Latest-posts). Each: serde shape + 1 test +
  RFC subsection in RFC-001 update.
- **G2** Block patterns (`Pattern` aggregate + repository) — reusable
  block compositions.
- **G3** Reusable blocks (`ReusableBlock` aggregate with ref-counting
  + cache invalidation on update).
- **G4** Block validation pass: per-block invariant checks invoked
  by `PostRepository::insert/update`. Reject invalid blocks at the
  write boundary, not at render.
- **G5** OpenAPI surface (utoipa) for all M2–M4 endpoints; first
  Swagger UI mount at `/api/docs`.

**Exit**: editor backend expresses 90%+ of WP block shapes; OpenAPI
generated. ~31% parity.

---

## M5 · 2026-09-14 → 2026-10-09 · **Taxonomies + term assignment + admin scaffolding**

- **G1** Taxonomy model: `Taxonomy` + `Term` + hierarchical parent_id
  + `TermRepository` (find_by_slug, list_for_tenant, descendants,
  ancestors).
- **G2** `post_terms` join: `PostRepository::assign_terms` / `terms_for_post`;
  invariant: term must belong to same tenant.
- **G3** Custom taxonomies: tenant-defined taxonomies via admin
  (registry table + cache); WP-compat taxonomy slugs `category`, `tag`
  pre-seeded per tenant.
- **G4** `GET /api/v1/taxonomies/:tax/terms/:slug/posts` — list posts
  in term; pagination + capability gate.
- **G5** **Admin scaffolding kickoff**: Leptos islands skeleton in
  `crates/presentation/src/admin/`; login form + dashboard placeholder;
  CSRF via SameSite-strict cookie; no real content management yet
  (M7 lands that).

**Exit**: taxonomies queryable; admin login screen shippable. ~40% parity.

---

## M6 · 2026-10-12 → 2026-11-06 · **Comments + moderation + spam scaffolding**

- **G1** Comment model: nested via materialized-path or adjacency
  list (ADR-011); `Comment` aggregate; `CommentStatus` FSM (Pending,
  Approved, Spam, Trash).
- **G2** `CommentRepository` impl + integration tests; tenant-scoped;
  cascade rules on Post delete (soft → keep comments archived).
- **G3** Moderation queue endpoints: `GET /api/v1/comments?status=pending`
  (admin), `PATCH /api/v1/comments/:id/status` (Pending → Approved /
  Spam / Trash); capability `comments.moderate`.
- **G4** Spam scoring scaffold (G-S4): pluggable trait
  `SpamScorer` + default `HeuristicSpamScorer` (URL count, length,
  honeypot field, throttling); ADR-012 picks final approach.
- **G5** Public submit endpoint `POST /api/v1/posts/:slug/comments`
  with rate-limiting (tower-governor) + `SpamScorer::score` integration.

**Exit**: comments end-to-end with naive spam filter; moderation
admin paths present. ~48% parity.

---

## M7 · 2026-11-09 → 2026-12-04 · **Admin panel: post list + post editor (block UI MVP)**

- **G1** Admin shell: navigation, breadcrumbs, capability-aware
  menu rendering; per-route guards via `caps.require()`.
- **G2** Post list view: filterable by status/author/term, paginated;
  Leptos table island.
- **G3** Block editor MVP (front-end half of G-S1): renders existing
  Block variants in an editable canvas; create/insert/reorder/delete
  blocks; saves draft every 30s (uses M3 autosave).
- **G4** Media browser island: grid view, upload from editor, alt-text
  editor.
- **G5** User & role manager: list users, assign roles, capability
  matrix view.

**Exit**: an editor can log in, browse posts, write+publish a post
with images, end-to-end. ~60% parity. **This is the first
demo-worthy milestone.**

---

## M8 · 2026-12-07 → 2026-12-31 · **Themes API + first bundled theme + template hierarchy**

- **G1** ADR-013 **`ax.theme.toml` vs `theme.json` compat** (G-S3);
  pick path.
- **G2** Theme installer: `themes/<name>/` workspace member, `theme.toml`
  manifest, image-variant matrix declaration, template list.
- **G3** Template hierarchy resolver: given a URL, pick the template
  chain (single → archive → index); cache resolved chain.
- **G4** Bundled `themes/minimal/` — a 3-template no-CSS-framework
  theme that proves the API.
- **G5** Theme manager UI in admin: install / activate / preview.

**Exit**: public traffic renders via a real theme. ~70% parity.

---

## M9 · 2027-01-04 → 2027-01-29 · **Extension API + WASM sandbox + first plugin (SEO basics)**

- **G1** ADR-014 **WASM sandbox vs compile-time linked** (G-S2).
  Default: wasmtime + WASI snapshot 2 + capability tokens.
- **G2** Hook registry: typed action/filter equivalent; deterministic
  ordering; capability-aware invocation.
- **G3** Plugin lifecycle: install → activate → deactivate →
  uninstall; sandbox per plugin; settings API.
- **G4** First plugin `extensions/seo-basics/` — meta tags, OG tags,
  sitemap.xml, robots.txt, canonical URL.
- **G5** Plugin manager UI in admin.

**Exit**: a customer can install a WASM plugin and have it modify
output. ~79% parity.

---

## M10 · 2027-02-01 → 2027-02-26 · **Caching L1+L2 + invalidation fan-out + backup story**

- **G1** moka L1 wiring: per-instance page cache keyed by
  `tenant + slug + capability_hash + theme_hash`; TTL + size budgets.
- **G2** Dragonfly L2 wiring (via fred): cross-instance cache layer;
  fail-open on Dragonfly unavailability.
- **G3** Invalidation fan-out: on Post/Term/Media/Theme write, emit
  cache-purge event via NATS JetStream (per ENTITY §3.8); subscribers
  drop matching keys.
- **G4** ADR-015 **edge cache invalidation mechanism** (G-S5).
- **G5** Backup story: `nas2-cli backup --tenant <id>` exports
  per-tenant SQL dump + media S3 sync; restore path; documented.

**Exit**: cached read p95 within §7 budget; backup tested. ~86% parity.

---

## M11 · 2027-03-01 → 2027-03-26 · **Search (tantivy) + reindex pipeline**

- **G1** tantivy index per tenant (sharded directory); index schema
  for Post (title + body text + taxonomies + author).
- **G2** Reindex pipeline: pgmq queue `ax_search_reindex`; worker
  consumes on Post/Term writes; full reindex bootstrap via `nas2-cli`.
- **G3** Public search endpoint `GET /api/v1/search?q=...` —
  capability-gated to `cms.page.read`; supports pagination + facets.
- **G4** Admin search bar — live search across posts/comments/media.
- **G5** Performance bench: search p95 under 50 ms at 10 K posts;
  documented in `docs/perf/search-baseline.json`.

**Exit**: search works; reindex catches up within 5 s of write. ~92% parity.

---

## M12 · 2027-03-29 → 2027-04-23 · **WP importer + PGO/BOLT + edge + production deploy**

- **G1** WXR parser (`nas2-cli import wxr <file>`): posts + comments
  + taxonomies + attachment URLs; dry-run mode; slug-collision
  resolver.
- **G2** Image rehoster: download `<wp:attachment_url>` to media
  store; URL rewriter in block bodies.
- **G3** PGO + BOLT pipeline real impls (`xtask pgo-build` / `bolt-
  optimize`); nightly bench compares baseline.
- **G4** Edge integration (Cloudflare Workers): static delivery,
  auth hint cookie → tenant routing, image variant negotiation by
  Accept header.
- **G5** Production deploy procedure: Fly.io Machines manifest;
  staged rollout; `/health/*` checks; on-call runbook.

**Exit**: first real customer WP site imported and served via
production deploy. ~100% WP-equivalent core parity.

---

## Cross-month invariants (every month, every week)

1. **P0 verification gate** runs first thing every day. Any red ends
   the session before P1 starts.
2. **Spine files unchanged** unless explicitly authorized per day
   (rare; M1 had three one-line spine edits in xtask/main.rs).
3. **No `git push` from AVTONOM** — operator pushes.
4. **`xtask architecture-check / magic-check / capability-coverage /
   check-planning-refs`** must stay green at month-end.
5. **Per-month RETRO** in W4 D5 generates next-month bootstrap.
6. **No new workspace `Cargo.toml` deps** without an ADR.

---

## Year-2 candidates (not in this roadmap)

- E-commerce: Cart + product CPT + Stripe/Lemon webhook + tax engine.
- Forms: block-based form builder + submission storage + email export.
- Advanced SEO: schema.org rich results, structured data validators,
  Open Graph image generators.
- Migration FROM AX•CMS to other systems (export to Markdown / Hugo /
  WP).
- Static export mode (serve as pre-rendered HTML + edge).
- Real-time collaboration in the block editor (CRDT — yrs).
- Internationalization (i18n) full implementation.
- Plugin marketplace + signing + curation.

---

## Risk register (12-month, top 7)

| # | Risk | Trigger | Mitigation |
|---:|---|---|---|
| 1 | Block editor frontend turns into half a year of UX work | M7 G3 | early ADR M3; ship JSON-edit-textarea fallback if Leptos editor slips |
| 2 | WP `theme.json` compat unbounded | M8 | ADR-013 picks compat scope ahead of impl; explicit "non-goals" list |
| 3 | WASM sandbox perf > Tokio overhead | M9 | bench wasmtime cold start; if > 5 ms, switch first plugin to compile-time linkage |
| 4 | Dragonfly + NATS operational complexity hits month 10 | M10 | start with moka L1 only; L2/NATS behind feature flag until customer pain |
| 5 | tantivy index disk usage explodes per tenant | M11 | per-tenant size cap + sharded directories; benchmark with 10 K-post fixtures |
| 6 | WXR parser hits a WP edge case we don't model | M12 | strict mode + lossy mode; lossy logs to a per-import report |
| 7 | Production deploy reveals an §3.4 pool drift in prod pgbouncer | M12 | `xtask pool-mode-check` already exists; CI runs against staging weekly |

---

## How to use this document

- **Each month's bootstrap** (`avtonom-month-bootstrap-YYYY-MM.md`)
  reads the matching month section as input — its AUDIT compares
  reality to the predicted exit-state; its ROADMAP adapts G1..G5 to
  any drift.
- **If reality diverges** (scope creep, blocker, capacity change):
  next month's RETRO §7 (Next-month proposals) may shift later months.
  Edit this document — but commit the diff with a note explaining
  the shift.
- **Mid-month deviations** stay in `SESSION_LOG.md` `Recommendations`
  → roll up into the RETRO that ends the month → bubble into
  MASTER updates if structural.
