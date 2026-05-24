# ENTITY — AX · NAS на Rust (Axum + Leptos)

**Версия:** v3.3
**Дата:** 2026-05-24
**Назначение:** конституция альтернативной реализации NAS на Rust-стеке (Axum + Leptos + SQLx + PostgreSQL).
**Тон документа:** не "как построить Rust SaaS", а "**как не дать Rust SaaS превратиться в неподдерживаемого монстра через 18 месяцев**".

---

## 0. Статус

**Этот документ — exploration, не commitment миграции.**

Production-таргет NAS — `barbie/SITE1/` (NestJS 10 + Drizzle + Next.js 15 + PostgreSQL 16). Phase 0 ~98%, 32 коммита. ROADMAP.md описывает Phase 1 (subscriptions, payments, OAuth, email).

**AX-документ существует для:**
- зафиксировать честную проектную спецификацию того, **что было бы**, если бы NAS строился с нуля на Rust в 2026 году
- использоваться как input для **Decision Criteria** в §13: при каких условиях миграция оправдана
- служить шаблоном для возможных производных проектов под `barbie/`, если бенчмарк AX превзойдёт SITE1 настолько, что бизнес-кейс становится положительным

**Дефолт:** SITE1 остаётся production-target. AX — параллельный артефакт.

**v3.0 переориентирует документ:** v1.0/v2.0 описывали **как система устроена**. v3.0 описывает **как она строится и как она НЕ деградирует**. Без §§2.5–2.8, §4.8, §4.9, §10.5, §11.5, §11.6, §12.5, §18–§22 архитектура неизбежно сползает в pseudo-clean accidental complexity, CQRS-ради-CQRS, allocation soup и shared-DB hell с SITE1.

**v3.1 встраивает AX в parent constitution** (`barbie/ENTITY.md`) — см. §0.1.

---

## 0.1 AX и parent constitution

Родительская конституция `barbie/ENTITY.md` — **canonical для всего workspace `barbie/`**. AX не дублирует и не отменяет её; этот документ — Rust-specific extension с явными границами:

| Тема | Источник правды | Роль AX-документа |
|------|-----------------|-------------------|
| Workspace правила, проекты под Barbie | `barbie/ENTITY.md §0–§3` | подчиняется |
| Стек SITE1 (NestJS + Drizzle + Next) | `barbie/ENTITY.md §1` | AX определяет **альтернативный** стек (§4) для exploration |
| Multi-tenant doctrine | `barbie/ENTITY.md §2.2` + `§9` | AX = compile-time усиление того же принципа (§3) |
| VPS deploy regulation | `barbie/ENTITY.md §6` | AX расширяет blue/green + expand-contract поверх baseline (§21.0) |
| TLA Entity для модулей SITE1 | `barbie/ENTITY.md §9` | для AX-внутренних модулей применим **с подменой стека** (Drizzle→SQLx, Nest→Axum); cross-stack решения = §2.5 RFC/ADR pipeline |
| Engineering Entity (NAS Core Architect v3) | `barbie/ENTITY.md §11` | AX **наследует** identity, security mentality, delivery standard, philosophy |
| Memory cross-session | `barbie/ENTITY.md §11 Cross-Session Continuity` | AX использует тот же `memory/` mechanism с префиксом `project_ax_*` (когда/если активируется) |
| CLAUDE.md §M режимы (MANUAL/SEMIAUTO/AVTONOM) | `CLAUDE.md §M` | этот документ = spine; правки только с явным ок |

**При конфликте AX vs `barbie/ENTITY.md` — побеждает родитель.** AX-документ — exploration spec, не parallel parliament.

**Что AX добавляет поверх parent:**
- compile-time enforcement многих принципов, которые в SITE1 проверяются runtime'ом (newtype `TenantId`, `TenantScoped<T>`, fitness functions §2.6, NO MAGIC POLICY §2.7)
- 4-layer RFC/ADR/Plan/Validation pipeline (§2.5) для cross-stack решений, которых нет внутри SITE1
- failure isolation на уровне отдельных tokio runtime'ов (§10.5) — невозможно в Node без worker_threads
- explicit interop boundary с SITE1 (§12.5) — anti-corruption layer и ownership table

---

## 1. Цель и не-цели

**Цель:** мульти-tenant CRM-платформа, дисциплинированно использующая возможности Rust-runtime'а:

- p95 < 80 ms на CRUD-эндпоинтах под 200 RPS
- стабильный RSS ≤ 256 MB на API-инстанс под 1000 параллельных tenant-сессий
- compile-time валидация SQL через SQLx prepared queries (offline mode для CI)
- compile-time enforcement tenant-scoping через типы (newtype `TenantId` + явные `Tenant`-aware repositories)
- 5-уровневая мульти-tenant защита, где **2 уровня compile-time, не runtime**
- **fitness functions** валидируют архитектурные инварианты на каждом PR (§2.6)
- **NO MAGIC** код-база: boring, explicit, predictable (§2.7)

**Не-цели:**

- "переписать всё" — если миграция когда-либо начнётся, она идёт **vertical slices** (один модуль за раз с feature parity), не big-bang rewrite
- многоязычная команда — Rust требует одну команду одного языка; AX исключает "TS на фронте, Rust на бэке" гибрид (Leptos закрывает обе стороны, но через replaceable contract — §4.9)
- эксперимент со стеком ради стека — каждое решение в §4 обосновано под мульти-tenant SaaS, не "хайп Rust"
- **"clever" Rust** — макро-DSL, runtime reflection-подобные паттерны, нестандартные proc-macro поведения запрещены §2.7
- **CQRS/Event Sourcing как дефолт** — §22 явно ограничивает применение

---

## 2. Архитектура — Four-Layer TLA

v1.0 описала 4-слойный pattern. v2.0 зафиксировала конкретные крейты и compile-time границы. v3.0 добавляет **enforcement layer** (§2.6) и **execution model** (§2.5).

| Слой | Crate | Содержание | Может зависеть от |
|------|-------|------------|------------------|
| L1 Presentation | `presentation/` (адаптеры) | Leptos / SPA / API / Admin адаптеры через единый contract (§4.9) | `application/`, `common/` |
| L2 Application | `application/` | Use cases, command/query handlers, port traits | `domain/`, `common/` |
| L3 Domain | `domain/` | Value objects, aggregates, domain events, invariants | `common/` (только базовые типы) |
| L4 Infrastructure | `infrastructure/` | SQLx repos, S3, queue, email, cache | `application/` (implements traits), `domain/` |

**Правила границ (compile-time enforced через Cargo workspace deps + §2.6 fitness checks):**

- `domain/` имеет только базовые crate-зависимости: `serde`, `uuid`, `chrono`, `garde`
- `application/` определяет **port traits**; `infrastructure/` их implements (adapter pattern)
- `presentation/` не делает прямых вызовов в `infrastructure/` — только через `application/`
- `infrastructure/` НЕ экспортирует `sqlx::Row`, `sqlx::PgRow`, `sqlx::Pool` наружу — только доменные сущности и port traits
- circular imports невозможны: Cargo workspace отрицает их структурно
- нарушение detect'ится `cargo xtask architecture-check` в CI (§2.6)

**Поток зависимостей:**

```
Presentation ─→ Application ─→ Domain
                    ↑              ↑
                    └─ Infrastructure
```

**Поправка к v1.0:** v1.0 сказала "Infrastructure знает SQL, но не знает бизнес-логики". Это создаёт ложное впечатление, что infrastructure-репо возвращают raw rows. На самом деле каждая infrastructure-implementation возвращает **доменные сущности**, mapping `row → domain` происходит внутри infrastructure-слоя, не утекает наружу.

---

## 2.5 Engineering Execution Model — Four-Layer Planning

**Связь с `barbie/ENTITY.md §9 TLA Entity`:**

- TLA Entity (Strategic / Architectural / Incremental) — protocol для модулей **внутри одного стека**. Применим в AX напрямую с заменой Drizzle→SQLx, Nest→Axum.
- §2.5 4-layer RFC/ADR/Plan/Validation — protocol для решений с **cross-stack последствиями** (§12.5 interop, миграция модуля с SITE1 на AX, добавление нового внешнего сервиса с парой adapter'ов SITE1+AX, любое изменение архитектурного invariant'а §2).
- Эти схемы **дополняют** друг друга, не конкурируют. Для модуля строго внутри AX — TLA Entity достаточен. Для всего, что пересекает SITE1↔AX boundary или меняет архитектурный invariant — обязательно RFC+ADR.

Архитектурная декомпозиция (§2) ≠ planning protocol. v3.0 фиксирует **mandatory pipeline для любого нового модуля или нетривиального изменения**.

| # | Слой планирования | Что фиксируется | Артефакт | Хранится в |
|---|-------------------|-----------------|----------|------------|
| P1 | Strategic | зачем фича существует, бизнес-причина, constraints | RFC | `docs/rfc/RFC-NNN-<slug>.md` |
| P2 | Architectural | bounded context, invariants, отвергнутые альтернативы | ADR | `docs/adr/ADR-NNN-<slug>.md` |
| P3 | Execution | tasks, migrations, rollout | Implementation Plan | `docs/plans/PLAN-NNN-<slug>.md` |
| P4 | Verification | success criteria, perf budget, tenant-isolation tests | Validation Spec | `docs/validations/VAL-NNN-<slug>.md` |

**Жёсткие правила:**

- **Ничего не попадает в `crates/` без P4 sign-off.** PR без ссылок на RFC/ADR/Plan/Validation блокируется CI (`cargo xtask check-planning-refs` — §2.6).
- Изменение, затрагивающее **одну функцию** в существующем модуле без новых invariants — exempt (точечный fix не требует RFC).
- Новый трейт в `application/ports/` ⇒ обязательно ADR.
- Новая SQL-миграция ⇒ обязательно Validation Spec с tenant-isolation тестами.
- Новый внешний сервис (S3, email provider, OAuth) ⇒ обязательно RFC + ADR.

**Шаблон RFC (минимально достаточный):**

```markdown
# RFC-NNN <Title>

## Business reason
- что меняется в продукте, метрика успеха

## Constraints
- технические ограничения, deadline, регуляторные

## Out of scope
- что НЕ делается этим RFC
```

**Шаблон ADR:**

```markdown
# ADR-NNN <Decision Title>

## Context
- что вынудило принять решение

## Decision
- одно предложение: выбран подход X

## Alternatives considered
- Y — отвергнут потому что...
- Z — отвергнут потому что...

## Consequences
- что станет проще
- что станет сложнее
- какие новые риски

## Reversal cost
- low / medium / high — насколько дорого откатить
```

**Шаблон Implementation Plan:** нумерованный список шагов, каждый — atomic commit-cell, с указанием spine/non-spine файлов (см. CLAUDE.md §M).

**Шаблон Validation Spec:**

```markdown
# VAL-NNN <Module>

## Success criteria (measurable)
- p95 < X ms
- 0 cross-tenant reads in fuzz test
- N+1 absent (verified by query-log assertion)

## Tests required
- [ ] domain unit (invariants)
- [ ] application unit (use cases, mocked ports)
- [ ] infrastructure integration (real Postgres + RLS)
- [ ] HTTP e2e (oneshot)
- [ ] tenant-isolation fuzz (§17)
- [ ] performance benchmark (oha) vs baseline

## Observability
- new tracing spans
- new metrics
- new Sentry tags
```

**Связь с CLAUDE.md §M режимами:**

- **MANUAL** — все 4 артефакта пишутся пользователем или AI с подтверждением каждого.
- **SEMIAUTO** — AI пишет P1–P4 как часть Level 2 manifest'а; пользователь даёт одно одобрение на пачку.
- **AVTONOM** — AI пишет P1–P4 в session-plan, далее работает; решения по выбору документируются в `SESSION_LOG.md`.

**Связь с `barbie/ENTITY.md §11 Delivery Standard`:** для любого PR'а, изменяющего код/инфраструктуру, в commit message / PR description выдаётся:

- **PLAN** — ссылка на P3 Implementation Plan
- **CHANGES** — список затронутых файлов (spine / non-spine по CLAUDE.md §M)
- **RISKS** — извлечение из P2 ADR "Consequences" + актуальные риски из §14
- **VALIDATION** — статус критериев из P4 Validation Spec (что зелёное / что осталось / что заведомо unverified)
- **NEXT** — следующий step из P3 или новый RFC

Это не отдельный шаг — это **извлечение уже написанных артефактов P1–P4** в формат delivery report. Если артефактов нет — нет и delivery report'а; PR блокируется.

---

## 2.6 Architecture Fitness Functions

Архитектурные правила должны быть **исполняемы**, не декларативны. v3.0 фиксирует обязательный набор автоматизированных проверок.

**Crate `xtask/` (custom build automation):**

```bash
cargo xtask architecture-check    # cross-crate deps правила
cargo xtask check-planning-refs   # PR содержит ссылки на RFC/ADR/Plan/Val
cargo xtask alloc-budget          # dhat-rs snapshot vs budget (§11.5)
cargo xtask query-budget          # EXPLAIN ANALYZE regression (§11.6)
cargo xtask magic-check           # ban-list проверка (§2.7)
```

**Внешние инструменты (CI-required):**

| Инструмент | Назначение | Когда запускается |
|-----------|-----------|-------------------|
| `cargo-deny` | license allowlist, ban list, advisory deny | каждый PR |
| `cargo-udeps` | неиспользуемые dependencies | nightly CI |
| `cargo-audit` | RustSec CVE database | nightly CI + перед каждым release |
| `cargo-vet` | trust policy для зависимостей | каждый PR (для diff) |
| `clippy --deny warnings` | стандартные lints + project-local | каждый PR |
| `rustfmt --check` | форматирование | каждый PR |

**Конкретные правила `cargo xtask architecture-check`:**

```
- domain/ может зависеть только из: serde, uuid, chrono, garde, thiserror
- domain/ не может зависеть от: tokio, sqlx, axum, reqwest, sentry, tracing
- presentation/ не может прямо зависеть от: infrastructure/
- infrastructure/ публичные API не возвращают: sqlx::Row, sqlx::PgRow, sqlx::Pool
- forbidden crate-imports (ban-list): openssl-sys (используем rustls), unicase, time<0.3
- forbidden symbols: tokio::spawn (вне crates/runtime/supervisor — §4.8)
- forbidden patterns: SELECT * (regex over migrations/ и query!), OFFSET (вне admin tools)
```

