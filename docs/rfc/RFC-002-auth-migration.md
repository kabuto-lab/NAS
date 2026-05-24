# RFC-002 · Auth + admin write-path migration to AX (Rust)

| Field | Value |
|-------|-------|
| **Status** | Draft — pending sign-off |
| **Date** | 2026-05-25 |
| **Author** | Claude (AVTONOM session, AI assistant) |
| **Reviewers required** | user (single-person ops) |
| **Phase** | P1 Strategic (`ENTITY.md §2.5`) |
| **Gate inputs** | `RFC-001-cms_pages-migration.md` (Phase A pilot proven), `ADR-001-four-layer-rls.md`, `ENTITY.md §7` (Auth & RBAC) |
| **Gate outputs** | unblocks P2 `ADR-002-auth-architecture.md`, P3 `PLAN-002-auth-impl.md`, P4 `VAL-002-auth.md` |
| **Target module** | AX auth foundation + first admin write endpoint (`POST /api/v1/cms/pages/admin`) |
| **Migration phase** | A extension (Phase A read-path → Phase A write-path on same module, `ENTITY.md §12.6 Migration Playbook`) |

---

## Business reason

**Phase A pilot (cms_pages read) доказал паттерн: RLS + with_tenant + 21k fuzz cases zero leaks, 6/6 integration tests green, sub-100ms p95 latency. Сейчас расширяем pilot на write-path: тот же module, тот же tenant, добавляем authenticated admin endpoint.**

### B1 — Доказать admin-side migration pattern на минимально рискованном scope

Read-path был осторожным выбором (read SQL easier to audit, lower regression risk). Write-path требует трёх новых building blocks которые SITE1 не имеет в строгой форме:

- **JWT verification** (SITE1 имеет, но без cross-stack secret sharing)
- **Capability-based authorization** (SITE1 имеет через `@RequireCapabilities()` decorator; AX установит typed-extractor аналог)
- **RLS-enforced INSERT** (SITE1 не имеет RLS вообще — это будет первый INSERT в системе где DB-уровень тоже валидирует tenant)

Единственный write endpoint (create draft post) — narrowest possible blast radius. Update / publish / delete — отдельные follow-up RFC, после того как pattern доказан.

### B2 — Установить shared-secret JWT pattern для будущей co-existence

В Phase A pilot SITE1 и AX работают параллельно на двух разных портах. Admin UI пользователя живёт в SITE1 (Next.js, 5111). Чтобы admin UI мог делать POST к AX endpoint'у — JWT нужно подписать на SITE1 (login flow остался NestJS) и **верифицировать на AX**. Это значит: shared `JWT_SECRET` через .env, mirror'нутая структура claims.

Альтернатива (own login flow на AX) — больший scope, отсроченный выигрыш, дублирование password storage и risk drift. Phase A: AX = JWT verifier only. Phase B+: AX может стать canonical issuer когда mature.

### B3 — Hard wall на DB-уровне для admin operations впервые

SITE1 RBAC = application-level only. Bug в capability decorator → admin одного tenant'а потенциально создаст row для чужого (если payload содержит спуфнутый `tenantId`). RLS POLICY на INSERT'е делает это compile-time impossible на DB-уровне (postgres rejects если `tenant_id` в новой row != current_setting).

Это hard wall vs application-only check. Pattern, который если докажем здесь — стандарт для всех future write endpoint миграций.

### B4 — Capability registry kick-off

Domain типизация capability'и (Rust enum vs string в SITE1) даёт compile-time проверку `RequireCapability<PostsCreate>` — невозможно опечататься. Phase A scope: 1-2 variant'а (PostsCreate). Phase B+: расширение по мере добавления endpoint'ов. Registry table в DB — append-only, capability_key — varchar(128) PK.

---

## Goal (single, narrow)

**Доставить first authenticated admin write endpoint в AX:**

```
POST /api/v1/cms/pages/admin
Authorization: Bearer <JWT signed by SITE1 or test-helper, HS256, shared JWT_SECRET>
Content-Type: application/json
X-Tenant-Slug: imperiumspa
{ "slug": "draft-post", "locale": "ru", "title": "...", "body": [...] }

→ 201 Created
{ "id": "uuid", "slug": "draft-post", "status": "draft", ... }
```

С полным цепочкой проверок:
1. tenant_resolver middleware → TenantContext в extensions
2. auth middleware → JWT verify (HS256) → AuthenticatedUser в extensions
3. RequireCapability<PostsCreate> extractor → 403 если capability отсутствует у user'а
4. handler → INSERT через with_tenant(SET LOCAL app.current_tenant_id) → RLS POLICY на insert валидирует tenant_id
5. Response 201 + JSON

---

## Success criteria (P4 verifiable, see VAL-002)

| # | Criterion | Verification |
|---|-----------|--------------|
| S1 | Valid JWT + correct capability + correct tenant → 201 + row inserted | Integration test + manual curl |
| S2 | No JWT → 401 NOT_AUTHENTICATED | Integration test |
| S3 | Expired JWT → 401 INVALID_TOKEN | Integration test (sign с exp в прошлом) |
| S4 | Valid JWT, user without PostsCreate capability → 403 MISSING_CAPABILITY | Integration test (seed user with только PostsRead) |
| S5 | Valid JWT signed для другого tenant → 403 TENANT_MISMATCH | Integration test (tenant_id в JWT != X-Tenant-Slug resolved) |
| S6 | Malformed JWT → 401 INVALID_TOKEN | Integration test |
| S7 | Wrong JWT_SECRET (forged token) → 401 INVALID_TOKEN | Integration test |
| S8 | Capability resolution cached (moka, 60s TTL) — повторный request не делает DB roundtrip | Tracing span check / metric counter |
| S9 | RLS POLICY blocks insert если приложение forge'нет `tenant_id` в payload (spoofed) | Integration test (try to insert с tenant_id != current_setting) |

