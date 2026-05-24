# BRIDGE · 04 — DESIGN (визуальная и UX спека)

> Назначение: AI-кодер должен **выдать тот же HTML / CSS-vars / fonts**, что SITE1, иначе тенант увидит «другой сайт». Этот файл — точная фотография визуальной системы NAS на уровне DOM/CSS, плюс отдельные правила для admin UI (которое мы НЕ переписываем, но обязаны не сломать interop).
>
> Источник правды дизайна тенантских сайтов — runtime `tenant_design_tokens` (per-tenant). Источник правды дизайна `/admin` UI — `barbie/SITE1/dashboard-2077.html` (см. memory `project_nas_dashboard_design_source`).

---

## 1. Двойная дизайн-система NAS

NAS работает с **двумя независимыми визуальными мирами**, которые легко перепутать:

| Мир | Кто видит | Где живёт дизайн | Что AX делает |
|-----|-----------|------------------|---------------|
| **Tenant public sites** (`/{slug}`, `/{slug}/{page}`) | Конечные клиенты салона | `tenant_design_tokens` per-tenant (БД) + CSS-vars + Google Fonts | **AX рендерит** в Phase A |
| **NAS admin** (`/admin/*`) | Staff платформы (platform-admin, tenant-admin) | Tailwind + Rufo шрифт + dashboard-2077.html ground truth | **AX НЕ трогает** (остаётся Next.js) |

**Категорическое правило:** AX рендерит **только публичные tenant-страницы**. UI админки — это спайн SITE1, переписывать в Leptos out-of-scope (даже отдалённо).

---

## 2. Tenant public sites — design tokens system

### 2.1 Источник: схема таблицы

`packages/db/src/schema/tenant-design-tokens.ts:27-68`:

```typescript
export const tenantDesignTokens = pgTable('tenant_design_tokens', {
  tenantId: uuid('tenant_id').references(() => tenants.id, { onDelete: 'cascade' }).primaryKey(),

  // Цвета — 4 ролевых, hex 6/8 (с alpha)
  bg:         varchar('bg',         { length: 16 }).notNull().default('#FFFFFF'),
  headColor:  varchar('head_color', { length: 16 }).notNull().default('#0A0A0A'),
  accColor:   varchar('acc_color',  { length: 16 }).notNull().default('#D4AF37'),
  bodyColor:  varchar('body_color', { length: 16 }).notNull().default('#1A1A1A'),

  // Шрифты — 3 ролевых (Google Fonts по имени)
  headFont:   varchar('head_font',  { length: 64 }).notNull().default('Unbounded'),
  accFont:    varchar('acc_font',   { length: 64 }).notNull().default('Unbounded'),
  bodyFont:   varchar('body_font',  { length: 64 }).notNull().default('Inter'),

  // Brand assets — S3 keys
  logoKey:    varchar('logo_key',    { length: 500 }),
  logoAlt:    varchar('logo_alt',    { length: 255 }),
  faviconKey: varchar('favicon_key', { length: 500 }),

  // Layout/nav
  navTemplate: varchar('nav_template', { length: 32 })
    .$type<'top-classic' | 'mega-images' | 'vertical-side'>()
    .notNull()
    .default('top-classic'),

  // Escape hatches (Phase B)
  customCss: text('custom_css'),
  extras:    jsonb('extras').$type<Record<string, string>>().default(sql`'{}'::jsonb`),

  updatedAt: timestamp('updated_at').defaultNow().notNull(),
}, (_t) => ({
  navTemplateCheck: check('tenant_design_tokens_nav_template_check',
    sql`nav_template IN ('top-classic','mega-images','vertical-side')`),
  colorFormatCheck: check('tenant_design_tokens_colors_hex_check',
    sql`bg ~ '^#[0-9A-Fa-f]{6,8}$' AND head_color ~ '^#[0-9A-Fa-f]{6,8}$'`),
}));
```

**3-rolled colors + 3-rolled fonts** — это **жёсткий контракт**. Тенант не может определить «5-й цвет accent2» — только escape через `customCss` или `extras` (write-side, не в Phase A).

### 2.2 Roles → CSS-vars mapping

`apps/web/src/components/tenant-site/TenantSiteShell.tsx:44-52`:

```typescript
const styleVars: React.CSSProperties = {
  '--bg':          dt.bg,
  '--head-color':  dt.headColor,
  '--head-font':   `'${dt.headFont}', serif`,
  '--acc-color':   dt.accColor,
  '--acc-font':    `'${dt.accFont}', serif`,
  '--body-color':  dt.bodyColor,
  '--body-font':   `'${dt.bodyFont}', system-ui, sans-serif`,
} as React.CSSProperties;
```

