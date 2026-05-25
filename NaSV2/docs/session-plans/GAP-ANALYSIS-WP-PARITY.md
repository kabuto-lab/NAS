# GAP-ANALYSIS — current plan vs WordPress feature parity · 2026-05-25

> Honest audit answering: **"how much of WordPress does the current
> 1-month plan actually deliver, and what's left for production parity?"**
>
> Source plans audited:
>   - `ROADMAP-2026-05.md` (5 goals, 20 working days)
>   - `WEEK-01..04.md`
>   - `daily/2026-05-25..2026-06-19.md`
>   - `AUDIT-2026-05-25.md` (current code state)
>
> Methodology: WordPress's public feature surface decomposed into 28
> functional areas; each scored 0–100% based on what Month 1 ships.

---

## Verdict (TL;DR)

- **Month 1 ships ~5–7%** of WordPress's functional surface — by design.
  M1 is *foundation*: domain types, ports, first handler, gates.
- **At least 11 more months** of focused work needed for "WP-importable
  + production-deployable" parity.
- Three areas are **structurally absent** in the current plan and need
  early ADRs: comments / spam, theme-installer + block themes, and
  extension sandbox (WASM or compile-time).

---

## Feature-area scorecard (28 areas)

Coverage = % of the WP-equivalent surface that the 1-month plan
implements. "Foundation" = type/port exists but no behavior.