---

## Constraints

| # | Constraint | Source |
|---|------------|--------|
| C1 | **shared JWT_SECRET через .env** — SITE1 signs, AX verifies (HS256) | Phase A pattern, simplest cross-stack |
| C2 | **JWT claims mirror SITE1 structure** — `{ sub, tenant_id, role, kind, exp, iat }` | Cross-stack token compat |
| C3 | **HS256 not RS256** — symmetric shared secret, simpler ops | Phase A; RS256 = Phase B+ when key rotation needed |
| C4 | **No session management** — JWT only (mirror SITE1 access-token); refresh tokens out of scope | Phase B feature |
| C5 | **No login endpoint on AX** — login flow stays SITE1 NestJS | Migration is incremental |
| C6 | **Capability storage via join tables** (user_roles + role_capabilities) | ENTITY §7.3, flexibility for capability registry growth |
| C7 | **Capability cache: moka future 60s TTL, max 5000 user_ids** | ENTITY §15 cache budget |
| C8 | **Argon2 password hashing** (already в workspace) — но password verify сейчас не нужен (no login on AX) | OWASP, ENTITY §4.4 |
| C9 | **PgBouncer transaction mode** — required для `SET LOCAL` в insert flow | Same as Phase A read-path |
| C10 | **No `git push`** — local commits only, AVTONOM rule | CLAUDE.md §M |
| C11 | **No SITE1 edits** — AX migration parallel-running, isolation by scope | RFC-001 baseline |
| C12 | **Expand-only migration** (no DROP, no ALTER TYPE) | Phase A discipline |

---

## Risks + mitigations

| # | Risk | Mitigation |
|---|------|-----------|
| R1 | JWT_SECRET divergence между SITE1 .env и AX .env | Document в `_ax_dev_setup.sql` + `.env.example`; failure mode at startup if missing |
| R2 | SITE1 claims structure drift (поле renamed, тип изменён) | Mirror exactly текущую SITE1 structure; добавить cross-stack contract test в Phase B |
| R3 | Capability moka cache stale после permission revoke | 60s TTL bounded; manual invalidation endpoint — Phase B; for Phase A acceptable risk |
| R4 | RLS POLICY blocks legitimate insert (config bug) | Integration test S1 covers happy path; manual smoke verifies in dev |
| R5 | Argon2 verify in capability resolution hot path = latency spike | Capability resolve использует cache hit; argon2 только в login (not in AX Phase A) |
| R6 | jsonwebtoken crate vulnerability surface | Already in `cargo deny` audit; bump to latest 9.x |
| R7 | Cross-tenant token replay (steal SITE1 admin's token, use against AX) | Same security boundary as SITE1 (shared secret); document acceptable risk for Phase A |
| R8 | Capability enum growth past 12 variants (ENTITY §2.8 complexity budget) | Group via `Capability::Posts(PostsCapability)` sub-enums |

---

## Explicit out-of-scope

| # | Item | Defer to |
|---|------|----------|
| OOS1 | Login endpoint on AX (issue tokens) | Phase B+ when AX = canonical auth |
| OOS2 | Refresh token rotation | Phase B |
| OOS3 | Session management (tower-sessions) | Phase B |
| OOS4 | Password reset / forgot-password | Phase B |
| OOS5 | 2FA / TOTP | Phase B+ |
| OOS6 | OAuth (Google/GitHub) | Out of scope NAS overall |
| OOS7 | Admin UI changes (Next.js) | Separate frontend task, after AX endpoint live |
| OOS8 | Update/publish/delete cms_pages endpoints | Follow-up RFC-003 |
| OOS9 | Other modules (media, appointments) | Future RFC each |
| OOS10 | Capability invalidation/revocation endpoint | Phase B |
| OOS11 | Audit log of admin actions | Phase B |
| OOS12 | RS256 / key rotation | Phase B+ |
| OOS13 | Performance benchmarking (oha) on write path | Phase B; T16 separate work |
| OOS14 | Caddy production wiring for POST endpoint | User VPS task post sign-off |

---

## Why now (vs. defer)

- Read-path pilot stable (15958d8 — T15 fuzz green, 153f112 — integration tests green). Pattern proven.
- SITE1 admin UI стабильно; user может перенаправить POST endpoint через Caddy без disruption (Phase A canary).
- Next migration ступенька — без неё дальнейшие модули нельзя мигрировать (все write-paths требуют auth). Сейчас оптимальный momentum.

---

## Open questions (resolved as AI-Default per AVTONOM)

| # | Q | Resolution |
|---|---|------------|
| Q1 | RoleKey newtype или enum? | newtype `RoleKey(String)` — flexibility для tenant-specific roles (admin, editor, viewer) |
| Q2 | Capability enum closed vs open? | closed enum + sub-enum (Capability::Posts(PostsCap)) — type safety beats flexibility для Phase A |
| Q3 | Argon2 params — defaults or custom? | defaults — match OWASP recommendation, password verify не в hot path для AX |
| Q4 | JWT claims field name для capability? | NONE — capabilities resolved from DB по `sub` (user_id); JWT не несёт capability list |
| Q5 | Tenant mismatch — какой error? | reuse AppError::TenantMismatch (already есть, security event) |

---

## Sign-off

**By Author (Claude AVTONOM):** content complete per AVTONOM authority for P1 strategic doc. Ready for Phase 2 implementation.

**By Reviewer (user):** [ ] pending review

**Effective date:** 2026-05-25 на sign-off