**AX обязан вывести идентичный CSS-vars блок** в `<style>` или `style="..."` атрибуте root-обёртки. Имена переменных — точно те же, fallback'и (`serif`, `system-ui, sans-serif`) — те же.

### 2.3 Inline style block (полный CSS, который выводит TenantSiteShell)

`apps/web/src/components/tenant-site/TenantSiteShell.tsx:70-98`:

```css
.tenant-site {
  background: var(--bg);
  color: var(--body-color);
  font-family: var(--body-font);
  line-height: 1.6;
}
.tenant-site h1, .tenant-site h2, .tenant-site h3 {
  font-family: var(--head-font);
  color: var(--head-color);
  letter-spacing: -0.01em;
  line-height: 1.1;
}
.tenant-site .accent {
  font-family: var(--acc-font);
  color: var(--acc-color);
}
.tenant-site a {
  color: var(--acc-color);
  text-decoration: none;
  transition: opacity 0.2s;
}
.tenant-site a:hover { opacity: 0.7; }
.tenant-site .container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 1.5rem;
}
```

**AX'у:** этот блок в `crates/presentation/src/leptos/tenant_shell.rs` повторить **дословно**. Не «улучшать», не консолидировать с другими блоками, не выносить в external `.css` — inline для cacheability и для совместимости с edge-rendering (Caddy не должен ждать отдельный CSS-fetch).

### 2.4 Google Fonts loading

`apps/web/src/components/tenant-site/TenantSiteShell.tsx:14-20`:

```typescript
function buildGoogleFontsUrl(...families: string[]): string {
  const dedup = Array.from(new Set(families.filter(Boolean)));
  const params = dedup
    .map((f) => `family=${encodeURIComponent(f).replace(/%20/g, '+')}:wght@300;400;500;600;700`)
    .join('&');
  return `https://fonts.googleapis.com/css2?${params}&display=swap`;
}
```

Используется в `<head>`:

```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Unbounded:wght@300;400;500;600;700&family=Inter:wght@300;400;500;600;700&display=swap" />
```

**Веса:** 300/400/500/600/700 (фиксированный набор).
**Display:** `swap` (без FOIT).
**Dedup:** если `headFont == accFont == bodyFont` — одно `family=`.

**AX repro:**

```rust
fn build_google_fonts_url(families: &[&str]) -> String {
    let mut seen = std::collections::BTreeSet::new();
    let params: Vec<String> = families.iter()
        .filter(|f| !f.is_empty() && seen.insert(*f))
        .map(|f| format!("family={}:wght@300;400;500;600;700",
            urlencoding::encode(f).replace("%20", "+")))
        .collect();
    format!("https://fonts.googleapis.com/css2?{}&display=swap", params.join("&"))
}
```

### 2.5 Override mechanism — `?td=base64(tokens)`

`apps/web/src/lib/td-overrides.ts`:

```typescript
const ALLOWED_KEYS: ReadonlyArray<DesignTokenKey> = [
  'bg', 'headColor', 'headFont', 'accColor', 'accFont', 'bodyColor', 'bodyFont',
];

export function decodeTdParam(td: string | string[] | undefined): Partial<TenantDesignTokens> | undefined {
  if (!td || typeof td !== 'string') return undefined;
  try {
    const json = Buffer.from(td, 'base64').toString('utf8');
    const parsed = JSON.parse(json);
    if (!parsed || typeof parsed !== 'object') return undefined;
    const out: Partial<TenantDesignTokens> = {};
    for (const k of ALLOWED_KEYS) {
      const v = (parsed as Record<string, unknown>)[k];
      // Whitelisted keys + length check (anti-injection)
      if (typeof v === 'string' && v.length > 0 && v.length < 200) {
        out[k] = v;
      }
    }
    return Object.keys(out).length > 0 ? out : undefined;
  } catch { return undefined; }
}
```

**AX repro** (`crates/presentation/src/middleware/td_overrides.rs`):

```rust
use serde::Deserialize;
use base64::Engine as _;

#[derive(Debug, Default, Deserialize)]
pub struct TdOverrides {
    pub bg: Option<String>,
    pub head_color: Option<String>,
    pub head_font: Option<String>,
    pub acc_color: Option<String>,
    pub acc_font: Option<String>,
    pub body_color: Option<String>,
    pub body_font: Option<String>,
}

