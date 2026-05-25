# M7 — Vision · Architecture · Decomposition

> **Month 7 · 2026-11-09 → 2026-12-04 (20 working days)**
> Theme: Admin panel + post list + post editor (block UI MVP) +
> media browser + user/role manager.
> **🏆 First demo-worthy milestone of the year.**

---

## §1 · Vision — Where M7 sits in the final RustPress (April 2027)

By end of M12, RustPress is a single-binary, multi-tenant, Rust-
based CMS that ships:

- **Public read path** — Leptos SSR templates serving cached pages
  with p95 ≤ 20 ms (cached hit), routed through Cloudflare Workers
  at the edge for static + auth-hint resolution.
- **Admin panel** — Leptos islands-based SPA shell, server-rendered
  first paint, hydration only on interactive surfaces (PostList,
  BlockEditor, MediaBrowser, UserManager, PluginManager,
  ThemeManager).
- **REST API surface** — ~30+ endpoints, OpenAPI documented,
  capability-gated, JWT-authenticated, CSRF-protected.
- **Plugin ecosystem** — WASM sandbox (wasmtime + WASI Snapshot 2),
  capability-token-gated host imports, signed `.tar.zst` archives,
  marketplace-ready.
- **Theme ecosystem** — `ax.theme.toml` manifest, template hierarchy
  resolver, bundled `themes/minimal/`, customer-installable signed
  archives.
- **WP-importer** — WXR parser + image rehoster + slug-collision
  resolver; one-shot migration from WordPress.
- **Observability** — tracing/OTLP/Tempo + Prometheus + Pyroscope
  continuous profiling.

**M7's contribution to that final picture:** the *first complete
admin workflow* — an editor logs in, browses posts, edits a post in
the block editor, uploads media, publishes. This is the user-facing
proof that everything from M1-M6 (domain, repos, auth, REST, RLS,
media pipeline, block library, comments) hangs together.

After M7, every subsequent month is *additive polish + ecosystem*:
themes (M8), plugins (M9), caching (M10), search (M11), production
hardening + WP-importer (M12). M7 is the **demo we'd show a
prospective customer or investor.**

---

## §2 · Architectural Layering — full project map (with M7 deltas)

```
┌─────────────────────────────────────────────────────────────────────┐
│                         RustPress Workspace                         │
└─────────────────────────────────────────────────────────────────────┘
                          │
   ┌──────────────────────┼──────────────────────┐
   ▼                      ▼                      ▼
[PUBLIC]              [ADMIN UI]               [WORKERS]
  Leptos SSR            Leptos islands          libvips, pgmq,
  + edge cache          (M7 builds here)        search-index,
                                                cache-purge subs

L1 · presentation  (crates/presentation)
  │ ── api/                       (REST handlers — M2-M6 shipped ~25 eps)
  │ ── admin/                     (Leptos islands — M5 scaffold; M7 fills)
  │     │── shell/                (route guards, nav, breadcrumbs, layout) ← M7 W1
  │     │── post_list/            (paginated table, filter, bulk)          ← M7 W2
  │     │── post_editor/          (block canvas + per-variant edits)        ← M7 W3
  │     │── media_browser/        (grid + upload + alt-text)               ← M7 W4 D1-D3
  │     │── user_manager/         (role + capability matrix)               ← M7 W4 D4
  │     └── components/           (Button, Modal, FormField — reused)
  │ ── ssr/                       (public template renderers — M8)
  │ ── auth/                      (JWT middleware — M2)
  │ ── csrf.rs                    (double-submit — M2 W8 D3)
  │ ── caps.rs                    (CapabilitySet extractor — M2 W7 D2)
  │ ── router.rs                  (top-level Router builder)
  │ ── app_state.rs               (AppState — extended per month)

L2 · application  (crates/application)
  │ ── ports/                     (Post/User/Site/Media/Comment/Term repos)
  │ ── queries/                   (read-side use cases)
  │ ── commands/                  (write-side use cases — Create/Update/Login)
  │ ── auth/                      (JwtVerifier, JwtIssuer, password)
  │ ── hooks/                     (M9 — extension registry)
  │ ── services/                  (cache, search, mail — composed adapters)

L3 · domain  (crates/domain)
  │ ── post.rs                    (Post + PostStatus FSM + PostSlug)
  │ ── block.rs                   (19 variants by end of M4)
  │ ── site, user, role, capability, email, taxonomy, term, comment, media
  │ ── pattern, reusable_block    (M4)
  │ ── post_revision              (M3)

L4 · infrastructure  (crates/infrastructure)
  │ ── persistence/               (Pg{Post,User,Site,Media,Comment,Term,...}Repository)
  │ ── queue/                     (PgmqQueue — M1)
  │ ── auth/                      (HmacJwtVerifier + HmacJwtIssuer)
  │ ── storage/                   (LocalFs + S3 adapters — M3)
  │ ── search/                    (Tantivy — M11)
  │ ── cache/                     (moka L1, fred L2 — M10)
  │ ── email/                     (lettre — later)

L5 · runtime  (crates/runtime)
  │ ── supervisor.rs              (TaskSupervisor + TaskCategory — M1)
  │ ── observability.rs           (tracing + OTLP + Prometheus — M1)

L6 · tenant  (crates/tenant)
  │ ── context.rs                 (TenantContext)
  │ ── resolver.rs                (InMemory + PgTenantResolver — M3)
  │ ── with_tenant.rs             (single-RTT GUC helper — M2 W6)
  │ ── middleware.rs              (resolve_tenant axum middleware)

L7 · cross-cutting
  │ ── common                     (TenantId, AppError, Page<T>, sanitize, ids)
  │ ── pool-validator             (§3.4.1 RLS contract — M1)
  │ ── extension-api              (M9 — semver contract for WASM plugins)
  │ ── theme-api                  (M8 — semver contract for themes)
  │ ── image-pipeline             (libvips workers — M3)
  │ ── search-engine              (Tantivy index wrapper — M11)
  │ ── edge-adapter               (CF Workers contracts — M12)

Apps:
  apps/server                     (single-binary bootstrap; 3 pools; supervisors)
  apps/cli                        (nas2-cli: db migrate, themes pack, backup, import)
```