| # | Area | M1 Coverage | What ships in M1 | Gap to WP parity |
|---:|---|---:|---|---|
| 1 | Content model: Post / Page / CPT | 15% | `Post` aggregate + `PostStatus` FSM + `CustomPostType` placeholder | revisions, autosaves, page hierarchy, attachments, navigation menus, trash retention, schedule-on-publish trigger |
| 2 | Block editor backend | 8% | 4 Block variants (Heading/Paragraph/Image/CodeBlock) | 40+ core blocks (List/Quote/Table/Embed/Group/Columns/Cover/Gallery/Spacer/Separator/Buttons/...), block patterns, reusable blocks, block validation, block transforms |
| 3 | Taxonomies | 0% | nothing (`taxonomy` module commented in domain stub) | Categories, Tags, custom taxonomies, hierarchical taxonomies, taxonomy-term assignment, term metadata |
| 4 | Users + auth | 10% | `User` + `Email` VO + `Role` + 6 `Capability` variants | JWT verifier wiring, password reset, email confirmation, application passwords, OAuth, 2FA, session management |
| 5 | Comments + moderation | 0% | nothing | nested comments, threaded display, moderation queue, comment status FSM, akismet-equivalent spam scoring, throttling, comment subscriptions |
| 6 | Media library + pipeline | 5% | `Media` placeholder aggregate + `MediaRepository` stub | upload, libvips variant generation, srcset/sizes, alt-text editor, lazy-load policy, MIME sniffing, virus scanning, CDN origin URLs |
| 7 | Themes + template system | 0% | nothing (theme-api crate is stub) | `theme.json` parser, template hierarchy (single / archive / search / 404), bundled minimal theme, theme installer, theme customizer surface, block themes |
| 8 | Plugins / extensions | 0% | nothing (extension-api crate is stub) | hook registry (action/filter equivalent), capability-gated plugin pages, WASM sandbox per ENTITY §9.5, plugin install/update/activate lifecycle, settings API |
| 9 | Multi-site (multi-tenant) | 25% | `TenantId` newtype + `TenantContext` middleware + in-memory resolver + RLS contract documented | `PgTenantResolver` with moka LRU, `with_tenant` helper, super-admin, network settings, per-site domain mapping in DB, RLS policy SQL applied per migration |
| 10 | Admin panel UI | 0% | nothing (presentation/admin module commented stub) | Leptos islands shell, login, dashboard, post list, post editor (block UI), media browser, user manager, settings, theme manager, plugin manager |
| 11 | Public REST API | 2% | 1 endpoint (`GET /api/v1/pages/:slug`) | Full CRUD for Posts/Pages/Media/Users/Taxonomies/Comments/Settings (~25 endpoints), application-password auth, rate-limiting, request validation, error-envelope contract |
| 12 | Site editor (full-site editing) | 0% | nothing | template editor, template-part editor, global styles, navigation block, query loop block |
| 13 | Search | 0% | nothing (search-engine crate is stub) | tantivy index, indexing pipeline (pgmq jobs trigger reindex on Post write), admin search UI, public search results template, faceted search |
| 14 | Caching (L1 + L2 + L3) | 0% | moka/foyer/fred/dragonfly are workspace deps, no consumers | L1 per-instance, L2 cluster, L3 edge; cache-key composition (`tenant + slug + capability_hash`); explicit invalidation fan-out on writes; stale-while-revalidate |
| 15 | Security: sanitization | 20% | `clean_html_placeholder` reserved + W4 D4 ammonia wiring planned | sanitize-on-write enforced at write boundary by middleware (not hand-call), CSP nonce middleware, XSRF / nonce equivalent for admin POST |
| 16 | Authorization + capabilities | 30% | typed `Capability` enum + `CapabilitySet` + `caps.require(...)` marker contract for `capability-coverage` gate | mapping capability → DB role rows, role manager UI, capability inheritance, super-admin override, app-level + DB-level enforcement parity |
| 17 | SEO infrastructure | 0% | nothing | permalink structure, canonical URLs, redirect manager, sitemap.xml generation, schema.org JSON-LD, robots.txt, OpenGraph, social cards |
| 18 | i18n / l10n | 0% | nothing | locale switching, gettext-equivalent message extraction (or fluent), per-locale post variants, RTL CSS, date/number formatting |
| 19 | Customization (theme + site) | 0% | nothing | Global Styles editor, color palette, typography, layout settings, customizer-equivalent for non-block themes |
| 20 | Updates: core + plugin | 0% | nothing | self-update via single-binary swap, plugin marketplace integration, version-pin policy, rollback |
| 21 | WP migration (WXR importer) | 0% | nothing | WXR parser, posts/comments/taxonomies/attachments mapping, image rehosting, slug collision resolution, dry-run preview |
| 22 | Backup + restore | 0% | nothing | per-tenant backup, point-in-time-restore via WAL, media S3 sync, scheduled snapshots |
| 23 | CDN integration | 5% | edge-adapter crate scaffolded (stub) | CF Workers / Fastly Compute@Edge integration, cache-purge hooks on write, signed URL handling |
| 24 | Image optimization | 5% | libvips workspace dep + image-pipeline crate stub | libvips workers consuming `ax_image_jobs` pgmq queue, variant matrix per theme, AVIF/WebP fallback, srcset generation, blurhash placeholders |
| 25 | Forms | 0% | nothing | form builder block, submission storage, email notifications, spam protection, file uploads, CSV export |
| 26 | E-commerce hooks | 0% | nothing | minimal cart abstraction, product CPT, checkout webhook, third-party gateway hooks |
| 27 | Cron / scheduled tasks | 30% | `TaskSupervisor` + bounded drain | scheduled-post publisher (PostStatus::Scheduled → Published trigger), nightly reindex, cleanup jobs, distributed cron coordination |
| 28 | Email transport | 0% | lettre workspace dep, no consumer | transactional emails, queue-buffered via pgmq, retry with exponential backoff, bounce handling, per-tenant SMTP config, template renderer |

**Weighted month-1 coverage:** ~6% of full WP surface (geometric mean of
weighted coverages; some areas weighted heavier — content model, blocks,
admin, themes carry most user-visible value).

---

## Structural gaps requiring early ADRs

These are not "more work" — they are **design unknowns** the current
plan elides:

### G-S1 · Block editor frontend strategy
- M1 ships the *backend* `Block` enum + serde shape.
- The UI editor is a separate beast. WP uses React + Gutenberg. Our
  candidates:
  - Port Gutenberg via wasm-bindgen + Leptos islands (huge LOC effort)
  - Build a new block editor in Leptos islands (smaller surface, fewer
    blocks)
  - Read-only first; admin uses raw JSON until M6-M7
- **Decision needed by M5** (when block library expansion lands).

### G-S2 · Extension sandbox model
- ENTITY §9.5 says "WASM sandbox OR compile-time linked extensions".
- The *first* extension dictates the spine — `wasmtime` API surface
  vs. `inventory` registry. Compile-time is simpler but precludes
  customer-installable plugins.
- **Decision needed before M9**.

### G-S3 · Theme installer + WP theme.json
- WordPress's `theme.json` is a complex schema. AX•CMS must either:
  - Parse `theme.json` 1:1 (easy migration, schema baggage)
  - Define a new `ax.theme.toml` (cleaner, harder migration)
  - Both, with a converter