**`cargo-deny.toml` baseline:**

```toml
[licenses]
allow = ["MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception",
         "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unicode-DFS-2016",
         "MPL-2.0"]
copyleft = "deny"

[bans]
multiple-versions = "warn"
deny = [
    { name = "openssl" },
    { name = "openssl-sys" },
    { name = "native-tls" },
]

[advisories]
yanked = "deny"
vulnerability = "deny"
unmaintained = "warn"
```

**CI gate:** PR не мерджится, если упал хоть один fitness-check. Override возможен только через ADR с явной фиксацией исключения и review двух человек.

---

## 2.7 NO MAGIC POLICY

**Origin:** этот раздел — операционализация `barbie/ENTITY.md §11 Philosophy` для Rust-контекста. Родительская формулировка: *"Prefers boring, deterministic, reversible, observable, and maintainable systems. Rejects magic, hidden state, premature complexity, and trend-chasing."* §2.7 фиксирует, **что именно** в Rust считается "магией" и **как** это запрещено технически.

Самые поддерживаемые Rust-системы — **boring, explicit, predictable**. Не "clever". v3.0 фиксирует явный запрет на классы паттернов, которые ломают читаемость и debug'абельность.

**Запрещено:**

- macro-heavy DSL'и собственного производства (`my_handler! { ... }` стиль)
- proc-macro со скрытым побочным эффектом (модифицируют global state, регистрируют handlers неявно)
- runtime-reflection-подобные паттерны (`AnyMap`, `TypeId`-based dispatch вне явно ограниченных мест)
- неявные globals (`lazy_static`, `once_cell` для mutable state без явного reasoning в ADR)
- скрытый `tokio::spawn` вне TaskSupervisor (§4.8)
- скрытый IO в `domain/` или `application/` (только через port traits)
- кодогенерация без commit'нутого артефакта (если `build.rs` генерирует код — generated файл commit'ится в git и review'ится в diff)
- "smart" pointer chains (`Arc<RwLock<HashMap<K, Arc<Mutex<V>>>>>`) — это сигнал, что shared mutable state не продуман

**Разрешено (whitelist):**

- `#[derive(Serialize, Deserialize, Debug, Clone)]` — стандартный serde
- `#[derive(garde::Validate)]` — validation rules инспектабельны
- `sqlx::query!`, `sqlx::query_as!` — output виден в `.sqlx/` metadata, commit'ится в git
- `#[derive(thiserror::Error)]` — error variants явны
- `#[derive(utoipa::ToSchema)]` — OpenAPI generated artifact commit'ится
- axum extractors (стандартные) — поведение документировано

**Enforcement:** `cargo xtask magic-check` сканирует:
- `macro_rules!` определения вне `crates/common/macros/` (а там — review каждой строки)
- `proc-macro` crate dependencies вне whitelist
- `Arc<RwLock<...>>` / `Arc<Mutex<...>>` patterns с предупреждением (требует комментарий с обоснованием на соседней строке)

**Принцип:** код, который понятен junior'у с базовым Rust через 18 месяцев, побеждает код, который "elegant" для senior'а сегодня.

---

## 2.8 Complexity Governance

Без явных бюджетов сложности код-база растёт неконтролируемо.

**Бюджеты (enforced clippy + xtask):**

| Метрика | Лимит | Где enforce |
|---------|-------|-------------|
| function length | < 80 LOC | clippy `too_many_lines` |
| function args | ≤ 7 | clippy `too_many_arguments` |
| trait methods | ≤ 7 | xtask custom check |
| enum variants | ≤ 12 | xtask custom check (исключения с ADR) |
| generic depth | ≤ 3 уровня | clippy `type_complexity` |
| async nesting | ≤ 4 (await chains) | xtask custom check |
| cyclomatic complexity | ≤ 15 | clippy `cognitive_complexity` |
| crate compile time (cold) | < 60 s per crate | xtask alert |
| workspace compile time (clean release) | < 8 min | xtask alert |

**Architecture review (отдельный ADR) обязателен при:**

- добавлении нового tokio Runtime
- добавлении нового persistence layer (помимо Postgres)
- добавлении новой очереди (помимо pgmq)
- добавлении нового сетевого протокола (gRPC, GraphQL, WebSocket вне ограниченных мест)
- добавлении новой async boundary (новый Server Function namespace, новый extractor контракт)
- добавлении proc-macro crate в зависимости
- введении нового middleware в hot-path
- добавлении нового внешнего сервиса (S3, email, OAuth, payment)