**M7's footprint** is concentrated in `crates/presentation/src/admin/`
— the previously-empty admin/ tree (scaffold from M5 W4) gets filled
with five island sub-modules. NO new crates. NO new workspace deps
(leptos, leptos_axum, leptos_meta are already in workspace).

---

## §3 · Decomposition rationale — why these 20 days, in this order

### §3.1 · Build order (strict dependency chain)

```
W1 (admin shell)    →    W2 (post list)    →    W3 (block editor)
  │                       │                       │
  │ provides:             │ provides:             │ provides:
  │  - layout             │  - route to editor    │  - autosave path
  │  - route guards       │  - selected post id   │  - inline media use
  │  - capability nav     │                       │
  │  - login/logout UX    │                       │
  │                       │                       │
  └──────────────────────►W4 (media + users + RETRO)
                            │ depends on W1 (guards) + W3 (editor uses media)
```

**Why W1 first:** every other island needs a layout + guard
infrastructure. Building post-list before guards = re-doing
authentication per island.

**Why W2 before W3:** post-list is the navigation entry to the
editor. Editor needs an ID to load; without list, the editor is a
URL-only artifact. Also, the patterns established in W2 (Leptos
table island + URL state) carry directly into W3 (canvas state).

**Why W3 (editor) before W4 (media browser):** the editor calls
*into* the media browser as a sub-modal for image-block insertion.
W4 D2 (upload from editor) physically depends on the editor existing
in usable form.

**Why W4 D4 (user manager) before D5 (RETRO):** user manager is the
last surface needed for the demo flow ("show me a role's caps"); it
also stresses the matrix-rendering pattern that other future
admin pages (M9 plugin manager, M8 theme manager) will reuse.

### §3.2 · ADR-019 (Leptos islands state pattern) — write D1, applied throughout

By W2 D1 we hit the first real reactive surface. Before that day's
code lands, ADR-019 picks **signal-based per-island** with
context-passing for cross-island state (e.g. selected post id flowing
to the editor). Reason vs alternatives:

| Alternative | Why rejected |
|---|---|
| Single global RwSignal | breaks island boundaries; SSR snapshot pollution |
| leptos_router state | only handles URL params; can't carry rich state |
| Reactive context with leptos::provide_context | **chosen** — explicit, scoped to admin shell, SSR-friendly |

### §3.3 · Why "Leptos islands path" not "JSON textarea path" for the editor

Per the M3 G1 ADR-010 decision, the project picks Leptos islands.
This is reflected in M7 W3 (5 days for block editor). If ADR-010
had picked the textarea fallback path, M7 W3 would shrink to 2 days
and the freed budget would shift to M8 themes work earlier. The
prompts below assume **Leptos islands chosen**.

### §3.4 · Quality budgets enforced every day

Per the master roadmap quality contract:

- Every new component: ≥ 2 island-render tests (server + hydrated)
- Every new admin route: capability-coverage marker (W4 D2 gate from M1 catches)
- Every form: CSRF token wired (M2 W8 D3 middleware enforces)
- Every API call from islands: typed via the existing OpenAPI utoipa types (M4 G5 wired)
- No raw `tokio::spawn` (M1 magic-check catches; supervisor only)
- No new workspace `Cargo.toml` deps
- Daily perf bench delta in SESSION_LOG if the day touched server-rendered code

### §3.5 · The capability matrix this month exercises

Admin pages require:
- `cms.page.read` — view list
- `cms.page.draft` — create/edit drafts
- `cms.page.publish` — change status to Published / Scheduled
- `media.upload` — use the upload UI
- `manage.users` — view + edit user manager
- `manage.site` — admin-only routes (super-admin)

The capability-aware nav (W1 D2) renders only items the user can
reach. Routes themselves enforce via `caps.require()` (M1 W4 D2 gate
catches missing markers).

### §3.6 · M7 risk register (week-specific risks roll up here)

| # | Risk | Likelihood | Impact | Mitigation |
|---:|---|---|---|---|
| 1 | Leptos islands hydration mismatch (SSR ≠ client first render) | High | Med | strict pattern: islands READ initial state from server-injected JSON, NEVER fetch on mount |
| 2 | Block editor drag-drop pulls in 50KB+ JS lib (sortable.js) | Med | Med | use Leptos signal-based reorder with explicit up/down + later DnD; ship without DnD on D3 if perf budget pressured |
| 3 | Autosave races with manual save → revision storm | Med | Med | autosave debounce 30s + locked `is_saving` flag; rejects concurrent autosave; M3 protocol survives |
| 4 | CSRF token expiration mid-edit → silent 403 on save | High | High | refresh CSRF on every server-rendered page load; client retries with fresh token on 403 once |
| 5 | bulk-status-change cross-tenant injection via mass-assigned IDs | Low | **High** | RLS catches at DB layer (M2 W5); double-check at AppState by re-extracting tenant_id from request, NEVER from body |
| 6 | Media upload from editor triggers re-render of unrelated islands | Med | Low | use leptos::create_signal scoped to editor; explicit invalidation, no global refresh |
| 7 | Leptos 0.7 SSR + hydration debug churn (compile times) | High | Low | cargo-leptos watch mode in dev; CI uses production build |

### §3.7 · Definition of "done" for M7 (the demo script)

An editor sitting at their laptop on 2026-12-04 can:

1. Open https://demo.ax-cms/admin/login
2. Log in with seeded admin@example.com / Admin123!
3. See the dashboard with capability-filtered nav (Posts / Media / Users visible)
4. Navigate to Posts → filtered list of all posts with status/author filters
5. Click "New Post" → empty editor canvas
6. Insert a Heading block + Paragraph block + Image block (Image picker opens media browser)
7. Upload an image from the picker
8. See the image variant generated within ~3 seconds (libvips worker)
9. Set alt text inline
10. Type into the paragraph
11. Watch autosave indicator pulse every 30s
12. Click Publish → see status transition + redirect to public preview URL
13. Open the public URL in incognito → see the rendered post

That's the demo. Every step has a corresponding day of work in M7.

---

## §4 · Day-by-day catalogue

Detailed prompts live in `docs/session-plans/daily/2026-11-09.md`
through `daily/2026-12-04.md`. Index:

| Date | Day | Goal | File | One-line scope |
|---|---|---|---|---|
| 2026-11-09 | W1·D1 Mon | G1 | `daily/2026-11-09.md` | Admin route guards + per-route capability gate middleware |
| 2026-11-10 | W1·D2 Tue | G1 | `daily/2026-11-10.md` | Capability-aware nav rendering + ADR-019 (Leptos state pattern) |
| 2026-11-11 | W1·D3 Wed | G1 | `daily/2026-11-11.md` | Breadcrumbs component + Path-extractor hierarchy |
| 2026-11-12 | W1·D4 Thu | G1 | `daily/2026-11-12.md` | Sliding-session JWT refresh + logout endpoint |
| 2026-11-13 | W1·D5 Fri | G1 | `daily/2026-11-13.md` | Session timeout UX + login redirect + W1 integration tests |
| 2026-11-16 | W2·D1 Mon | G2 | `daily/2026-11-16.md` | PostList island scaffold + server-rendered first page |
| 2026-11-17 | W2·D2 Tue | G2 | `daily/2026-11-17.md` | Sort + pagination controls with URL state sync |
| 2026-11-18 | W2·D3 Wed | G2 | `daily/2026-11-18.md` | Filter by status / author / term (server-side query) |
| 2026-11-19 | W2·D4 Thu | G2 | `daily/2026-11-19.md` | Bulk-select checkboxes + bulk-action UI |
| 2026-11-20 | W2·D5 Fri | G2 | `daily/2026-11-20.md` | Optimistic bulk-status with rollback + e2e test |
| 2026-11-23 | W3·D1 Mon | G3 | `daily/2026-11-23.md` | Block editor canvas — render Vec<Block> editable |
| 2026-11-24 | W3·D2 Tue | G3 | `daily/2026-11-24.md` | Block insert UI — palette + slash-command |
| 2026-11-25 | W3·D3 Wed | G3 | `daily/2026-11-25.md` | Block reorder — up/down (DnD if budget allows) |
| 2026-11-26 | W3·D4 Thu | G3 | `daily/2026-11-26.md` | Per-variant editors (Heading / Paragraph / Image / CodeBlock) |
| 2026-11-27 | W3·D5 Fri | G3 | `daily/2026-11-27.md` | Block delete + autosave wiring (PATCH ?autosave=true) |
| 2026-11-30 | W4·D1 Mon | G4 | `daily/2026-11-30.md` | Media browser island — grid + thumbnails + infinite scroll |
| 2026-12-01 | W4·D2 Tue | G4 | `daily/2026-12-01.md` | Upload from editor (drag-drop into canvas) |
| 2026-12-02 | W4·D3 Wed | G4 | `daily/2026-12-02.md` | Alt-text inline editor + save via PATCH /api/v1/media/:id |
| 2026-12-03 | W4·D4 Thu | G5 | `daily/2026-12-03.md` | User & role manager view (read-only matrix) |
| 2026-12-04 | W4·D5 Fri | RETRO | `daily/2026-12-04.md` | 🏆 Demo milestone + RETRO + M8 bootstrap |

---

## §5 · Cross-day invariants

Every day in M7:

- Starts with the standard `P0 · Verification gate (V1..V4)`
- Spine files DO NOT change unless the day explicitly says so
  (M7 has **zero** spine touches — pure additive presentation work)
- No new workspace `Cargo.toml` deps — `leptos`, `leptos_axum`,
  `leptos_meta`, `serde`, `serde_json`, `axum` already cover M7's
  needs
- `xtask architecture-check / magic-check / capability-coverage /
  check-planning-refs` must all stay green at end-of-day
- Every island gets at least one server-render test + at least one
  hydration smoke test (where applicable)
- Every form submission goes through CSRF middleware (M2 W8 D3)
- Capability markers `caps.require(<cap.name>)` on every admin
  route handler (M1 W4 D2 capability-coverage gate enforces)
- Daily commit message references PLAN-G1 through PLAN-G5 as
  appropriate (check-planning-refs gate)

---

## §6 · After-M7 carry-over predictions

These items are EXPECTED to surface in the M7 RETRO §7 (Next-month
proposals) — pre-noting so M8 bootstrap doesn't have to rediscover:

1. **Public theme renderer + template hierarchy** — M8 G3-G4 already
   schedules this; M7 leaves a TODO at the public route `/`.
2. **Admin polish backlog** — tooltips, keyboard shortcuts,
   accessibility audit — defer to a "M7.5" technical-debt sprint or
   absorb into M9 plugin manager UI work.
3. **Editor: more block-specific UIs** (Gallery image-picker,
   Table-cell editor) — M7 covers Heading/Paragraph/Image/CodeBlock;
   the other 15 variants from M4 get JSON-textarea fallback rendering
   in M7, real editors land in M9-M10 polish.
4. **Real-time collab** — explicit non-goal of Y1; year-2 with `yrs`
   CRDT.
5. **Mobile/tablet admin layout** — defer to M9; current admin is
   desktop-first.

These predictions get reconciled in RETRO §7 against actual M7
results.

---

## §7 · How to use this document

- Each daily prompt in `docs/session-plans/daily/2026-11-09.md` ..
  `2026-12-04.md` is **self-contained** — paste verbatim as Claude
  Code opening message; AVTONOM executes the day.
- This meta-doc is the **why** layer. When a daily prompt makes a
  non-obvious choice (e.g. "no DnD library, signal-based reorder
  only"), the reasoning is here in §3.6.
- If reality diverges (e.g. a day overruns), the carry-over flows
  into the next day's `## CARRY-OVER from <prev>` block. RETRO
  reconciles at month-end.