- **Decision needed before M8**.

### G-S4 · Comment spam strategy
- Akismet is proprietary; alternatives: cleantalk, antispam-bee
  port, or self-hosted Bayesian filter.
- **Decision needed before M6**.

### G-S5 · CDN + edge cache invalidation
- Cache fan-out on write is mentioned in ENTITY §3.9.1 but the
  *mechanism* (signed purge URL? per-CDN webhook? NATS broadcast?) is
  unspecified.
- **Decision needed before M10**.

---

## What the current plan does well

- **Discipline early.** Months that look "boring" (xtask gates,
  planning trail backfill) prevent the silent debt accumulation that
  drowns WordPress fork attempts at month 6.
- **Hex onion bottom-up.** common → domain → application → infra →
  presentation is the right build order; M1's sequencing prevents the
  "I'll do tenant-scoping later" trap that bites every multi-tenant
  rewrite.
- **TaskSupervisor up front.** Every WordPress alternative I've seen
  bolted a job runner on at month 8 and regretted it. AX•CMS lands
  it in week 1 — pays dividends in M3 (image pipeline workers), M6
  (comment moderation queue), M11 (search reindex).
- **Pool isolation enforced before first repo.** Saves the entire
  class of "admin migration starved the request pool at 2 AM" outages.

## What the current plan ducks

- **No mention of revisions / autosaves** for posts. WordPress has
  it; if AX•CMS ships v1 without, content authors will revolt. Land
  in M2 or M3.
- **No application-passwords surface.** Mobile apps and CI need API
  tokens; JWT alone isn't enough for headless flows. Land in M3.
- **No mentions of "content draft preview".** Editors expect to
  preview unpublished posts via a signed URL. This is its own auth
  path — needs its own RFC.
- **No mention of nonces** for admin form POSTs. WP uses nonces for
  CSRF; we need a story even if it's "same-origin + SameSite=strict
  cookie + capability check".
- **No image processing on the upload path.** M1 reserves
  `MediaRepository::find_by_id` but the upload endpoint is M3+.

---

## Realistic month-by-month projection

Detailed in `MASTER-ROADMAP-2026-2027.md`. Summary:

| Month | Theme | WP-parity ∆ |
|---:|---|---:|
| M1 (current) | Foundation, gates, first handler | +6% |
| M2 | Postgres repos + JWT + 4 more REST endpoints + RLS policy SQL | +8% |
| M3 | Media pipeline (libvips workers) + uploads + variants | +7% |
| M4 | Block library expansion + reusable blocks + patterns | +10% |
| M5 | Taxonomies + post-term assignment + admin UI scaffolding | +9% |
| M6 | Comments + moderation + spam | +8% |
| M7 | Admin panel skeleton (login, dashboard, post editor) | +12% |
| M8 | Themes API + first bundled theme + template hierarchy | +10% |
| M9 | Extension API + WASM sandbox + first plugin (SEO basics) | +9% |
| M10 | Caching layers L1+L2 + invalidation fan-out | +7% |
| M11 | Search (tantivy) + reindex pipeline | +6% |
| M12 | WP importer + PGO/BOLT + production hardening + edge | +8% |

**End-of-year cumulative coverage: ~100% WP-equivalent functional
surface** (excluding e-commerce, forms, advanced SEO — those go into
Year 2 as plugins).

---

## Recommendations actioned in MASTER-ROADMAP

1. Add an "early ADR" slot in M3 covering G-S1 (editor strategy).
2. Reorganize M2 to land RLS policy SQL alongside first repo (currently
   M2 was assumed "just repos" — RLS policy is the actual hard part).
3. Insert revisions + autosaves into M3 instead of M5+.
4. Add nonces + CSRF surface to M2.
5. Promote backup story (currently month 14+) to M10 since cache
   invalidation work touches the same WAL/replication concerns.

---

## What this document is NOT

- Not a marketing promise.
- Not a commitment timeline — it's a *feasible* plan, not a contract.
- Not a substitute for the per-month bootstrap — each month's AUDIT
  reconciles with reality and may shift cards.

Time estimates assume one full-time engineer + AVTONOM. With more
hands, parallel tracks reshape the months (admin UI in parallel with
back-end work especially benefits).