impl TdOverrides {
    /// SECURITY: whitelist'ed keys + length cap (200) — anti-injection в CSS context
    pub fn decode_from_query_param(td: &str) -> Option<Self> {
        let bytes = base64::engine::general_purpose::STANDARD.decode(td).ok()?;
        let raw: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        let mut out: TdOverrides = serde_json::from_value(raw).unwrap_or_default();
        // length sanity на каждое поле
        out.cap_lengths(200);
        Some(out).filter(|o| o.any())
    }
}
```

**Use case AX в Phase A:** при render PublishedPage в HTML, презентация мерджит `TenantDesignTokens` ← `TdOverrides`. Идентично SITE1 (`TenantSiteShell.tsx:31-33`).

---

## 3. Layout templates (`nav_template`)

`tenant_design_tokens.nav_template` принимает три значения. От него зависит layout class root-обёртки:

`apps/web/src/components/tenant-site/TenantSiteShell.tsx:56-59`:

```typescript
const layoutClass =
  menu.template === 'vertical-side'
    ? 'tenant-site min-h-screen md:pl-64'   // sidebar 16rem на md+, mobile collapse
    : 'tenant-site min-h-screen';           // top-classic + mega-images используют top-nav
```

Все три варианта рисует компонент `Navigation` — он сам решает по `menu.template`. В Phase A (cms_pages public render по `/{slug}/{page}`) **Navigation скорее всего вообще не рисуется** — это часть home/landing-route, а не CMS-page route. См. `apps/web/src/app/(tenants)/imperiumspa/[slug]/page.tsx:27-32` — рендерится только `<main>` с `<EdRenderer>` и `<TenantEditFab>`.

**AX правило для Phase A:** для CMS-page route рисует **только** `<main>` с ED-секциями. Не рисует navigation, не рисует footer (пока). Если потом захочется home-route (тоже на AX) — `Navigation` переезжает отдельным циклом.

---

## 4. ED-renderer (главный формат страниц после Stage 28)

### 4.1 Section / Column / Widget tree

`apps/web/src/components/cms/ed-editor/EdRenderer.tsx` (упрощённо):

```typescript
interface Section {
  id: string;
  columns: Column[];
  style?: { padding?: string; bg?: string };
}
interface Column {
  id: string;
  width: 1 | 2 | 3 | 4 | 6 | 12;   // 12-grid units
  widgets: Widget[];
}
type Widget =
  | HeadingWidget | TextWidget | ButtonWidget | DividerWidget
  | SpacerWidget | IconBoxWidget | CtaWidget | ImageWidget;
```

### 4.2 Render rules (per widget)

| Widget type | Что рисует | CSS-vars use? |
|-------------|-----------|--------------|
| `heading` | `<h1>/<h2>/<h3>` с inline style `color`, `font-family`, `text-align` | НЕТ — берёт из props (inline-style override) |
| `text` | `<div>` с `dangerouslySetInnerHTML` (SAFE — body уже sanitized на write) | inline style color, font-family |
| `button` | `<a>` с classes `btn btn-{variant} btn-{size}` | accent color через class |
| `divider` | `<hr>` с inline `border-width`, `border-color` | НЕТ |
| `spacer` | `<div style="height: Npx">` | НЕТ |
| `iconBox` | `<div class="icon-box">` с иконкой + title + description | accent для иконки |
| `cta` | большой блок с background, title, subtitle, button | accent color background |
| `image` | `<img src alt width height loading=lazy>` | НЕТ |

**Принцип SITE1 (повторить):** каждый widget использует **inline-style** для color/font (потому что widget редактор задаёт точечно), а **не** CSS-vars. CSS-vars — это **defaults тенанта**, widget'ы могут отвергнуть default'ы своим inline-style.

**AX в Leptos** (`crates/presentation/src/leptos/cms_page_component.rs`):

```rust
use leptos::*;

#[component]
pub fn EdRenderer(sections: Vec<Section>) -> impl IntoView {
    view! {
        <div class="ed-canvas">
            { sections.into_iter().map(|s| view! {
                <section class="ed-section" id=s.id.clone() style=section_style(&s)>
                    <div class="ed-row" style="display:flex;gap:1rem;flex-wrap:wrap;">
                        { s.columns.into_iter().map(|c| view! {
                            <div class=format!("ed-col col-{}", c.width) style=col_style(&c)>
                                { c.widgets.into_iter().map(render_widget).collect_view() }
                            </div>
                        }).collect_view() }
                    </div>
                </section>
            }).collect_view() }
        </div>
    }
}

