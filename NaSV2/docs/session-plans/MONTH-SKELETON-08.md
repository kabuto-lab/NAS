# MONTH-SKELETON M8 · 2026-12-07 → 2026-12-31 · Themes API + first bundled theme + template hierarchy

## Phase

P3 Admin + themes (end). Year-end short month (~18 working days).

## Assumed entering state

- Admin panel: login → post editor → media → users works end-to-end
- ~60% WP parity (M7 demo milestone)

## Goals

**G1 · ADR-013: `ax.theme.toml` vs WP `theme.json` compat**
Decision: ship `ax.theme.toml` natively + a `theme.json → ax.theme.toml`
converter (M12 has the WP importer; theme-side converter ships now to
unblock theme migration tooling later). Schema covers: templates,
template-parts, image variants, color palette, typography, layout
breakpoints, settings (key-value with type hints).

**G2 · Theme installer + workspace integration**
`themes/` workspace member directory. Each theme = `themes/<slug>/`
with `ax.theme.toml` manifest + `templates/*.rinja` + `assets/`.
`ThemeRegistry` loads at boot; `ThemeInstaller` accepts uploaded
`.tar.zst` archives + verifies signature (ADR-020 covers signing).

**G3 · Template hierarchy resolver**
Given URL + tenant + content type, resolve a template chain. WP-style
order: `single-{post-type}-{slug}.rinja` → `single-{post-type}.rinja`
→ `single.rinja` → `index.rinja` → 404 template. Cached per
`(tenant, theme, url-pattern)` — fits M10 cache key contract.

**G4 · Bundled `themes/minimal/`**
Three templates: `index.rinja` (post list), `single.rinja` (single
post), `archive.rinja` (term archive). No CSS framework — semantic
HTML5 + minimal `<style>` block. Proves the theme API works end-to-end.

**G5 · Theme manager UI**
Admin panel `/admin/themes`: list installed themes, activate, preview
in iframe, upload new archive (admin capability). Active theme shown
prominently. Per-tenant active theme.

## Weekly themes

| Week | Dates | Theme |
|---|---|---|
| M8 W1 | 12-07..12-11 | ADR-013 + theme manifest parser |
| M8 W2 | 12-14..12-18 | Installer + template hierarchy resolver |
| M8 W3 | 12-21..12-25 (light) | Bundled `themes/minimal/` |
| M8 W4 | 12-28..12-31 (4 days) | Theme manager UI + RETRO |

## ADRs needed

- **ADR-013** — `ax.theme.toml` schema
- **ADR-020** — theme archive signing (ed25519 + public-key registry)

## Migrations

- `0013_themes.sql` — installed themes per tenant + active selection

## Exit criteria

1. `themes/minimal/` loads; `index.rinja` renders a list of posts at
   `https://<host>/` (no `/api/` prefix — public path)
2. Theme manager admin page renders installed themes + activate works
3. Template hierarchy cache reduces resolve to single hash lookup
4. `cargo run -p nas2-cli -- themes pack themes/minimal` produces
   a signed `.tar.zst`
5. ~70% WP parity

## Carry-over seed for M9

- Extension API (M9 G2) shares signing infrastructure with themes
- Template hierarchy resolver is the M10 cache key composer