**Regression alarm:** xtask еженедельно сравнивает compile-time и binary size с baseline (commit'нутый файл `docs/baselines/compile-time.json`); деградация > 15% — issue assigned автоматически.

---

## 3. Multi-Tenancy — 5-уровневая защита

| # | Уровень | Реализация |
|---|---------|------------|
| 1 | DNS + reverse-proxy | Caddy 2 on-demand TLS, `ask`-endpoint в `application/` для validation custom-доменов |
| 2 | Axum middleware | `TenantResolver` extension: Host → `TenantId`, кэш через `moka` (LRU + TTL 5 мин) |
| 3 | Request context | `axum::extract::Extension<TenantContext>` + newtype `TenantId(Uuid)` — нельзя случайно подставить `UserId` |
| 4 | Postgres RLS | `SET LOCAL app.current_tenant_id = $1` в transaction-scope; роль `app_tenant` БЕЗ `BYPASSRLS` |
| 5 | DB schema | `tenant_id` NOT NULL + composite indexes ведут с `tenant_id` + partial unique |

**Критическое уточнение:**

> RLS с `SET LOCAL` **обязательно** требует PgBouncer в `transaction` pool mode. В `session` mode `SET LOCAL` держится между запросами одного коннекта и пул сжирается; в `statement` mode `SET LOCAL` вообще не работает. SQLx `PgPoolOptions` подключается ЗА PgBouncer'ом, не напрямую к Postgres.

Без этого нюанса вся 5-уровневая защита разваливается под нагрузкой.

**Newtype-защита против "забыли скоупнуть" (уникальная возможность Rust):**

```rust
pub struct TenantId(uuid::Uuid);
pub struct TenantScoped<Q>(pub Q, pub TenantId);

// Каждый repository принимает TenantScoped<Query>, не raw Query.
// `find_appointments(query)` без tenant не компилируется.
pub trait AppointmentRepository {
    async fn find(&self, q: TenantScoped<AppointmentQuery>) -> Result<Vec<Appointment>>;
}
```

Это **compile-time доказательство** tenant-scoping. В TypeScript-стеке возможно только runtime через guard.

---

## 4. Стек

### 4.1 Runtime и framework

| Слой | Технология | Версия | Обоснование |
|------|-----------|--------|-------------|
| Runtime | Rust + tokio | stable 1.84+ / tokio 1.43+ | актуальный stable, без nightly |
| HTTP server | Axum | 0.7.x | Tower-ecosystem, минимальные allocations, отличная extractor-модель |
| Tower middleware | tower-http | 0.6.x | CORS, compression, trace, timeout — battle-tested |
| Frontend (один из адаптеров) | Leptos | 0.7.x | SSR streaming, fine-grained reactivity, server functions; **через Presentation Contract — §4.9** |
| Build | `cargo-leptos` | latest | Hot-reload, single-binary SSR, CSS-bundling |
| Task supervisor | custom `crates/runtime/` | — | wrapper над tokio для §4.8 concurrency policy |

### 4.2 Database и persistence

| Слой | Технология | Обоснование |
|------|-----------|-------------|
| DB | PostgreSQL 16 | как в SITE1; RLS, partial indexes, JSONB |
| Driver | SQLx 0.8.x | compile-time SQL validation через `cargo sqlx prepare` |
| Migrations | `sqlx::migrate!` | embedded в bin, идемпотентно, no external CLI на проде |
| Pooling | PgBouncer (внешний) | **transaction mode** — load-bearing с RLS |
| ORM-helper | НЕТ (raw SQLx) | SeaORM/Diesel избыточны; SQLx query macros покрывают 95% |

**Намеренный отказ от ORM:** SeaORM ломает доменную чистоту L3 (active-record). Diesel — slower compile times, weaker async story. SQLx с `query_as!` достаточен.

### 4.3 Validation, errors, observability

| Задача | Crate | Почему |
|--------|-------|--------|
| Validation | `garde` 0.20+ | derive-friendly, кастомные правила, integration с serde |
| Library errors | `thiserror` | для library-крейтов |
| App-level errors | `eyre` (или `color-eyre`) | для bin-крейтов; читаемые stacktraces |
| API errors | custom `AppError` enum + `IntoResponse` | централизованный mapping к HTTP |
| Tracing | `tracing` + `tracing-subscriber` | structured, span-based, native async |
| OTEL export | `tracing-opentelemetry` + `opentelemetry-otlp` | стандарт; экспорт в Tempo / Jaeger / Honeycomb |
| Error tracking | `sentry-tracing` | автоматический breadcrumb из spans |
| Metrics | `metrics` + `metrics-exporter-prometheus` | scrape с `/metrics`, Grafana dashboards |
| Allocation profiling | `dhat-rs` | в bench-bin + опционально в production sample-mode (§11.5) |
| Secrets | `secrecy` | `SecretString` тип, явный `expose_secret()` (§18) |

### 4.4 Auth, session, RBAC

| Задача | Crate | Обоснование |
|--------|-------|-------------|
| Password hashing | `argon2` | **не bcrypt** — argon2 рекомендован OWASP; в Rust нет legacy-инерции bcrypt |
| JWT | `jsonwebtoken` | проверенный, ничего экзотического |
| Sessions | `tower-sessions` + `tower-sessions-sqlx-store` | Postgres-backed, no Redis dependency |
| RBAC | custom (axum extractor + permissions enum) | универсальных production-ready Rust RBAC-крейтов нет |
| CSRF | tower-sessions + double-submit cookie | стандарт |
| TLS | rustls (через axum-server) | **не openssl** — supply-chain hygiene (§19) |

### 4.5 Background jobs

| Задача | Crate | Обоснование |
|--------|-------|-------------|
| Queue | **`pgmq`** (Postgres Message Queue extension + Rust client) | аналог `pg-boss` из TS-плана; "Redis не для очередей" |
| Альтернатива (Redis) | `apalis` | только если pgmq упрётся в throughput — для CRM не случится |

Подробнее по runtime изоляции — §10.5.

### 4.6 Storage, email, прочее

| Задача | Crate |
|--------|-------|
| S3 client | `aws-sdk-s3` (официальный AWS SDK для Rust) |
| Presigned URLs | `aws-sdk-s3` presigning |
| File type sniffing | `infer` (security — не доверять content-type от клиента, §18) |
| Image processing | `image` + `webp` (re-encode, never pass-through, §18) |
| Email | `lettre` (SMTP) или HTTP-adapter к Resend |
| Email templates | `askama` (Jinja-like, compile-time validated) |
| OpenAPI | `utoipa` + `utoipa-axum` + `utoipa-swagger-ui` |
| HTTP tests | `axum::Router::oneshot` (unit), `reqwest` (integration) |
| DB tests | `testcontainers-rs` + ephemeral Postgres |
| Browser tests | **`fantoccini`** (WebDriver) или внешний JS-Playwright |

**Поправка v1.0:** v1.0 указал "Playwright (Rust bindings)" — production-ready Rust-binding'ов Playwright не существует. Реалистично: `fantoccini` через WebDriver или вынесенный JS-Playwright suite.

### 4.7 Frontend dependencies (Leptos adapter — один из §4.9)

| Задача | Реализация |
|--------|------------|
| UI primitives | свои Leptos-компоненты под `dashboard-2077` эстетику (RF Rufo + scoop + ambient) |
| Tailwind | через `cargo-leptos` style processing |
| Icons | SVG inline или `iconify-icon` через web-component |
| Forms | `leptos-router` form actions + `garde` validation |
| Charts | `plotters` (SSR) или Chart.js через JS interop |
| Client-side state | Leptos signals (`RwSignal`, `Memo`, `Resource`) |

---

## 4.8 Concurrency Model

Большинство Rust-backend'ов деградируют из-за отсутствия явной concurrency policy: orphan tasks, hidden leaks, cancellation bugs. v3.0 фиксирует протокол.

**Запреты:**

```rust
// ЗАПРЕЩЕНО вне crates/runtime/supervisor/
tokio::spawn(async move { ... });

// ЗАПРЕЩЕНО вообще
std::thread::spawn(...);  // используй tokio::task::spawn_blocking
```

**Единственный разрешённый entry-point:**

```rust
// crates/runtime/src/supervisor.rs
pub struct TaskSupervisor {
    runtime: TaskCategory,        // Http | Queue | Image | Report
    cancel: CancellationToken,
    handle: TaskTracker,
}

impl TaskSupervisor {
    pub fn spawn<F>(&self, name: &'static str, fut: F) -> TaskHandle
    where F: Future<Output = ()> + Send + 'static
    { /* ... */ }

    pub async fn drain(self, timeout: Duration) -> Result<(), DrainError>
    { /* graceful shutdown — wait or kill */ }
}
```

**Категории runtime — failure isolation см. §10.5:**

| Категория | Runtime | Backpressure |
|-----------|---------|--------------|
| HTTP request handling | tokio runtime A (worker_threads = cpus) | request timeout 30s, body size limit |
| Queue workers | tokio runtime B (worker_threads = cpus/2) | bounded channel cap 1024 на consumer |
| Image processing | tokio runtime C + `spawn_blocking` pool (max 8) | bounded queue cap 32, дроп с retry на overflow |
| Report generation | tokio runtime D (worker_threads = 2, low priority) | bounded queue cap 16 |
| Email send | shared queue runtime, бекофф 30s/5min/1h | bounded channel cap 512 |

**Cancellation:**

- Все long-running операции принимают `CancellationToken` через context.
- Shutdown сигнал (SIGTERM) ⇒ `cancel.cancel()` ⇒ supervisor останавливает новые задачи и ждёт `drain(30s)`.
- Превышение drain timeout ⇒ forced abort с логированием orphan'ов в Sentry.

**Backpressure:**

- Бесконечные `tokio::sync::mpsc::unbounded_channel` — запрещены (xtask check).
- Любой channel — `bounded(N)` с явным N в комментарии или const.
- Overflow handling декларируется в каждом call-site: `drop_oldest` / `drop_newest` / `block` / `return_error`.

**CPU-heavy задачи:**

- ImageMagick-эквиваленты, PDF rendering, ML inference — только через `spawn_blocking` или отдельный subprocess.
- Не блокировать tokio worker > 100µs.

---

## 4.9 Presentation Contract — Leptos как replaceable adapter

Leptos — НЕ production-proven на уровне Next.js / React / SvelteKit. Если экосистема Leptos сломается или замедлится в развитии, AX не должен потерять frontend полностью.

**Решение:** `presentation/` определяет **contract**, а не реализацию. Адаптеры подключаются как отдельные crates.

```
crates/
└── presentation/
    ├── contract/        # traits: PageRenderer, FormBinder, RouteResolver
    ├── ssr-leptos/      # реализация на Leptos 0.7
    ├── spa-api/         # JSON API surface (для будущего external SPA)
    ├── admin-htmx/      # отдельный admin UI (если Leptos упадёт — backup)
    └── openapi/         # utoipa-axum, генерирует OpenAPI из application/ports/
```

**Правила:**

- Бизнес-логика **никогда** не в `ssr-leptos/` — только rendering + form-binding.
- Leptos Server Functions — тонкий wrapper, делегирующий в `application/` command/query.
- Если Leptos 0.8 сломает API ⇒ только `ssr-leptos/` переписывается; `application/`, `domain/`, `infrastructure/` нетронуты.
- `spa-api/` сразу присутствует (даже если не используется UI'ем) — он гарантирует, что system не зависит от SSR семантики.
- Замена на Dioxus/Yew/htmx — это новый crate-адаптер, не workspace-wide migration.

**Quarterly review** включает оценку Leptos health: коммиты в upstream, breaking changes в roadmap, размер community. Если health деградирует — активация `admin-htmx/` как primary path.

---

## 5. Структура проекта (Cargo workspace)

```
barbie/ax/
├── Cargo.toml                      # workspace root
├── Cargo.lock
├── rust-toolchain.toml             # pin stable-1.84
├── deny.toml                       # cargo-deny config (§2.6)
├── .sqlx/                          # offline query metadata (in git)
├── migrations/                     # SQL миграции (sqlx::migrate!)
├── xtask/                          # custom build automation (§2.6)
│   └── src/
│       ├── architecture_check.rs
│       ├── magic_check.rs
│       ├── alloc_budget.rs
│       └── query_budget.rs
├── crates/
│   ├── domain/                     # L3 — pure structs
│   │   ├── tenant/
│   │   ├── salon/
│   │   ├── appointment/
│   │   └── ...
│   ├── application/                # L2 — use cases, port traits
│   │   ├── ports/                  # traits для repos, queues, storage
│   │   ├── commands/               # write-side handlers
│   │   ├── queries/                # read-side handlers
│   │   └── services/               # cross-cutting (audit, notifications)
│   ├── infrastructure/             # L4 — adapter impls
│   │   ├── persistence/            # SQLx repos
│   │   ├── storage/                # S3 adapter
│   │   ├── queue/                  # pgmq adapter
│   │   └── email/                  # lettre / Resend
│   ├── common/                     # cross-crate types: TenantId, AppError, Result
│   ├── tenant/                     # tenant middleware + RLS SET LOCAL helper
│   ├── runtime/                    # TaskSupervisor (§4.8), runtime categories
│   ├── presentation/               # contract + адаптеры (§4.9)
│   │   ├── contract/
│   │   ├── ssr-leptos/
│   │   ├── spa-api/
│   │   └── openapi/
│   └── contracts/                  # SITE1↔AX interop contracts (§12.5)
├── apps/
│   └── server/                     # Axum entrypoint, binds Leptos SSR + supervisors
│       ├── src/main.rs
│       └── src/router.rs
├── docs/
│   ├── rfc/                        # P1 артефакты (§2.5)
│   ├── adr/                        # P2 артефакты
│   ├── plans/                      # P3 артефакты
│   ├── validations/                # P4 артефакты
│   ├── perf/explain/               # EXPLAIN ANALYZE snapshots (§11.6)
│   ├── baselines/                  # compile-time, binary size baselines (§2.8)
│   ├── releases/                   # rollback scripts (§21)
│   ├── security/                   # threat model, audits (§18)
│   └── interop/                    # SITE1↔AX contracts (§12.5)
├── docker-compose.dev.yml          # mirror SITE1 layout (другие порты)
├── .env.example
└── ENTITY.md                       # этот файл
```

**Vertical slices** внутри `domain/`, `application/`, `infrastructure/` — по фичам: `tenant/`, `salon/`, `appointment/`, `cms/`, `media/`, `chat/`, `billing/`.

---

## 6. Tenant Context — Rust-specific реализация

```rust
// crates/common/src/tenant.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash,
         serde::Serialize, serde::Deserialize)]
pub struct TenantId(pub uuid::Uuid);

#[derive(Clone, Copy, Debug)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub user_id: Option<UserId>,
    pub request_id: RequestId,  // ULID
}
// Copy by design — clippy предупредит про .clone() (§11.5 alloc budget).

// crates/tenant/src/middleware.rs
pub async fn tenant_resolver(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let host = req.headers().get(HOST).ok_or(AppError::BadRequest)?;
    let tenant = state.tenant_cache.resolve(host).await?;
    let ctx = TenantContext {
        tenant_id: tenant.id,
        user_id: extract_user_id(&req),
        request_id: RequestId::new(),
    };
    req.extensions_mut().insert(ctx);
    Ok(next.run(req).await)
}

// crates/infrastructure/src/persistence/transaction.rs
pub async fn with_tenant<T, F>(
    pool: &PgPool,
    ctx: &TenantContext,
    f: F,
) -> Result<T, AppError>
where
    F: for<'a> FnOnce(&'a mut Transaction<'_, Postgres>)
        -> BoxFuture<'a, Result<T, AppError>>,
{
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL app.current_tenant_id = $1")
        .bind(ctx.tenant_id.0)
        .execute(&mut *tx).await?;
    let result = f(&mut tx).await?;
    tx.commit().await?;
    Ok(result)
}
```

**`with_tenant` — единственный entry-point в БД** для tenant-scoped операций. SQL без обёртки технически возможен, но запрещается:
- `cargo xtask architecture-check` находит `sqlx::query` вне `with_tenant` scope в `infrastructure/` (heuristic + allowlist)
- code review checklist
- ENTITY §2 как в SITE1

---

## 7. Error Handling

```rust
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("not found: {0}")]      NotFound(String),
    #[error("validation: {0}")]     Validation(#[from] garde::Report),
    #[error("unauthorized")]        Unauthorized,
    #[error("forbidden: {0}")]      Forbidden(String),
    #[error("tenant mismatch")]     TenantMismatch,
    #[error("conflict: {0}")]       Conflict(String),
    #[error("bad request: {0}")]    BadRequest(String),
    #[error("rate limited")]        RateLimited,
    #[error("internal: {0}")]       Internal(#[from] eyre::Report),
    #[error("database")]            Database(#[from] sqlx::Error),
}
```

`IntoResponse for AppError` мэппит варианты в HTTP-статусы. Каждый вариант, помеченный как **security event** (`TenantMismatch`, `Unauthorized`, `Forbidden`), захватывает Sentry-событие с tenant_id/user_id/request_id в scope. `Internal` / `Database` логируются на `error` level с полным stacktrace через `tracing`.

**Tenant-mismatch** — отдельный вариант, потому что это **security event**, не обычная 403. Высокий severity в Sentry, отдельный алерт.

---

## 8. Validation

`garde` на L2/L3, не на presentation:

```rust
#[derive(Debug, garde::Validate)]
pub struct CreateAppointmentInput {
    #[garde(custom(positive_duration))]
    pub duration_min: u32,
    #[garde(custom(future_starts_at))]
    pub starts_at: DateTime<Utc>,
    #[garde(range(min = 0))]
    pub price_kopecks: i64,
}
```

Server Functions Leptos валидируют **дважды**: клиент (мгновенный фидбек), сервер (security boundary). Один `derive(garde::Validate)` обслуживает оба прохода.

---

## 9. Background Jobs

**`pgmq` (Postgres Message Queue)** — extension Postgres + Rust-клиент.

Преимущества:
- **транзакционная enqueue** в том же `BEGIN`, что бизнес-операция: создать `appointment` и enqueue `email-confirmation` атомарно
- 0 новых инфраструктурных зависимостей
- visible в `psql` для дебаггинга
- delayed jobs, retries, dead-letter queue — встроенные

Воркер запускается **через TaskSupervisor категории Queue** (§4.8). В том же бинарнике или отдельном процессе — оба варианта валидны, но изоляция runtime'ов (§10.5) обязательна.

**Redis не нужен для очередей.** Если позже понадобится для rate-limit / session-cache в multi-instance setup — **Dragonfly** как drop-in (Redis API, в 3-5× меньше RAM). Решение требует ADR (§2.5).

---

## 10. Observability

| Сигнал | Стек |
|--------|------|
| Logs | `tracing` + `tracing-subscriber` → JSON в stdout → Loki / Better Stack |
| Traces | `tracing-opentelemetry` → OTLP → Tempo / Honeycomb |
| Metrics | `metrics` + Prometheus exporter on `/metrics` |
| Errors | `sentry` + `sentry-tracing` — auto breadcrumbs из spans |
| Health | `/health` (process), `/health/ready` (DB + S3 + queue probes) |
| Allocations | `dhat-rs` в bench + опциональный production sampler (§11.5) |

**Каждый request** получает `request_id` (ULID) + `tenant_id` в trace context. Любой error в Sentry приходит с обоими. Это превращает "10000 одинаковых 500 Internal" в diagnosable группы "tenant X имеет проблему в endpoint Y".

---

## 10.5 Failure Domain Isolation

Один tokio runtime на всё приложение — production-критическая ошибка: heavy reports / image processing убивают latency API.

**Изоляция:**

| Domain | Изоляция | Crash impact |
|--------|----------|--------------|
| HTTP API | tokio runtime A (`http_rt`) | падение = API down, остальное живёт и осушает очереди |
| Queue workers | tokio runtime B (`queue_rt`) | падение = jobs паузятся, API отвечает (с graceful degradation в Sentry events) |
| Image processing | tokio runtime C + `spawn_blocking` pool | падение пула = uploads fail-fast, остальное живёт |
| Report generation | tokio runtime D, low priority | падение = отчёты в DLQ, API/queue/uploads нетронуты |
| Email send | внутри queue_rt, bounded channel | падение = ретраи через pgmq |

**Бинарь:** `apps/server` запускает все runtime'ы под общим `TaskSupervisor`. Shutdown drain'ит их по приоритету: API last (после queue, image, report).

**Альтернатива (для очень большой нагрузки):** разнести image processing в **отдельный subprocess** под `wasmtime` sandbox. Triggered ADR — §2.8 architecture review при добавлении нового process boundary.

**Catastrophic failure handling:**

- panic в request handler ⇒ tower-http catch-panic middleware ⇒ 500 + Sentry event, runtime жив
- panic в queue worker ⇒ supervisor рестартит worker с exponential backoff
- OOM ⇒ systemd рестартит бинарь; alert в Sentry "instance restarted by OOM-killer"

---

## 11. Performance Budget — с методологией

| Метрика | Цель | Метод | Когда валидируем |
|---------|------|-------|------------------|
| p95 latency CRUD | < 80 ms | `oha` benchmark, 200 RPS, 60s warm-up, реальный Postgres + 100k seeded rows | каждый PR через CI smoke |
| p99 latency CRUD | < 200 ms | то же | то же |
| Throughput | > 5000 req/s на core | `oha -c 100 -z 60s` | при изменениях hot-path |
| RSS per instance | < 256 MB / 1000 sessions | `procfs` опрос каждые 5s в нагрузке | weekly |
| Cold start | **< 200 ms** (не 80 — это нереально) | `time ./target/release/server` до первого `READY` log | при каждом release |
| DB connection acquire | < 5 ms p95 | `tracing` span `db.acquire` | continuous |

**Поправка v1.0:** "cold start < 80 ms" — это для tiny CLI-tool, не для full-stack Axum + Leptos SSR + SQLx pool init + pgmq + Sentry + OTEL warm-up. Реалистично 150–200 ms.

**Не бенчмарк vs бенчмарк:** числа сами по себе не решают. Сравнивать **только** при равной нагрузке, одинаковом seed-data, одинаковом наборе middleware и одинаковом hardware. Любой "Rust в 10× быстрее Node" без этих условий — junk benchmark.

---

## 11.5 Allocation Budget

Rust позволяет контролировать аллокации — но только если это **бюджетируется и измеряется**. Иначе async код легко расходует тысячи аллокаций на запрос.

**Инструментарий:**

- `dhat-rs` — heap profiling в bench-bin'ах, перед каждым release
- `jemallocator` + `jemalloc-ctl` — production allocator с stats endpoint
- `tikv-jemalloc-ctl` для introspection

**Бюджет (enforced в xtask + CI smoke):**

| Метрика | Бюджет | Endpoint класс |
|---------|--------|----------------|
| heap allocations / request | < 500 | CRUD read |
| heap allocations / request | < 1500 | CRUD write |
| total bytes / request | < 64 KB | CRUD read |
| async tasks / request | < 20 | все |
| Arc clone depth | bounded (clippy custom lint) | все |
| large allocs (> 16 KB) | traced + tagged | все |
| `TenantContext.clone()` calls | 0 (Copy, не Clone) | все |

**Hot-path политика:**

- избегать `Vec<String>` где возможен `Vec<&str>` или `SmallVec`
- избегать `format!` в hot-path — использовать `write!` в pre-allocated buffer
- избегать `Box<dyn Trait>` где возможен generic
- `Arc` — только когда shared ownership реально нужен (не "потому что так компилируется")

**Regression detection:** `cargo xtask alloc-budget` в nightly CI сравнивает dhat-snapshot с baseline (`docs/baselines/alloc-{endpoint}.json`); deviation > 20% — issue assigned.

---

## 11.6 Query Budget Enforcement

SQLx compile-time validation ≠ safe queries. Compile-time проверяет **синтаксис и типы**, не **сложность и план**.

**Бюджет (enforced):**

| Метрика | Бюджет | Enforcement |
|---------|--------|-------------|
| joins per query | ≤ 5 | xtask AST scan |
| `SELECT *` | запрещено | xtask regex check |
| `OFFSET` pagination | запрещено вне admin tools | xtask regex check (allow-list) |
| N+1 patterns | запрещено | dataloader pattern + integration test с query-count assertion |
| sequential scan на > 1000 rows | запрещено | EXPLAIN snapshot regression |
| response payload | bounded (max 1 MB JSON) | tower-http response limit |

**Pagination:** только **keyset** (`WHERE (created_at, id) < ($cursor_ts, $cursor_id) ORDER BY created_at DESC, id DESC LIMIT N`). OFFSET — O(N) на больших таблицах, прогнозируемый источник tenant-noisy-neighbor проблем.

**EXPLAIN ANALYZE snapshots:**

- для каждого endpoint hot-path коммитится snapshot `docs/perf/explain/{endpoint}.txt` с актуальным планом
- CI запускает EXPLAIN на seed-данных и сравнивает estimated cost; deviation > 30% — fail
- new index ⇒ regen всех затронутых snapshots

**Slow-query regression test** в integration suite: запрос с 100k seed rows должен укладываться в budget.

---

## 12. Миграционный путь (если когда-либо)

**При каких условиях AX мог бы заместить SITE1:**

1. SITE1 успешно прошёл production-pilot (≥ 6 месяцев на VPS с ≥ 5 живыми тенантами без критических инцидентов изоляции).
2. Бенчмарк AX-прототипа vs SITE1 на **feature-parity срезе** (1 модуль, например `appointments`) показывает **≥ 3× прирост** по throughput **или** **≥ 50% снижение** RSS под равной нагрузкой. Меньшие выигрыши не оправдывают cost'а.
3. В команде ≥ 1 senior Rust-разработчик с production-Axum опытом.
4. Существует бизнес-кейс, где Node-стек упёрся (например, > 100 тенантов с тяжёлой отчётностью одновременно, и SITE1 не вытягивает даже с PgBouncer + read replicas).

**Если условия выполнены — миграция идёт vertical-slice-by-slice:**

- AX-сервер развёртывается **параллельно** SITE1.
- Caddy роутит выбранные tenant-slug'и на AX, остальные на SITE1.
- По одному модулю переносится (`appointments` → 2 недели на тест-тенанте → 2 недели в production на 1 живом тенанте → roll-out на остальные).
- SITE1 остаётся canonical write-path до полного переноса модуля.
- Database **общая** для обоих: и AX, и SITE1 ходят в один Postgres через PgBouncer. RLS работает для обоих.

**Если ни одно условие не выполнено — AX остаётся документом.**

---

## 12.5 Interop Boundary — SITE1 ↔ AX

Параллельная работа двух стеков на одной БД без явного контракта = shared-DB hell с поведенческой дивергенцией. v3.0 фиксирует протокол.

**Ownership Table (кто canonical для какой сущности):**

| Сущность | Canonical | Reader | Period |
|----------|-----------|--------|--------|
| `tenants` | SITE1 | оба | вся миграция |
| `users` | SITE1 | оба | вся миграция |
| `auth_sessions` | SITE1 | SITE1 (AX переиспользует JWT) | всегда |
| `billing_*` | SITE1 | SITE1 | всегда |
| `appointments` | AX (после pilot) | оба | после P4 sign-off |
| `cms_*` | SITE1 → AX (поэтапно) | оба | по модулям |

**Артефакты в `docs/interop/`:**

- `OWNERSHIP.md` — таблица выше + история изменений
- `CONTRACTS/{entity}.md` — для каждой shared entity: схема, инварианты, кто пишет, кто читает, версионирование
- `EVENTS/{topic}.md` — pgmq topics, контракт payload'а с schema-version
- `MIGRATIONS/POLICY.md` — кто владеет схемой какой таблицы; expand-contract контракт (§21)
- `ROLLBACK/{module}.md` — как откатить модуль с AX на SITE1 за < 5 мин

**Правила:**

- AX не пишет в SITE1-owned таблицы напрямую — только через SITE1 HTTP API или published events.
- SITE1 не пишет в AX-owned таблицы напрямую — симметрично.
- Shared таблицы (например, `tenants`): схемой владеет SITE1, миграции применяются SITE1; AX adapt'ится через read-only views в `infrastructure/persistence/views/`.
- Любая cross-stack миграция ⇒ ADR (§2.5) + INTEROP review двумя людьми (по одному от стека, если команда вырастет).

**Event contracts:**

```
pgmq topic: tenant.user.registered
payload schema: docs/interop/EVENTS/tenant.user.registered.v1.json
producer: SITE1
consumers: AX (welcome email pipeline), SITE1 (audit log)
versioning: never break v1; introduce v2 as new topic, dual-publish during transition
```

**Anti-corruption layer:** на стороне AX в `crates/contracts/` живут DTO для всех SITE1-imported данных. Mapping `SITE1Dto → AxDomain` явный, не через serde-passthrough. Это защищает доменную модель AX от изменений schema на стороне SITE1.

---

## 12.6 Migration Playbook — Module Vertical Slice с TLA Entity

§12 фиксирует **условия** миграции (когда вообще можно начинать). §12.5 — **контракт shared state**. §12.6 — **операционный playbook**: что делать день за днём, когда §13 Decision Criteria выполнены и выбран пилот-модуль.

**Главный invariant:** один модуль за миграционный цикл. Никаких «давайте сразу appointments + cms». Single concurrent migration — иначе rollback становится комбинаторным.

### 12.6.1 Migration Phases (4 фазы на модуль)

| Фаза | Длительность | Trigger завершения | Если регрессия — действие |
|------|--------------|-------------------|---------------------------|
| **A. Pilot** | 2 недели | 1 тест-тенант на AX без single P1 alert | Rollback через Caddy (§12.6.6), reopen RFC |
| **B. Stage** | 2 недели | 2–3 живых тенанта (low-criticality) на AX, latency parity ± 10% | Pause rollout, не rollback всех, изоляция причины |
| **C. GA** | до 4 недель | Все тенанты на AX для этого модуля, SITE1-write путь disabled | Per-tenant rollback остаётся доступным 30 дней |
| **D. Retire** | 1 неделя | SITE1 модуль удалён, OWNERSHIP.md финализирован | Не rollback'ается — переход в новый ADR |

Между фазами — **mandatory pause** ≥ 48ч. Это не overhead — это окно, в которое регрессии материализуются на хвостовом трафике (отчёты, cron'ы, редкие пользовательские потоки). Без паузы regression-debt накапливается до Phase D.

### 12.6.2 TLA Entity, применённый к миграции одного модуля

**Связь с `barbie/ENTITY.md §9` и AX §2.5:** TLA Entity — это generic protocol для нетривиальных задач **внутри одного стека**. Миграция модуля SITE1→AX — это именно нетривиальная задача внутри AX (с cross-stack контрактом, но реализация — AX). Поэтому каждый миграционный цикл = одна полная итерация TLA Entity Level 1 → Level 2 → Level 3, плюс §2.5 P1–P4 артефакты в `docs/`.

**Маппинг TLA ↔ артефакты:**

| TLA Level | §2.5 артефакт | Что фиксируется для миграции | Sign-off |
|-----------|---------------|------------------------------|----------|
| **L1 Strategic** | RFC | Почему этот модуль, какая бизнес-метрика, success criteria, downtime budget = 0, rollback < 5 мин | RFC review (2 ppl) |
| **L2 Architectural** | ADR + Plan | Port traits, schema delta (expand step), interop contract delta, Caddy routing, ownership transition, testcontainers strategy | ADR review + DBA |
| **L3 Incremental** | Plan execution + Validation | Файл-за-файлом: schema → domain → ports → infrastructure → handlers → tests → caddy. Каждый файл — atomic commit-cell с человеком в петле | Per-file review |

**L1 Strategic — пример checklist'а для модуля:**

```markdown
## RFC-NNN Migration: cms_pages SITE1 → AX

### Business reason
- p95 публичного рендера падает с N ms до M ms при росте до K тенантов
- Сейчас SITE1 next-render узкое горло — Rust SSR + zero-copy template ожидает X-кратный winning

### Success criteria (measurable, MUST pass before Phase B)
- [ ] p95 GET /{slug}/* ≤ 50ms (vs SITE1 baseline 120ms)
- [ ] RSS делёные на pages/sec ≤ 50% от SITE1
- [ ] 1M cross-tenant fuzz attempts = 0 leaks
- [ ] N-1 schema compat verified (SITE1 на старой схеме читает новые row'ы)
- [ ] Rollback drill < 5 мин в stage

### Constraints
- downtime budget = 0 (нет maintenance window)
- public rendering MUST не сломаться даже на 1 секунду
- preview overrides ?td= (см. SITE1 td-overrides.ts) MUST работать

### Out of scope
- ED-editor (остаётся в SITE1 до отдельного цикла)
- WP-importer (SITE1, AX только читает результат)
- /admin/cms UI (Next, не переезжает)
```

**L2 Architectural — что обязательно в ADR:**

```markdown
## ADR-NNN Architecture: cms_pages module port traits + schema delta

### Decision
- AX-side traits в crates/application/ports/cms.rs:
  - trait CmsRepository (read-only initially, append-only events)
  - trait CmsRenderer (Leptos SSR adapter в presentation/)
- SITE1 остаётся canonical writer cms_pages таблицы
- AX читает через read-only views: cms_pages_v_active (RLS-enforced)

### Schema delta (expand step, additive only)
- ADD COLUMN cms_pages.ax_render_cache_key TEXT NULL
- CREATE VIEW cms_pages_v_active AS SELECT ... WHERE status='published'
- CREATE POLICY rls_cms_pages_v_active FOR SELECT ...
- NO DROP, NO RENAME, NO NOT NULL добавлений до Phase D

### Interop contract delta
- SITE1 публикует pgmq: cms.page.published (payload schema v1)
- AX subscribes для cache invalidation (но НЕ для render path — render всегда live)
- OWNERSHIP.md update: cms_pages canonical=SITE1, reader=oba (без изменений на Phase A)

### Caddy routing
- /tenant-pilot-1/* + path /(home|about|...) → AX upstream
- preview overrides ?td=* → пока SITE1 (отдельный ADR на Phase B)
- остальное → SITE1

### Alternatives considered
- AX как canonical writer cms_pages: отвергнуто — слишком большой blast radius на Phase A
- AX без RLS, только app-level filter: отвергнуто — нарушает §3 5-уровневую защиту

### Reversal cost: low
- Caddy whitelist отзывается за 30s
- View и колонка остаются (idempotent)
- pgmq topic не trash'ится
```

**L3 Incremental — порядок файлов (template для одного модуля):**

```
0. RFC + ADR + Plan sign-off → docs/rfc/, docs/adr/, docs/plans/
1. migrations/00NN_<module>_expand.sql                          ← human sign-off
2. crates/domain/src/<module>/mod.rs (value objects)            ← human sign-off
3. crates/domain/src/<module>/aggregate.rs                       ← human sign-off
4. crates/application/src/ports/<module>.rs (traits)             ← human sign-off
5. crates/application/src/use_cases/<module>/*.rs                ← human sign-off (per use case)
6. crates/infrastructure/src/persistence/<module>_repo.rs        ← human sign-off
7. crates/infrastructure/src/views/<module>_views.rs (read-only) ← human sign-off
8. crates/presentation/src/api/<module>_handlers.rs              ← human sign-off
9. crates/presentation/src/leptos/<module>_components.rs (если SSR) ← human sign-off
10. tests/integration/<module>_test.rs (with testcontainers)     ← human sign-off
11. tests/fuzz/<module>_tenant_isolation.rs                       ← human sign-off (1M attempts)
12. docs/perf/explain/<module>_*.txt (EXPLAIN ANALYZE baselines) ← auto-update
13. docs/validations/VAL-NNN-<module>.md (criteria signed off)    ← P4 sign-off
14. ops/caddy/<env>/snippets/<module>.caddy (routing delta)      ← human sign-off
15. ops/observability/dashboards/<module>.json (Grafana)         ← human sign-off
16. docs/releases/<version>/ROLLBACK.md (per-module recipe)      ← human sign-off
```

Без файла N+1, пока не подписан N. Это **прямая копия §9 TLA Entity Level 3 правила** «по одному файлу за шаг; перед следующим — согласование с человеком». Никаких автоматических batch commit'ов.

### 12.6.3 Per-Module Workflow — день за днём (template)

Для модуля среднего размера (cms_pages — 14 рабочих дней до GA):

| День | Активность | Артефакт | TLA Level | Gate |
|------|-----------|----------|-----------|------|
| D0 | RFC review meeting | docs/rfc/RFC-NNN-*.md | L1 | RFC sign-off |
| D1 | ADR + Plan draft | docs/adr/ + docs/plans/ | L2 | ADR sign-off |
| D2 | Schema migration expand | migrations/ + sqlx prepare | L3.1 | DBA sign-off |
| D3 | Domain crate | crates/domain/<module>/ | L3.2 | architecture review |
| D4–5 | Application ports + use cases | crates/application/ | L3.3–3.4 | code review |
| D6–7 | Infrastructure adapter (SQLx) | crates/infrastructure/ | L3.5 | query budget pass (§11.6) |
| D8 | Presentation handlers (HTTP + SSR) | crates/presentation/ | L3.6 | route smoke |
| D9 | Integration tests (testcontainers) | tests/integration/ | L3.7 | green CI |
| D10 | Tenant-isolation fuzz | tests/fuzz/ | L3.8 | 1M attempts, 0 leaks |
| D11 | Performance benchmark vs SITE1 | docs/perf/ + oha output | L3.9 | ≥ success criteria |
| D12 | Caddy + observability deploy в stage | ops/ + grafana dashboards | L3.10 | smoke на stage |
| D13 | Stage pilot tenant cutover | Caddy whitelist update | A start | latency parity 24h |
| D14 | GO/NO-GO decision meeting | docs/releases/<ver>/DECISION.md | gate | P4 sign-off |
| D15–28 | Phase A (Pilot, 1 production tenant) | observability watch | A run | 0 P1 alerts |
| D29 | Phase B start (2–3 tenants) | Caddy whitelist expand | B start | parity 2 weeks |
| D43 | Phase C start (all tenants) | Caddy whitelist all | C start | full traffic |
| D71 | Phase D (SITE1 module retire) | migrations contract step | D | OWNERSHIP.md final |

Большие модули (appointments — ~28 дней до GA) масштабируют D3–D11 ×2. Маленькие модули (services — простая CRUD без интегра ционных hairballs) сжимают до 10 дней. Никогда не сжимают Phase A < 14 дней — это **observability dwell time**, не девелопмент.

### 12.6.4 Cutover Procedure — пошаговый рецепт

**Pre-flight checklist (за 1 час до cutover):**

```bash
# 1. AX health
curl -fsS https://ax.example.com/health || abort
# 2. Sentry / OTLP / Grafana alerts up
# 3. Per-tenant baseline metrics в Grafana — snapshot за последние 7 дней
# 4. Test rollback в stage за 5 мин
# 5. On-call notified, rollback contact known
# 6. Database connection pool headroom ≥ 30% на обоих PgBouncer
# 7. Sticky session ОТКЛЮЧЕНА — JWT-based, host-independent
# 8. Cache CDN headers одинаковые на SITE1 и AX (Cache-Control, Vary)
# 9. OWNERSHIP.md и CONTRACTS/*.md — committed (не stash)
# 10. ROLLBACK.md для этого релиза — committed и прочитан on-call'ом
```

**Cutover для одного тенанта (Phase A → Phase B):**

```caddy
# ops/caddy/Caddyfile.snippets/cms-ax-pilots.caddy
@cms_ax_pilots {
  host pilot-tenant-1.spa.me
  path /home /about /contacts /services /price /reviews
}
handle @cms_ax_pilots {
  reverse_proxy ax-server:7000 {
    health_uri /health
    health_interval 5s
    health_timeout 1s
    fail_duration 30s
    max_fails 3
  }
}
# Остальной трафик уходит в SITE1 (handler ниже по правилам)
```

**Sequence:**

1. `caddy reload` (atomic, < 100ms)
2. Watch `tail -f /var/log/caddy/access.log | grep pilot-tenant-1` — первые 60 секунд
3. Grafana: latency, error rate, RSS — 5 минут
4. Если зелёное — продолжить watch ещё 1 час
5. Если красное — `git revert <caddy commit> && caddy reload` (rollback по §12.6.5)
6. Через 24h без инцидентов — eligible для расширения списка тенантов

### 12.6.5 Rollback Procedure — sub-5-минутный recipe

**Trigger conditions (любого достаточно):**
- p99 latency AX > 2× SITE1 baseline более 5 минут
- 5xx rate AX > 1% более 5 минут
- любой `TenantMismatch` event (§7)
- любой `unsafe_code` warning в production (защита есть, но если просочилось — критично)
- alloc rate > 110% budget (§11.5)

**Steps:**

```bash
# 1. Caddy whitelist revert (< 30s)
cd ops/caddy/
git revert <cutover_commit_sha>
caddy reload

# 2. Verify traffic вернулся в SITE1
tail -f /var/log/site1/access.log | grep pilot-tenant-1

# 3. AX-side: pause event consumers, чтобы не drift'нуть
systemctl stop ax-pgmq-consumer@cms.service

# 4. Snapshot diagnostic data
curl -s http://ax-server:7000/debug/dump > /tmp/ax-rollback-$(date +%s).json
journalctl -u ax-server.service --since "1h ago" > /tmp/ax-rollback-logs.txt

# 5. Page incident channel, attach diagnostic
slack-notify "#nas-ops" "Rolled back cms_pages cutover for pilot-tenant-1 — investigating"

# 6. Update OWNERSHIP.md → revert pilot tenant ownership claim
# 7. Reopen RFC, root-cause document, decide if Phase A repeats
```

**Schema rollback:** не делается **никогда** в Phase A/B/C. Все миграции expand-only (§21). Если схема плохая — следующий expand-фикс, не contract-rollback.

### 12.6.6 Validation Gates per цикл (12 gates, P4 sign-off условие)

| Gate | Owner | Pass criteria | Доказательство |
|------|-------|---------------|----------------|
| G1 Schema | DBA | expand-only, RLS политики покрывают новые таблицы/views, N-1 SITE1 читает new rows | testcontainers test diff'ит explain plans |
| G2 Domain invariants | architect | все invariants покрыты unit-тестами, no SITE1 imports | `cargo test -p domain` green |
| G3 Application use cases | code review | port traits только, mocked infrastructure, no leaks | `cargo test -p application` green |
| G4 Infrastructure | code review | SQLx prepared (offline), нет N+1, нет SELECT *, нет OFFSET | `cargo sqlx prepare --workspace` + §11.6 EXPLAIN diff |
| G5 Tenant isolation fuzz | security | 1M cross-tenant attack attempts, 0 leak detected | `cargo test --features fuzz fuzz_tenant_<module>` |
| G6 Performance | perf engineer | success criteria из RFC P95 + RSS | oha report committed в `docs/perf/<module>/<date>.txt` |
| G7 Allocation budget | perf engineer | RSS, heap, peak в budget §11.5 | dhat-rs report committed |
| G8 Query budget | perf engineer | EXPLAIN ANALYZE matches snapshot, no seq scan на rowcount > 1k | `docs/perf/explain/<module>_*.txt` committed |
| G9 Security audit | security | upload/SSRF/secret/CSP rules не ослаблены | `cargo deny check` + `cargo vet` + manual diff против §18 |
| G10 Interop contract | SITE1 lead + AX lead | OWNERSHIP.md updated, события versioned, anti-corruption mapping явный | `docs/interop/CONTRACTS/<module>.md` + 2-sided review |
| G11 Rollback drill | on-call | < 5 мин в stage, observability подтверждает traffic shift | `docs/releases/<ver>/ROLLBACK_DRILL.md` с timestamps |
| G12 Documentation | tech writer / architect | README, ROADMAP, OWNERSHIP обновлены | git diff на эти файлы в том же PR |

**Без всех 12 gates passed — модуль НЕ переходит из Phase A в Phase B.** PR-template (`.github/PULL_REQUEST_TEMPLATE/migration.md`) содержит 12 checkbox'ов; CI blocks merge на unchecked.

### 12.6.7 Observability During Cutover — что должно быть видно

**Метрики (Prometheus):**

```
# Per-tenant, per-module, per-stack
nas_request_duration_seconds{tenant, module, stack="site1|ax"}    histogram
nas_request_total{tenant, module, stack, status_class}            counter
nas_alloc_bytes_per_request{tenant, module, stack}                histogram (только AX)
nas_db_pool_active{stack, pool="api|report"}                      gauge
nas_pgmq_lag_seconds{topic, consumer_stack}                       gauge
nas_tenant_mismatch_total{stack}                                  counter (alert > 0)
```

**Grafana dashboard `cutover-watch`:**
- Top row: side-by-side p50/p95/p99 latency SITE1 vs AX (same tenants)
- Middle: error rate split by status_class
- Bottom: alloc + DB pool utilization
- Annotations: каждый `caddy reload` события

**Alerts (PagerDuty):**

| Alert | Trigger | Severity |
|-------|---------|----------|
| `MigrationLatencyDivergence` | AX p95 > 1.5× SITE1 p95 за 5m | P2 |
| `MigrationErrorDivergence` | AX 5xx > SITE1 5xx + 0.5pp за 5m | P1 |
| `TenantMismatchAny` | `nas_tenant_mismatch_total` rate > 0 | P1 (immediate page) |
| `AllocBudgetExceeded` | alloc > 110% RFC budget за 10m | P2 |
| `RollbackDrillStale` | last drill > 14d назад | P3 (planning) |

**Logs:**
- `request_id` ULID propagated cross-stack через `X-Request-Id` header
- Sentry tag `stack=site1|ax` + `migration_phase=pilot|stage|ga`
- AX `event=tenant_mismatch` → immediate Sentry event с tenant_id, user_id, request_id

### 12.6.8 Anti-Patterns — что НЕ делать

**Категорические запреты во время миграции:**

1. **Параллельная миграция двух модулей.** Single concurrent migration. Если бизнес хочет «и appointments, и cms одновременно» — RFC review должен отказать.
2. **Бенчмарк-only обоснование.** Если ≥ 3× выигрыша есть, но bus factor 1 — отказ (§13).
3. **Schema rename / drop в expand step.** Только additive. Contract step — отдельный релиз через N+2 минимум.
4. **Manual testing вместо fuzz.** Tenant isolation fuzz — обязательный gate, никакого «вручную проверили несколько кейсов».
5. **Dual-write SITE1 + AX в одну таблицу.** В Phase A/B/C — единственный canonical writer (см. §12.5 Ownership Table). Dual-write — это data drift incoming.
6. **Skip Phase A.** «У нас в stage хорошо прошло, давайте сразу на 5 тенантов» — нет. Phase A = 14 дней observability dwell.
7. **Cutover в пятницу.** Rollback должен быть в часы доступности команды. Каждый cutover — вторник или среда утром, не выходные.
8. **Использование SITE1 API в AX use case'ах.** AX читает SITE1-owned данные **только через published events или read-only views**. Прямой HTTP-call в SITE1 из AX use case = архитектурный лик; разрешён только в `infrastructure/sync/` адаптере с явным ADR.
9. **Rust-rewrite для exercise.** Каждая миграция должна закрывать конкретный business metric из RFC. «Перепишем salons потому что они простые» — отказ.
10. **«Один проход, переедем всё за квартал».** Цикл на модуль = 10–14 рабочих дней до Phase A + 4–6 недель Phase A–C. Реальный темп — 6–8 модулей в год при ≥ 2 разработчиках. Любой план быстрее = читерство.

### 12.6.9 Связь с другими секциями

| Топик | Источник | Применимость для миграции |
|-------|----------|---------------------------|
| Tenant context | §6 | `with_tenant` обёртка для каждого AX query модуля |
| Error handling | §7 | `TenantMismatch` — security event, мониторится отдельно |
| Validation | §8 | garde на боковом DTO + serde на anti-corruption mapping |
| Background jobs | §9 | pgmq consumers для published events SITE1; isolated runtime §4.8/§10.5 |
| Observability | §10 + §10.5 | shared request_id, отдельные runtime'ы для report/queue |
| Performance | §11.5 + §11.6 | allocation + query budgets в RFC success criteria |
| Interop | §12.5 | OWNERSHIP.md update — обязательный шаг каждого Phase D |
| Release engineering | §21 | expand-contract, blue/green, canary применяются буквально |
| CQRS discipline | §22 | миграция — не повод вводить read model split, только если ADR обоснует |
| Threat model | §18 | `unsafe_code` ban, upload policy наследуется неизменной |

---

## 12.7 Pilot Module Selection — Decision Matrix

Какой модуль идёт первым — не вопрос вкуса. §13 говорит «один модуль, самый load-bearing» — но это узкая эвристика. §12.7 — scoring-матрица для устранения произвольности.

**Критерии (взвешенные, 1–10):**

| # | Критерий | Вес | Объяснение |
|---|----------|-----|------------|
| C1 | Independence (низкая coupledness с другими SITE1 модулями) | 3× | Меньше cross-stack контрактов = меньше rollback blast radius |
| C2 | Read-heavy ratio (read% от total ops) | 2× | Read-only adapter тривиально откатить через Caddy; write — данные drift'нут |
| C3 | Existing test coverage в SITE1 | 2× | Спека «как должно работать» уже формализована — меньше regress'а |
| C4 | Business criticality (НИЖЕ — лучше для пилота) | 2× (inverted) | Пилот должен иметь low blast radius на бизнес |
| C5 | Perf upside potential (Rust expected gain) | 3× | Без серьёзного gain'а миграция = exercise |
| C6 | Data complexity (НИЖЕ — лучше) | 2× (inverted) | Сложные joins / aggregates сложнее повторить в Rust; нет нужды лидировать ими |
| C7 | Tenant isolation surface (НИЖЕ — лучше) | 1× | Меньше isolation paths = легче fuzz-покрыть |

**Кандидаты на 2026-Q3 (на основе текущего SITE1 Phase 0+1 состояния):**

| Модуль | C1 Indep | C2 Read% | C3 Tests | C4 Crit (inv) | C5 Perf | C6 Data (inv) | C7 Iso (inv) | Score | Verdict |
|--------|----------|----------|----------|---------------|---------|---------------|-------------|-------|---------|
| `cms_pages` | 9 (CMS не зависит от appointments / billing) | 9 (90% read — public render) | 7 (Stage 28+34) | 7 (выкл public render = плохо, но не fatal) | 9 (SSR — главный perf gap у Next) | 8 (плоские row'ы, jsonb body) | 7 (single-table+RLS) | **8.4** | **PILOT (1-й)** |
| `media` | 8 (только зависит от tenant) | 8 (uploads write-once, reads частые) | 5 (минимум тестов сейчас) | 5 (выкл upload = жалко, но не critical) | 9 (S3 streaming, file I/O — Rust сильнее) | 8 (key-value по сути) | 6 (RLS + S3 path scoping) | **7.6** | **2-й (после cms)** |
| `appointments` | 6 (зависит от staff / services / clients / salons) | 4 (write-heavy: create/update/cancel) | 7 (Phase 0 done, FSM tested) | 3 (центральный бизнес — выкл = катастрофа) | 8 (overlap check на больших календарях) | 5 (FSM + idempotency + overlap) | 6 (3 связанные таблицы) | **5.8** | **3-й (только после двух успешных)** |
| `services` | 8 | 7 | 8 (Stage 34) | 5 | 4 (CRUD простой, Node OK) | 8 | 8 | 6.7 | Не приоритет — нет perf upside |
| `salons` | 8 | 7 | 8 (Stage 34) | 5 | 4 | 8 | 8 | 6.7 | Не приоритет |
| `clients` | 7 | 6 | 7 (Stage 34) | 2 (PII!) | 5 | 6 | 5 | 5.4 | Никогда не пилот — PII overhead |
| `chat` | 4 (зависит от users + tenant_users + RBAC) | 5 | 9 (Stage 22 isolation + Stage 34) | 5 | 8 (SSE long-lived connections — Rust сильнее) | 6 | 4 | 5.9 | Risky — stateful, отложить |
| `staff` | 7 (M2M staff_services) | 6 | 7 (Stage 34) | 4 | 5 | 6 | 6 | 5.7 | Не приоритет |
| `tenants` (platform) | 10 (никто не зависит, все зависят от него) | 5 | 5 | 1 (выкл = вся платформа off) | 4 | 7 | 4 | 4.6 | **Никогда не первый.** Migrate последним. |

**Verdict 2026:**
- **1-й пилот:** `cms_pages` (score 8.4) — public render, read-heavy, isolated, Stage 28+31+34 уже sceleton'ит тесты
- **2-й:** `media` (7.6) — Rust сияет на streaming/file I/O, low criticality
- **3-й:** `appointments` (5.8) — но **только если первые два прошли GA без regression'ов**
- **Никогда первыми:** `clients` (PII), `tenants` (cascade на всё), `chat` (state)

**Pre-conditions для запуска (любого) пилота:**

1. SITE1 Phase 1 закрыт минимум на 3 месяца — стабильный baseline для бенчмарков
2. ≥ 2 Rust-разработчика в команде с production Axum опытом — bus factor
3. RFC + ADR sign-off на конкретный модуль с success criteria
4. PgBouncer + RLS validated in stage с обоими стеками
5. Caddy rolling-reload tested (no dropped connections)
6. Sentry + Grafana + alerts wired для AX
7. On-call rotation покрывает AX (не только SITE1)
8. Rollback drill отрепетирован минимум 1 раз в stage

Невыполнение любого из 8 — pilot не начинается. Игнорировать в стиле «потом доделаем» — гарантированный путь к §15 нового списка «что не повторять в v4.0».

---

## 13. Decision Criteria — когда AX оправдан

| Сценарий | Решение |
|----------|---------|
| Phase 0 не закрыт, Phase 1 не начат | **AX не делается.** Фокус: закрыть SITE1 Phase 0 → Phase 1. |
| SITE1 Phase 1 закрыт, бизнес растёт | **AX как PoC одного модуля.** Выбор модуля — §12.7 матрица (по умолчанию `cms_pages`, не `appointments`). |
| Бенчмарк AX даёт < 2× выигрыш | **Не мигрировать.** Стоимость переписывания не окупается. |
| Бенчмарк AX даёт ≥ 3× выигрыш + есть Rust-эксперт в команде | **Параллельный deploy, постепенный shift** — playbook §12.6, контракт §12.5. |
| Команда — один TS-разработчик | **AX никогда.** Bus factor становится 0 при потере человека. |
| Нужен perf-выигрыш под конкретную фичу (например ML inference, видео-обработка) | **AX как sidecar-сервис**, не replacement. Микро-сервис на Rust + SITE1 как orchestrator. |
| §12.6.7 pre-conditions не выполнены (≥ 2 Rust dev, drill отрепетирован, observability готова) | **Pilot не начинается.** Игнорировать = гарантированный regression catalog для v4.0 §15. |

---

## 14. Известные риски и mitigation

| Риск | Вероятность | Mitigation |
|------|-------------|------------|
| Leptos breaking changes / ecosystem stagnation | Высокая | §4.9 Presentation Contract — Leptos изолирован как один из адаптеров |
| SQLx compile-time errors блокируют CI без БД | Высокая | `cargo sqlx prepare --workspace` + commit `.sqlx/` метадаты; CI работает offline |
| RLS + PgBouncer interaction bugs | Средняя | Integration-тесты с реальным PgBouncer (testcontainers с двумя сервисами); explicit smoke-test после deploy'а |
| Rust dev velocity vs TS | Высокая (3-5× медленнее на feature work) | Принять как фактическое; AX оправдан **только** при значимом perf-выигрыше |
| Bus factor (Rust-эксперт уходит) | Высокая для single-dev команд | AX не запускается без ≥ 2 человек, способных поддерживать |
| Cargo build times | Средняя (full rebuild 2-5 мин) | `sccache` + `cargo-chef` в Dockerfile; incremental builds в dev; §2.8 compile-time alert |
| Hot-reload в Leptos слабее Next | Средняя | `cargo-leptos watch` приемлем, но не на уровне Next; принять |
| `axum` major version bumps | Средняя | Pin minor; major upgrade — отдельный спринт + ADR |
| Memory leaks в Leptos signals | Средняя | Code-review правило: signal hygiene (`Owner` scope), `Drop`-checks в dev |
| OpenAPI drift (Rust ↔ TS клиенты) | Средняя | `utoipa-axum` генерирует OpenAPI как часть build; клиенты генерируются из этого, не пишутся руками |
| Rust over-engineering (CQRS/ES/macro-DSL creep) | **Высокая** | §2.7 NO MAGIC + §22 CQRS Discipline + complexity review триггер §2.8 |
| Heavy reports starving API latency | Высокая | §10.5 Failure Domain Isolation — отдельные runtime'ы |
| Allocation regression (Arc cloning, async task explosion) | Высокая | §11.5 Allocation Budget + dhat-rs CI |
| Query regression (N+1, sequential scan на росте данных) | Высокая | §11.6 Query Budget + EXPLAIN snapshot CI |
| Supply-chain compromise через транзитивную зависимость | Средняя (растущая угроза) | §19 Dependency Governance (cargo-deny + cargo-vet + cargo-audit) |
| Tenant data leak через shared-DB взаимодействие SITE1↔AX | **Высокая** при отсутствии контракта | §12.5 Interop Boundary с явным ownership + anti-corruption layer |
| Orphan tokio tasks / cancellation bugs | Высокая | §4.8 TaskSupervisor — нельзя `tokio::spawn` напрямую |
| GDPR / data lifecycle non-compliance | Высокая (регуляторный риск) | §20 Data Lifecycle Governance |
| Release breaking N-1 compatibility | Средняя | §21 Release Engineering — expand-contract обязателен |

---

## 15. Что НЕ повторять из v1.0 и v2.0 (критика)

### v1.0 → v2.0 правки (зафиксированы)

1. **"Godlike" branding** удалено. Архитектура оценивается по результату.
2. **Cold start < 80 ms** заменено на реалистичный 150–200 ms.
3. **"Playwright (Rust bindings)"** заменено на `fantoccini` / external JS-Playwright.
4. **PgBouncer + transaction mode** добавлен как load-bearing для RLS.
5. **`tracing-opentelemetry`** добавлен (v1.0 потерял distributed traces).
6. **argon2** вместо подразумеваемого bcrypt.
7. **Signal hygiene** для Leptos формализован.
8. **CQRS** ограничен trigger'ом (read/write model divergence) — окончательно фиксирован в §22.
9. **`utoipa-axum` integration** определена.
10. **`pgmq` для background jobs** — v1.0 пропустил.
11. **Newtype `TenantId` + `TenantScoped<T>`** как compile-time gate.
12. **Decision criteria §13** добавлены.

### v2.0 → v3.0 правки (новые)

13. **v2.0 описывал HOW the system is structured, а не HOW it is built.** v3.0 добавил §2.5 Engineering Execution Model (RFC/ADR/Plan/Validation pipeline).
14. **Архитектурные правила были декларативны, не enforced.** v3.0 §2.6 фиксирует Architecture Fitness Functions (xtask + cargo-deny + custom CI).
15. **Не было allocation budget.** v3.0 §11.5: dhat-rs + jemalloc-ctl + numeric budgets + CI regression.
16. **Не было concurrency policy.** v3.0 §4.8 TaskSupervisor, запрет raw `tokio::spawn`, явный backpressure model.
17. **Leptos был core dependency.** v3.0 §4.9 делает Leptos одним из replaceable адаптеров через Presentation Contract.
18. **Не было anti-corruption layer для SITE1↔AX.** v3.0 §12.5 — ownership table, event contracts, rollback contracts.
19. **Не было failure isolation.** v3.0 §10.5 — отдельные runtime'ы для HTTP/queue/image/report.
20. **Security была размазана.** v3.0 §18 консолидирует threat model + `#![forbid(unsafe_code)]` + upload/SSRF/secrets policy.
21. **Не было dependency governance.** v3.0 §19: cargo-audit / deny / vet / udeps mandatory.
22. **Не было data lifecycle policy.** v3.0 §20: soft-delete / GDPR erase / backups / retention / audit log.
23. **Не было query budget.** v3.0 §11.6: keyset only, no SELECT *, no OFFSET, EXPLAIN snapshots в CI.
24. **Не было release engineering.** v3.0 §21: expand-contract migrations, blue/green, canary, feature flags, N-1 schema compatibility, mandatory rollback script.
25. **CQRS guidance был мягким.** v3.0 §22 делает CRUD service дефолтом, CQRS — только с measurable divergence + ADR.
26. **Не было NO MAGIC POLICY.** v3.0 §2.7 явно запрещает macro DSL, hidden proc-macros, runtime reflection, implicit globals, hidden IO.
27. **Не было complexity budgets.** v3.0 §2.8 фиксирует function length, trait methods, enum variants, async nesting, compile time с автоматическим regression alarm.

### v3.0 → v3.1 правки (alignment с parent constitution)

28. **v3.0 был написан без чтения `barbie/ENTITY.md`.** Это создало (a) дублирование принципов (NO MAGIC ↔ §11 Philosophy, Security Model ↔ §11 Security Mentality) без attribution, (b) пробелы (нет связи с §9 TLA Entity, §6 VPS, §11 Delivery Standard и Cross-Session Continuity). v3.1 добавил §0.1 explicit boundary, origin-cite в §2.7 и §18, §21.0 VPS baseline reference, в §2.5 — связь с §9 TLA Entity и §11 Delivery Standard, в §23 — расширенный parent reference с указанием релевантных секций.
29. **AX не был зарегистрирован в `barbie/ENTITY.md §0` таблице проектов.** v3.1 добавил строку `AX | barbie/ax/ | Rust-альтернативная спецификация NAS | exploration spec` в родителе. Это единственная правка parent constitution; всё остальное — AX-internal.
30. **§21 Release Engineering переопределял VPS-правила, дублируя `barbie/ENTITY.md §6`.** v3.1 разделил на §21.0 (наследуемый baseline) и §21.1 (AX-специфичные расширения: blue/green, canary, expand-contract). systemd vs PM2 — единственная AX-специфика, обоснована: cargo-built бинарь не нужно процесс-менеджерить через Node-PM2.

### v3.1 → v3.2 правки (миграционный playbook)

31. **§12 описывал условия миграции, но не процесс.** Между «можно начинать» и «работает в production» зияла пустота — каждый, кто начнёт миграцию, изобрёл бы её по-своему. v3.2 закрывает: §12.6 Migration Playbook с 4 фазами (Pilot / Stage / GA / Retire), пошаговым cutover и rollback < 5 мин, 12 validation gate'ами.
32. **TLA Entity не был применён к миграции.** v3.2 явно маппит §9 TLA Entity (L1 Strategic / L2 Architectural / L3 Incremental) на миграционный цикл одного модуля + связывает с §2.5 артефактами (RFC/ADR/Plan/Validation). Один цикл миграции = одна полная итерация TLA.
33. **Выбор пилот-модуля был произвольным («самый load-bearing»).** v3.2 §12.7 — scoring matrix по 7 критериям. Итог: `cms_pages` ≠ `appointments`. Перевод аргумента из «по ощущению» в воспроизводимое решение.
34. **Не было анти-паттернов миграции.** v3.2 §12.6.8 — 10 категорических запретов (параллельная миграция двух модулей, schema rename в expand, dual-write, cutover в пятницу, skip Phase A). Без этого список — гарантированный regression catalog.
35. **§13 Decision Criteria ссылался на §12 + §12.5, но не на playbook.** v3.2 обновил §13 ссылкой на §12.6.7 pre-conditions + §12.7 матрицу; убрал hard-code «appointments» из дефолта.

---

## 16. Локальный dev

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.84"
components = ["rustfmt", "clippy"]
```

```bash
# Один раз
cargo install cargo-leptos sqlx-cli cargo-chef cargo-deny cargo-audit cargo-udeps cargo-vet

# Стартап
docker compose -f docker-compose.dev.yml up -d
cd barbie/ax
sqlx migrate run
cargo leptos watch     # SSR + hot-reload

# Pre-commit (рекомендуется как git hook)
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo xtask architecture-check
cargo xtask magic-check
cargo deny check
```

**Порты разведены с SITE1 (чтобы оба стека жили одновременно):**

| Что | SITE1 | AX |
|-----|-------|------|
| API | :3010 | :3020 |
| Web/SSR | :3011 | :3021 |
| Postgres | :5442 | :5452 |
| PgBouncer | — | :6452 |
| MinIO | :9011/9012 | :9021/9022 |
| Mailhog | :8035/8025 | :8045/8024 |

**Имя docker-проекта:** `barbie-ax-dev` (через `COMPOSE_PROJECT_NAME` или `-p`).

**Важно:** на диске путь — `barbie/ax/` (lowercase), не `barbie/AX/`. Все внутренние ссылки используют lowercase. Linux VPS — case-sensitive, не путать.

---

## 17. Тесты

| Уровень | Стек | Coverage цель |
|---------|------|---------------|
| Unit (domain) | `cargo test` | 90%+ для invariants |
| Unit (application) | `cargo test` + `mockall` для traits | 80%+ для use cases |
| Integration (infra) | `testcontainers-rs` + real Postgres + RLS | 100% для tenant-isolation |
| E2E (HTTP) | `axum::Router::oneshot` + `reqwest` | golden paths |
| Browser | `fantoccini` или внешний JS-Playwright | smoke на 3-5 ключевых flow |
| Load | `oha` или `k6` | Performance Budget §11 |
| Fuzz (tenant-isolation) | `cargo-fuzz` или property-based via `proptest` | 100% read endpoints |
| Allocation | `dhat-rs` bench-bin | budgets §11.5 |
| Query plan | EXPLAIN snapshots | §11.6 |

**Tenant-isolation test — обязательный шаблон** для каждого нового read/write endpoint'а:

```rust
#[sqlx::test]
async fn appointment_isolation(pool: PgPool) {
    let (tenant_a, tenant_b) = seed_two_tenants(&pool).await;
    let appt_a = create_appointment_for(&pool, tenant_a).await;

    // tenant B пытается прочитать appointment A
    let result = with_tenant(&pool, &ctx_for(tenant_b), |tx| {
        Box::pin(async move {
            appointment_repo::find_by_id(tx, appt_a.id).await
        })
    }).await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}
```

**Этот тест прогоняет реальный SQL через реальный RLS**, не mock. Единственный надёжный тест мульти-tenant safety. Mock-based unit-тесты (как в SITE1 `test-utils/mock-db.ts`) проверяют только "программист вспомнил скоупнуть", а не "БД физически не отдаёт чужие строки". Compile-time `TenantScoped<T>` + RLS-integration tests — это и есть defence in depth, обещанный в §3.

---

## 18. Security Model

**Origin:** расширение `barbie/ENTITY.md §11 Security Mentality` для Rust-стека. Родительская формулировка: *"Default stance: everything is hostile. Continuously applies OWASP Top 10 awareness, zero-trust principles, rate limiting, brute-force protection, upload sanitization, permission-escalation analysis, and secret isolation. Security debt is never hidden — it is surfaced immediately and explicitly."*

§18 фиксирует конкретные Rust-специфичные mitigation'ы (forbid unsafe, secrecy crate, rustls вместо openssl, infer для sniffing, DNS-allowlist для SSRF), но **philosophy и trigger'ы — у родителя**.

Security разрозненная по коду = security неаудитабельная. v3.0 консолидирует.

### 18.1 Threat model (high-level)

| Threat | Surface | Mitigation |
|--------|---------|------------|
| Cross-tenant data read | API endpoints + DB | §3 5-уровневая + §17 isolation fuzz |
| Account takeover | auth flow, session cookies | argon2, rotating session IDs, CSRF, rate-limit on /login |
| Supply-chain compromise | Cargo.lock, transitive deps | §19 cargo-deny/vet/audit |
| File upload abuse (RCE, polyglot, SSRF via fetch) | /upload endpoints | type-sniff `infer`, re-encode images, max size, clamd scan |
| SSRF | webhook handlers, image fetch by URL | DNS allowlist в reqwest client |
| Deserialization attack | API JSON, session | serde `deny_unknown_fields`, no bincode для untrusted |
| Secret leak via logs | tracing output | `secrecy::SecretString`, lint на Debug-derive для secret types |
| CSP bypass / XSS | rendered HTML | nonce-based CSP, askama auto-escape |
| Tenant DNS hijack | custom domains | Caddy on-demand TLS validation endpoint, manual approval queue для новых доменов |

### 18.2 Trust boundaries

```
Internet ──[TLS]──> Caddy ──[mTLS opt]──> Axum
                                  │
                              [tenant-resolved]
                                  │
                              Application
                                  │
                       [TenantContext + RLS SET LOCAL]
                                  │
                              PgBouncer ──> Postgres
```

Каждый переход = explicit validation. **Никогда** не доверять данным после прошлого hop без re-validation.

### 18.3 `#![forbid(unsafe_code)]`

В workspace root:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
```

Исключения через `[workspace.metadata.unsafe-allowed]` (комментарий с обоснованием обязателен):

- allocator crates (jemallocator)
- SIMD intrinsics (если когда-либо понадобится — image processing)
- vetted FFI (с cargo-vet trust)

xtask проверяет, что новых `#[allow(unsafe_code)]` без metadata-entry нет.

### 18.4 Upload policy

```rust
// crates/infrastructure/src/storage/upload.rs

pub async fn accept_upload(
    bytes: Bytes,
    declared_content_type: &str,
) -> Result<StoredFile, AppError> {
    // 1. размер
    if bytes.len() > MAX_UPLOAD_BYTES { return Err(AppError::BadRequest("too large".into())); }

    // 2. sniff — не верим declared_content_type
    let kind = infer::get(&bytes).ok_or(AppError::BadRequest("unknown type".into()))?;
    if !ALLOWED_MIMES.contains(&kind.mime_type()) {
        return Err(AppError::BadRequest("type not allowed".into()));
    }

    // 3. re-encode для изображений (полностью переписываем — никаких polyglot)
    let normalized = if kind.matcher_type() == infer::MatcherType::Image {
        reencode_image(&bytes, kind)?
    } else {
        bytes
    };

    // 4. virus scan (clamd via tokio process)
    clamav::scan(&normalized).await?;

    // 5. tenant-scoped storage key
    let key = format!("t/{}/u/{}/{}", tenant_id, user_id, ulid::Ulid::new());
    storage::put(&key, normalized).await?;
    Ok(StoredFile { key, kind })
}
```

### 18.5 SSRF policy

`reqwest::Client` builds с DNS resolver, отвергающим:
- private CIDR (10/8, 172.16/12, 192.168/16, 127/8, ::1/128, fc00::/7)
- AWS/GCP metadata (169.254.169.254)
- linklocal (169.254/16)

Outbound HTTP только через этот client. xtask проверяет, что нет прямых `reqwest::get()` вне `crates/infrastructure/http/`.

### 18.6 Secret handling

```rust
use secrecy::{SecretString, ExposeSecret};

#[derive(Clone)]
pub struct DbConfig {
    pub url: SecretString,  // Debug skipped автоматически
}

// При использовании:
let conn_str = config.url.expose_secret();  // explicit, видно в review
```

Лог `tracing` ловит `SecretString` через стандартный Debug — выдаёт `[REDACTED]`. xtask проверяет, что новые struct с полями содержащими `password`, `token`, `secret`, `key` — используют `SecretString`.

### 18.7 CSP / CSRF

- CSP: `default-src 'self'; script-src 'self' 'nonce-{request_nonce}'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:` — nonce генерится per-request в middleware, injected в Leptos head.
- CSRF: tower-sessions + double-submit cookie pattern; Server Functions автоматически проверяют CSRF token.
- HSTS: `max-age=31536000; includeSubDomains; preload` (после testing period).

### 18.8 Аудиты

- **Internal:** каждые 6 месяцев — review threat model, обновление `docs/security/THREAT_MODEL.md`.
- **External:** перед public launch — независимый pen-test (бюджет, не "если будет время").

---

## 19. Dependency Governance

Rust supply-chain — растущая attack surface. xz-utils incident 2024, npm-стиль протечки в Cargo неизбежны.

**Mandatory tooling (CI):**

| Tool | Purpose | Frequency |
|------|---------|-----------|
| `cargo-audit` | RustSec CVE feed | nightly + перед release |
| `cargo-deny` | license allowlist, ban list, advisory, source registry | каждый PR |
| `cargo-udeps` | dead dependencies | nightly |
| `cargo-vet` | trust delegation (audit policy) | каждый PR diff |
| `cargo-crev` | community reviews (опционально) | manual для подозрительных |

**Policy для новых зависимостей:**

1. Любая новая crate в `[dependencies]` ⇒ ADR (P2 §2.5) с обоснованием:
   - почему не написать самим
   - кто maintainer (1 человек = высокий риск)
   - downloads / stars / last release date
   - transitive size
2. crate в hot-path (request handler, query path) ⇒ дополнительно `cargo vet certify` review.
3. crate в auth / crypto / serialization ⇒ только из:
   - rustcrypto/*
   - tokio-rs/*
   - serde-rs/*
   - dtolnay/*
   - bytecodealliance/*
   - hyper-rs/*
   - другое — отдельный ADR с pen-tester review

**`cargo-vet` trust delegation** (`supply-chain/audits.toml`):
- доверяем audits от: bytecodealliance, mozilla, embark-studios, fermyon
- наши собственные audits для специфичных crates

**Update policy:**
- patch updates (`x.y.Z`) — auto-merge, если CI зелёный
- minor (`x.Y.0`) — manual review + smoke benchmark
- major (`X.0.0`) — ADR обязательно
- Cargo.lock коммитится всегда

---

## 20. Data Lifecycle Governance

CRM с tenant-данными требует явной data policy. Без неё первый же GDPR-запрос становится месячным проектом.

| Concern | Policy |
|---------|--------|
| Soft delete | все user-facing сущности имеют `deleted_at TIMESTAMPTZ`; видимы в admin trash 30 дней |
| Hard delete | cron job (через pgmq scheduled) удаляет `deleted_at < NOW() - INTERVAL '30 days'` ежедневно |
| GDPR erase request | per-user erase job: anonymize PII (`email = 'erased-{user_id}@deleted'`, `name = 'erased'`), сохранить billing 7 лет (legal retention) |
| Audit log | append-only таблица `audit_events`, partition by month, REVOKE DELETE/UPDATE на role app_tenant; archive после 2 лет в cold storage |
| Backups | `pg_basebackup` ежедневно + continuous WAL archiving в S3-compatible (Backblaze B2) |
| PITR | RTO 1h, RPO 5 min; квартальная drill (restore в staging) |
| Retention — logs | structured logs 30 дней (Loki), затем drop |
| Retention — traces | OTLP traces 7 дней (Tempo), затем drop |
| Retention — sessions | inactive > 30 дней — purged ежедневно |
| Tenant export | per-tenant NDJSON archive (по entity type) + media manifest, signed S3 URL, expire 7 дней |
| Tenant deletion (full) | manual approval queue, 30-day grace period, затем hard erase кроме legal-hold данных |

**Encryption at rest:**
- Postgres: cluster-level encryption (LUKS на VPS)
- S3 backups: SSE-C с ключом в vault
- секреты в `secrecy::SecretString` в runtime (§18.6)

**Audit log контракт:**

```rust
// crates/domain/audit/src/lib.rs
pub struct AuditEvent {
    pub id: Ulid,
    pub tenant_id: TenantId,
    pub actor: Actor,           // User | System | ApiKey
    pub action: AuditAction,    // closed enum, не строки
    pub target: AuditTarget,    // entity_type + entity_id
    pub diff: Option<JsonValue>,// before/after для writes
    pub request_id: RequestId,
    pub occurred_at: DateTime<Utc>,
}
```

Каждый write через service-layer ⇒ автоматически логируется в `audit_events` в той же транзакции. xtask проверяет, что service methods пишущие в репо вызывают `AuditService::record`.

---

## 21. Release Engineering

### §21.0 VPS baseline (наследуется из `barbie/ENTITY.md §6`)

AX-deployment **не переопределяет** правила из родителя — наследует и расширяет:

- **один контур** локально ↔ GitHub ↔ VPS (`docker-compose.dev.yml`, `.env.example`, `docs/DEPLOY_SERVER.md` — в синхроне)
- `POSTGRES_PASSWORD` в compose **обязан совпадать** с паролем в `DATABASE_URL` (28P01-trap: смена `.env` не меняет существующий том)
- **никогда** `docker compose down -v` на проде
- after-pull скрипт обязателен: для AX это `cargo build --release --locked && sqlx migrate run && systemctl reload barbie-ax-api` (rust-эквивалент `npm run vps:after-pull` из родителя)
- AX использует **systemd** (не PM2): `EnvironmentFile=` подхватывает обновлённый `.env` через `systemctl daemon-reload && systemctl restart barbie-ax-api`
- **one-shell-command-per-message** правило для AI в SSH-сессии (как в родителе)
- отдельная Postgres-БД для AX (не схема в общей), отдельный поддомен, отдельный systemd unit

AX **расширяет** baseline следующими дисциплинами (ниже).

### §21.1 Стратегии (специфично для AX)

Без discipline здесь любой PR может уронить production.

| Concern | Strategy |
|---------|----------|
| Blue/green | два инстанса Axum за Caddy, health-gated swap, rollback = swap back |
| Canary | tenant-based: 1 тенант → 5 → 25 → all; gate на каждом шаге = SLO check (latency, error rate) |
| Migrations | **expand-contract** обязательно: (1) schema add deploy → (2) code deploy → (3) schema cleanup deploy — три отдельных release |
| Migration rollback | mandatory `down.sql` для каждой `up.sql` (sqlx это не требует — мы требуем); CI отвергает PR с миграцией без down |
| Code rollback | каждый release tag имеет `docs/releases/{tag}/rollback.md` с пошаговой инструкцией |
| Feature flags | mandatory для любого user-visible behavior change; default off, ramp по tenant; flag cleanup ADR через 90 дней (см. CLAUDE.md /schedule pattern) |
| Schema compatibility | **N-1** всегда: код на версии N должен читать данные, написанные N-1; xtask проверяет на CI через `sqlx prepare` против snapshot прошлого release |
| Release cadence | weekly (Tue), hotfix anytime; freeze на праздники |
| Deployment artifact | single static binary (musl или glibc, по target), images SHA256-pinned |

**Expand-contract пример (добавление NOT NULL column):**

```
Release N:
  migration:  ALTER TABLE x ADD COLUMN y TEXT;  -- nullable
  code:       читает x.y as Option<String>, пишет всегда Some(...)

Release N+1:
  migration:  UPDATE x SET y = 'default' WHERE y IS NULL;
              ALTER TABLE x ALTER COLUMN y SET NOT NULL;
  code:       читает x.y as String

Release N+2:
  cleanup: удалить legacy Option-handling код
```

**Никогда:** не делать ALTER NOT NULL без backfill между deploys.

**Pre-release checklist (`docs/releases/CHECKLIST.md`):**

- [ ] все P4 Validation Specs зелёные
- [ ] alloc budget regression < 20%
- [ ] query budget regression < 30%
- [ ] cargo-audit чисто
- [ ] cargo-deny чисто
- [ ] миграции expand-contract compliant
- [ ] rollback.md написан
- [ ] feature flags задокументированы
- [ ] EXPLAIN snapshots обновлены
- [ ] OpenAPI diff review'д

---

## 22. CQRS Discipline

Rust-команды склонны over-engineer CQRS / Event Sourcing. Это убивает velocity и читаемость.

**Default:** простой **service-layer CRUD**:

```rust
pub struct AppointmentService { repo: Arc<dyn AppointmentRepository> }

impl AppointmentService {
    pub async fn create(&self, ctx: &TenantContext, input: CreateAppointmentInput)
        -> Result<Appointment, AppError> { /* validate + persist + audit */ }

    pub async fn find_by_id(&self, ctx: &TenantContext, id: AppointmentId)
        -> Result<Appointment, AppError> { /* repo.find */ }
}
```

**CQRS разрешён ТОЛЬКО при measurable read/write model divergence.** Триггеры:

- reporting: aggregated view, отличающаяся от write-model (например, "tenant-wide booking heatmap")
- search: full-text index (Postgres tsvector или внешний — Meilisearch/Tantivy)
- analytics: pre-computed metrics с другим update cadence
- public-facing read APIs с резко другой shape данных (например, embed widget)

**Введение CQRS требует:**
1. ADR (§2.5) с конкретным measurement: "read p95 = X, write p95 = Y, у нас 100:1 read/write ratio и shape отличается"
2. явное разделение `commands/` и `queries/` в `application/` для затронутого модуля
3. projection refresh strategy (sync через triggers / async через pgmq)
4. документация в `docs/adr/` "когда projection считается stale"

**Event Sourcing — отдельный, ещё более ограниченный паттерн.** Триггеры:
- регуляторное требование иммутабельного audit trail (банкинг, healthcare)
- temporal queries — "состояние сущности на дату X"
- distributed reconciliation между несколькими сервисами

**Для CRM-домена ни один из этих триггеров неактуален.** Event Sourcing в AX **запрещён до явного бизнес-кейса с ADR**.

**Что НЕ является CQRS-триггером:**
- "будет красивее"
- "так делают в DDD книжках"
- "хочу попробовать"
- "может пригодится позже"

---

## 23. Карта документов

| Документ | Назначение |
|----------|------------|
| `ENTITY.md` (этот файл) | Конституция AX (exploration spec) |
| `../ENTITY.md` (`barbie/ENTITY.md`) | **Canonical workspace constitution.** AX подчиняется (§0.1). Релевантные секции: §6 VPS-регламент, §9 TLA Entity, §11 Engineering Entity / Security Mentality / Philosophy / Delivery Standard / Cross-Session Continuity |
| `../../ENTITY.md` (`ES/ENTITY.md`) | Прародительская конституция ES (Escort Platform); barbie форкнут отсюда. AX напрямую не наследует, но §6 VPS-регламент родителя берёт оттуда |
| `../../CLAUDE.md` (`ES/CLAUDE.md`) | §M режимы MANUAL/SEMIAUTO/AVTONOM + spine-list. AX-документ = spine |
| `../SITE1/ROADMAP.md` | Production-таргет NAS, состояние SITE1 (для §13 Decision Criteria) |
| `Cargo.toml` (workspace) | Crate manifest (создаётся при бенчмарк-PoC) |
| `deny.toml` | cargo-deny configuration (§2.6, §19) |
| `migrations/` | SQL миграции (создаются при первой имплементации) |
| `docs/rfc/` | P1 Strategic — RFC артефакты (§2.5) |
| `docs/adr/` | P2 Architectural — ADR артефакты |
| `docs/plans/` | P3 Execution — Implementation Plans |
| `docs/validations/` | P4 Verification — Validation Specs |
| `docs/perf/explain/` | EXPLAIN ANALYZE baselines (§11.6) |
| `docs/baselines/` | compile-time, alloc budgets baselines |
| `docs/releases/` | Per-release rollback и changelog |
| `docs/security/` | THREAT_MODEL.md, audit reports (§18) |
| `docs/interop/` | SITE1↔AX ownership, contracts, events (§12.5) |
| `docs/interop/CONTRACTS/` | Per-entity contract (schema, invariants, writer/reader, version) — обязательное чтение перед cutover'ом (§12.6.4) |
| `docs/interop/EVENTS/` | pgmq topics, payload schema с version, dual-publish правила (§12.5) |
| `docs/interop/MIGRATIONS/POLICY.md` | Кто владеет схемой какой таблицы; expand-contract правила (§12.6.5) |
| `docs/interop/ROLLBACK/` | Per-module < 5 мин rollback recipe (§12.6.5) |
| `docs/releases/<ver>/ROLLBACK.md` | Per-release rollback (требуется как gate G11, см. §12.6.6) |
| `docs/releases/<ver>/ROLLBACK_DRILL.md` | Подтверждение, что drill < 5 мин в stage проходил (gate G11) |
| `docs/releases/<ver>/DECISION.md` | GO/NO-GO решение на переход Phase A → B (§12.6.3 D14) |
| `ops/caddy/` | Caddy snippets для tenant routing + cutover (§12.6.4) |
| `ops/observability/dashboards/` | Grafana JSON dashboards включая `cutover-watch` (§12.6.7) |
| `xtask/` | Custom build automation (§2.6) |
| `istori/` | Сырая хронология AI-сессий (промт + response per turn); gitignored (§24) |
| `.claude/hooks/` | PowerShell-скрипты для SessionStart/UserPromptSubmit/Stop (§24) |
| `.claude/settings.json` | Регистрация hooks (§24) |

---

## 24. Session History (istori/)

**Назначение:** автоматическая запись каждой Claude Code-сессии в `barbie/ax/istori/` для cross-session continuity. Дополняет `barbie/ENTITY.md §11 Cross-Session Continuity` и `memory/` mechanism — не заменяет.

**Механизм — hooks в `.claude/settings.json` (выполняет харнесс Claude Code, не модель):**

| Hook | Trigger | Действие |
|------|---------|----------|
| `SessionStart` | старт каждой сессии | создаёт `istori/YYYY-MM-DD_HH-mm.md`; latest предыдущий файл инжектится в context как additional system context |
| `UserPromptSubmit` | после каждого user-prompt | дописывает `## PROMPT HH:mm:ss` + текст промта в активный session-файл |
| `Stop` | завершение Claude turn'а | извлекает последний text-block ассистента из transcript jsonl и дописывает `### RESPONSE HH:mm:ss` (≤ 1200 символов, дальше `[truncated]`) |

**Файлы:**

- `istori/*.md` — session-файлы; gitignored (per-machine — содержат сырые промты, потенциально с секретами)
- `istori/.gitkeep` — единственный файл папки, попадающий в git
- `.claude/hooks/session-start.ps1`, `user-prompt.ps1`, `stop.ps1` — PowerShell 5.1 скрипты (Windows-only)
- `.claude/.istori-current` — runtime pointer на активный session-файл; gitignored

**Соотношение со смежными механизмами:**

| Механизм | Scope | Curation | Что хранит |
|----------|-------|----------|-----------|
| `memory/` (`MEMORY.md` + per-fact `.md`) | `~/.claude/projects/<proj>/memory/`, локально | curated AI'ем | user/feedback/project/reference факты, долгоживущие |
| `SESSION_LOG.md` (AVTONOM, `CLAUDE.md §M`) | репо, в git | curated AVTONOM-сессией | финальный отчёт автономной работы |
| `NON_PROJECT/session-plans/` | репо, в git | curated до старта | план AVTONOM-сессии |
| **`istori/` (§24)** | локально, gitignored | **uncurated, raw** | дословная хронология промтов + responses |

`istori/` — низкоуровневый layer "всё подряд". `memory/` и `SESSION_LOG.md` — curated слои поверх.

**Privacy:** содержит дословные промты пользователя — секреты, дампы ошибок, debug-output могут попадать в файл. Папка gitignored. **Не публиковать. Не шарить через git.** Перед `git add` всегда проверять, что `istori/` не попадает.

**Текущие ограничения:**

- работает когда cwd = `barbie/ax/` (settings.json локальный). Для аналогичного покрытия `barbie/SITE1/` — создать собственный набор hooks в `barbie/SITE1/.claude/settings.json`, или поднять на `barbie/.claude/` для cross-subdir.
- параллельные Claude-сессии в одном cwd клобберят `.claude/.istori-current` — последний `SessionStart` побеждает. На practice редкий случай.
- Windows-only (PowerShell). Для Linux/macOS — переписать скрипты на bash.
- `Stop` hook сохраняет только первый text-block последнего assistant-сообщения. Если ответ был чисто tool-use без текста — запишется маркер `TURN END HH:mm:ss (no text response)`.

**Связь с CLAUDE.md §M:** istori/ работает во всех режимах (MANUAL/SEMIAUTO/AVTONOM) одинаково. Сам факт записи hook'ом не требует одобрения spine-touch — это инфраструктура, а не правка spine-файлов.

---

## История

- **v1.0 — 2026-05-24** — исходный черновик "Senior Godlike Architect" (4-Layer TLA, 5-уровневая защита, базовый стек).
- **v2.0 — 2026-05-24** — адаптация и углубление:
  - §0: явный status "exploration, не commitment миграции"
  - §3: PgBouncer transaction-mode + newtype `TenantId` как compile-time защита
  - §4: конкретные crates с версиями + альтернативы и обоснования
  - §6: Rust-код tenant context middleware + `with_tenant`
  - §9: pgmq вместо неуказанной очереди
  - §10: `tracing-opentelemetry` + ULID request-id + Sentry scope с tenant_id
  - §11: реалистичный performance budget с методологией
  - §12: миграционный путь vertical-slice-by-slice + общая БД
  - §13: decision criteria с явными gate'ами и сценариями
  - §14: расширенная risk-matrix с конкретными mitigations
  - §15: критика v1.0 пунктом (12 правок)
  - §16-17: локальный dev + тесты с реальными crates и Postgres/RLS
- **v3.0 — 2026-05-24** — переориентация документа с "как устроено" на "как строится и не деградирует":
  - §2.5 Engineering Execution Model (RFC/ADR/Plan/Validation pipeline)
  - §2.6 Architecture Fitness Functions (xtask + cargo-deny + CI gates)
  - §2.7 NO MAGIC POLICY (запрет macro DSL, hidden proc-macro, runtime reflection, implicit globals)
  - §2.8 Complexity Governance (function/trait/enum/async бюджеты + architecture review триггеры)
  - §4.8 Concurrency Model (TaskSupervisor, запрет raw `tokio::spawn`, backpressure)
  - §4.9 Presentation Contract (Leptos как один из replaceable адаптеров)
  - §10.5 Failure Domain Isolation (отдельные runtime'ы HTTP/queue/image/report)
  - §11.5 Allocation Budget (dhat-rs + jemalloc + numeric budgets)
  - §11.6 Query Budget Enforcement (keyset only, EXPLAIN snapshots, no SELECT *, no OFFSET)
  - §12.5 Interop Boundary SITE1↔AX (ownership table, event contracts, anti-corruption layer)
  - §18 Security Model (threat model, `#![forbid(unsafe_code)]`, upload/SSRF/secrets/CSP policy)
  - §19 Dependency Governance (cargo-audit/deny/vet/udeps mandatory)
  - §20 Data Lifecycle Governance (soft-delete, GDPR erase, backups, retention, audit log)
  - §21 Release Engineering (expand-contract, blue/green, canary, feature flags, N-1 compat)
  - §22 CQRS Discipline (CRUD default, CQRS только с measurable divergence)
  - §14 риски расширены: over-engineering, alloc regression, supply-chain, tenant data leak через interop
  - §15 расширен критикой v2.0 (15 новых правок)
- **v3.1 — 2026-05-24** — alignment с parent constitution `barbie/ENTITY.md`:
  - §0.1 — explicit boundary AX ↔ parent (таблица thematic ownership + правило "при конфликте побеждает родитель")
  - §2.5 — добавлена связь с TLA Entity §9 (TLA для intra-stack модулей, RFC/ADR для cross-stack)
  - §2.5 — добавлена связь с Delivery Standard §11 (PLAN/CHANGES/RISKS/VALIDATION/NEXT как извлечение из P1–P4)
  - §2.7 NO MAGIC — origin-cite на §11 Philosophy
  - §18 Security — origin-cite на §11 Security Mentality
  - §21 — разделён на §21.0 (VPS baseline из §6) и §21.1 (AX-расширения)
  - §23 — расширенный parent reference с указанием релевантных секций + ссылка на прародительский ES и CLAUDE.md §M
  - §15 — критика v3.0 (3 правки v3.0→v3.1)
  - В `barbie/ENTITY.md §0` добавлена строка AX (единственная правка parent constitution)
- **v3.2 — 2026-05-24** — миграционный playbook (gap между §12 conditions и реальным процессом):
  - §12.6 Migration Playbook — 4 фазы (Pilot 2w / Stage 2w / GA 4w / Retire 1w), per-module TLA Entity mapping, день-за-днём workflow template, Caddy cutover recipe, < 5 мин rollback procedure, 12 validation gates с owner'ами и доказательствами, observability с конкретными PromQL/Sentry tag'ами/alert'ами, 10 анти-паттернов
  - §12.7 Pilot Module Selection — scoring matrix 7 критериев × 9 кандидатов; verdict: `cms_pages` 1-й, `media` 2-й, `appointments` только 3-й; categorical exclusion для `clients` (PII), `tenants` (cascade), `chat` (state)
  - §13 — обновлены ссылки на §12.6/§12.7; убран hard-code «appointments» из дефолта пилота; добавлен 7-й сценарий «pre-conditions не выполнены»
  - §15 — критика v3.1 (5 правок v3.1→v3.2: №31–35)
  - §23 — добавлена ссылка на `docs/interop/CONTRACTS/` и `ops/caddy/` как артефакты cutover'а
  - Версия документа bump'нута 3.1 → 3.2
- **v3.3 — 2026-05-24** — session history infrastructure:
  - §24 Session History — автоматическая запись каждой Claude Code-сессии в `barbie/ax/istori/` через PowerShell-hooks (SessionStart / UserPromptSubmit / Stop) в `.claude/settings.json`; previous-session context инжектится автоматически при старте; uncurated raw layer под curated `memory/` и `SESSION_LOG.md`
  - §23 — добавлены `istori/`, `.claude/hooks/`, `.claude/settings.json` в Карта документов
  - `.gitignore` создан в `barbie/ax/` для исключения `istori/*` (кроме `.gitkeep`) и `.claude/.istori-current` из git

---

*Этот документ — спецификация альтернативного стека для NAS. Не миграционный mandate. Production-таргет остаётся `barbie/SITE1/` до соответствия §13 Decision Criteria. При любом конфликте этого документа с `barbie/ENTITY.md` — побеждает родитель (§0.1).*