fn render_widget(w: Widget) -> impl IntoView {
    match w {
        Widget::Heading(h) => view! {
            <h1 style=format!("color:{};font-family:{};text-align:{}",
                h.color.unwrap_or_default(), h.font.unwrap_or_default(), h.align.unwrap_or_default())>
                {h.text}
            </h1>
        }.into_view(),
        Widget::Text(t) => view! {
            <div inner_html=sanitize_for_render(&t.html)
                 style=format!("color:{};font-family:{}", t.color.unwrap_or_default(), t.font.unwrap_or_default()) />
        }.into_view(),
        // ... остальные widget types
    }
}
```

**SAFETY:** `inner_html` принимает HTML, который пришёл из БД. БД-content — sanitized на write (SITE1 Stage 35: `sanitize-html` для WP-import; manual edit идёт через ED-editor, который тоже sanitize'ит). AX дополнительно **на render** прогоняет через `ammonia` (Rust port DOMPurify) — defence in depth, в случае если SITE1 admin создал страницу до Stage 35 без sanitize.

### 4.3 Grid system (12-column)

SITE1 не использует Tailwind на tenant-сайтах — все размеры inline или через ed-canvas/ed-section/ed-col классы. Column width маппится так:

| `width` | flex-basis (приблизительно) |
|---------|---------------------------|
| 1 | 8.33% |
| 2 | 16.66% |
| 3 | 25% |
| 4 | 33.33% |
| 6 | 50% |
| 12 | 100% |

**AX render:** в `tenant_shell.rs` inline CSS добавляет правила:

```css
.ed-section { width: 100%; padding: 2rem 0; }
.ed-row { max-width: 1200px; margin: 0 auto; padding: 0 1.5rem; }
.ed-col.col-1  { flex: 0 0 8.33%; }
.ed-col.col-2  { flex: 0 0 16.66%; }
.ed-col.col-3  { flex: 0 0 25%; }
.ed-col.col-4  { flex: 0 0 33.33%; }
.ed-col.col-6  { flex: 0 0 50%; }
.ed-col.col-12 { flex: 0 0 100%; }
@media (max-width: 768px) {
  .ed-col { flex: 0 0 100% !important; }   /* mobile stack */
}
```

---

## 5. Meta tags (SEO/Open Graph)

Из `cms_pages.metaTitle` и `cms_pages.metaDescription`:

```html
<head>
  <title>{ metaTitle || title }</title>
  <meta name="description" content="{ metaDescription }" />
  <meta property="og:title" content="{ metaTitle || title }" />
  <meta property="og:description" content="{ metaDescription }" />
  <meta property="og:type" content="website" />
  <meta property="og:url" content="https://{slug}.spa.me/{page.slug}" />
  { coverImageKey && <meta property="og:image" content="{ resolveMediaUrl(coverImageKey) }" /> }
  <link rel="icon" href="{ resolveMediaUrl(designTokens.faviconKey) || '/favicon.ico' }" />
</head>
```

**AX обязан** все эти `<meta>` рисовать одинаково. Open Graph — это часть contract identity; если AX выдаст другой shape, Facebook/Twitter preview ломается для пользователя.

---

## 6. Admin UI (контекст — AX НЕ переписывает, но обязан понять)

### 6.1 Ground truth — `barbie/SITE1/dashboard-2077.html`

Per memory `project_nas_dashboard_design_source`: dashboard-2077.html — **canonical** для всей `/admin` визуальной системы.

Что определяет dashboard-2077.html:
- Цветовая палитра admin'а: `--bg`, `--bg-elev`, `--surface`, `--line`, `--text`, `--text-dim`, `--text-mute`, `--gold` (#D4AF37), `--accent-2`, `--green`, `--red`
- Шрифт: **RF Rufo** (RF Dewi алтернатива) для headings, JetBrains Mono для dashboard цифр (Stage 19)
- Rail layout: 56px sidebar, icon-only, pill expand на hover
- "Scoop" — inverse border radius технология (см. memory `design_inverse_radius_term`)

### 6.2 Что AX НЕ имитирует

- **Никакой** RF Rufo в tenant-сайтах. Это admin-only.
- **Никакой** dashboard-2077 палитры в tenant-сайтах. Tenant palette приходит из `tenant_design_tokens`.
- **Никакого** `/admin/*` route в Axum router. Все `/admin/*` → SITE1 (Caddy `not path /admin/*`).

### 6.3 Что AX обязан учитывать (interop)

- Tenant admin авторизуется в SITE1 (`/admin/login`), получает JWT. Этот **тот же** JWT принимается AX (общий `JWT_SECRET`).
- При создании/редактировании страницы — Admin UI отправляет запрос в **SITE1 API** (canonical writer); AX не участвует в write path.
- После save — SITE1 публикует `pgmq` event `cms.page.published` (если решим cache invalidation в AX); AX subscribe'ится, инвалидирует local cache (если есть).

---

## 7. Per-tenant brand examples (что в production уже есть)

Из `apps/web/src/lib/projects-data.ts` (10 seeded тенантов из dashboard mock'а):

| Slug | Brand vibe | bg | headColor | headFont | accColor |
|------|-----------|----|----------:|----------|----------|
| pentagon | Тактический эскорт, тёмный strict | #0A0A0C | #FFFFFF | Montserrat Alternates | #DC2626 (red) |
| dachaspa | Wellness retreat, светлый бежевый | #FAFAF7 | #3A3A3A | Cormorant Garamond | #B8634D (terracotta) |
| barbiespa | Luxury feminine pink | #FFB6D9 | #FF1493 | Outfit | #FFFFFF |
| imperiumspa | Private members club, тёмный gold | #0B0908 | #F0EBE0 | Bodoni Moda | #C9A961 (gold) |
| roxy-spa | Cyberpunk neon | #0A0F2C | #22D3EE (cyan) | Orbitron | #EC4899 (pink) |
| ... | ... | ... | ... | ... | ... |

**Что это значит для AX:** дизайн-система должна корректно отрисовать **очень разные** темы из одного и того же `Section[]` tree. CSS-vars + inline style widgets дают этот контракт; AX повторяет это **дословно**.

Контрольная проверка после AX-bootstrap'а: render одной и той же тестовой страницы (`tests/integration/test_page_design_variance.rs`) с разными `TenantDesignTokens` (взять 3 контрастных тенанта — pentagon, barbiespa, roxy-spa) и убедиться, что HTML отличается только в CSS-vars и Google Fonts URL. Если отличия больше — где-то прокралась hard-coded стилизация.

---

## 8. Responsive & accessibility

| Аспект | SITE1 (что есть) | AX обязан |
|--------|------------------|----------|
| Mobile breakpoint | `@media (max-width: 768px)` для grid collapse | Same |
| Container max-width | `1200px` | Same |
| Container padding | `0 1.5rem` | Same |
| `lang` атрибут | `<html lang="ru">` (per tenant locale) | Same — берёт из `page.locale` |
| `<img alt>` | from widget props или `logoAlt` | Same |
| `loading="lazy"` на img | Yes | Same |
| Кеш-aware `?td=` | Не cache'ится (no-store) | Same |
| Heading hierarchy | h1 один на странице (из widget или title) | Validate в Leptos render — debug_assert на dev build |

---

## 9. Чек-лист «AX выдал тот же дизайн»

После Phase A bootstrap, на одной и той же `cms_pages` row:

- [ ] `<head>` содержит `<link rel="stylesheet" href="https://fonts.googleapis.com/css2?...&display=swap">` с теми же families в том же порядке
- [ ] `<head>` содержит `<link rel="preconnect" href="https://fonts.googleapis.com">` + `<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>` (точно так же)
- [ ] Root `<div class="tenant-site min-h-screen">` (или `tenant-site min-h-screen md:pl-64` если `nav_template == 'vertical-side'`)
- [ ] Inline `<style>` блок с 7 правилами из §2.3 — символ-в-символ
- [ ] CSS-vars `--bg`, `--head-color`, `--head-font`, `--acc-color`, `--acc-font`, `--body-color`, `--body-font` — те же значения, те же имена
- [ ] `?td=base64(...)` корректно мерджит override на defaults (whitelist + length check)
- [ ] `<meta>` теги (title, description, og:*) — те же значения
- [ ] `<link rel="icon">` указывает на `faviconKey` тенанта (если задан)
- [ ] Section/Column/Widget renders inline style identical to SITE1 EdRenderer
- [ ] Grid CSS (col-1..col-12 + mobile stack) присутствует
- [ ] `<img alt>` всегда заполнен (debug_assert на render)
- [ ] Никаких admin-only CSS (`--gold`, RF Rufo) на tenant странице

Снять diff: запустить SITE1 на `:3011` и AX на `:7000` параллельно, открыть один и тот же тенант + страницу в обоих, использовать `diff` на HTML body (после нормализации whitespace). Допустимые отличия — `X-Stack` header, `X-Request-Id` value, timestamps в meta. Всё остальное **обязано** совпадать.
