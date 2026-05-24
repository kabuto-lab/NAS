# ADR-002 · Auth architecture — shared JWT + capability extractor + RLS-enforced INSERT

| Field | Value |
|-------|-------|
| **Status** | Draft — pending sign-off |
| **Date** | 2026-05-25 |
| **Author** | Claude (AVTONOM session) |
| **Reviewers required** | user |
| **Phase** | P2 Architectural (`ENTITY.md §2.5`) |
| **Dependent on** | `RFC-002-auth-migration.md` sign-off |
| **Unblocks** | `PLAN-002-auth-impl.md`, `VAL-002-auth.md` |
| **Reversal cost** | **Low** (revert Caddy POST route + DROP table users_*; users table preserves data) |

---

## Context

RFC-002 фиксирует *что* мы делаем (auth foundation + 1 admin write endpoint). ADR-002 фиксирует *как* — architectural decisions с обоснованием отвергнутых альтернатив.

Three load-bearing facts directing architecture:

1. **SITE1 = canonical auth issuer для Phase A** — AX только verifier, не signer
2. **PgBouncer transaction mode уже required** для read-path with_tenant; write-path использует тот же pattern
3. **Capability storage = first-time design** — SITE1 капабилити attached к user через role table, но AX устанавливает свой schema (forward-compatible с SITE1 через shared user_id)

---

## Decision

**Принят: shared-secret HS256 JWT + capability join-table (user_roles + role_capabilities) + moka cache 60s + typed extractor RequireCapability<C> + RLS POLICY на INSERT в cms_pages.**

### D1 — JWT secret strategy: shared HS256 via env

```
JWT_SECRET=<same as SITE1 .env>
JWT_ALGORITHM=HS256
JWT_EXPIRES_IN=900  # 15 minutes, mirror SITE1
```

**Why:**
- HS256 (symmetric) — simplest для shared-secret cross-stack pattern
- AX = verifier only в Phase A; signer = SITE1 (NestJS @nestjs/jwt)
- RS256 (asymmetric) considered → deferred to Phase B when key rotation policy mature

**Rejected:** issuing tokens from AX (means duplicate login flow + drift risk).
**Rejected:** RS256 (extra ops: keypair generation, key distribution, rotation tooling).

### D2 — JWT claims structure: mirror SITE1

```rust
struct Claims {
    sub: String,        // user_id (Uuid as string)
    tenant_id: String,  // Uuid as string
    role: String,       // "admin" | "staff" | "client"
    kind: String,       // "platform" | "tenant"
    exp: i64,           // Unix timestamp
    iat: i64,           // Unix timestamp
}
```

**Why:** byte-for-byte cross-stack compat. SITE1-signed token verifies на AX без transformation.

Capabilities **NOT** embedded в JWT (rejected): кепабилити меняются чаще expiry → DB lookup + cache better для freshness.

### D3 — Capability storage: join tables (not array column)

```sql
CREATE TABLE roles (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    key          varchar(64) NOT NULL,
    display_name varchar(128) NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, key)
);

CREATE TABLE capabilities (
    key         varchar(128) PRIMARY KEY,  -- append-only registry
    description text,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE role_capabilities (
    role_id        uuid NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    capability_key varchar(128) NOT NULL REFERENCES capabilities(key),
    PRIMARY KEY (role_id, capability_key)
);

CREATE TABLE user_roles (
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id uuid NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, role_id)
);
```

**Why:**
- Flexible: add new capability → INSERT capabilities + INSERT role_capabilities; no migration на users table
- Bounded query: 1 JOIN cycle (users → user_roles → role_capabilities) per cache-miss
- Indexable: PRIMARY KEY на composite columns gives fast lookups
- Mirror'ит SITE1 RBAC pattern (NestJS `RolesService` joins на role_permissions)

**Rejected:** `users.capability_keys text[]` column — denormalized, hard to maintain registry consistency, ALTER TYPE при изменении capability_key painful.

**Rejected:** RBAC via separate `Casbin` / `oso` library — extra dep, overkill для Phase A scope.

### D4 — Capability cache: moka future, 60s TTL, 5000 capacity

```rust
type CapabilityCache = moka::future::Cache<UserId, CapabilitySet>;

let cache = CapabilityCache::builder()
    .max_capacity(5000)
    .time_to_live(std::time::Duration::from_secs(60))
    .build();
```

**Why:**
- 60s TTL = balance между freshness и DB load (per ENTITY §15 cache budget)
- 5000 capacity covers expected active admin user count Phase A (single-tenant + low concurrency)
- Stale-while-revalidate semantics через moka built-in
- Cache invalidation manual endpoint — Phase B; для now revocation eventually consistent <60s

**Rejected:** Redis cache — extra infra, single-instance AX в Phase A не выигрывает от distributed cache.
**Rejected:** No cache — DB roundtrip on every authorized request unsuitable для admin UI с ~5 requests/page.

### D5 — Capability extractor: typed `RequireCapability<C>` (where applicable) + helper function fallback

```rust
// Preferred form: typed extractor (compile-time guarantee)
async fn create_page(
    State(state): State<AppState>,
    RequireTenant(ctx): RequireTenant,
    RequireCapability(_): RequireCapability<{ Capability::PostsCreate }>,
    Json(payload): Json<CreatePageRequest>,
) -> Result<Json<CmsPageResponse>, AppError> { ... }
```

**Fallback** если const generics не работают чисто для Capability enum (которое может содержать non-Copy variants):

```rust
// Runtime helper
async fn create_page(
    State(state): State<AppState>,
    RequireTenant(ctx): RequireTenant,
    AuthenticatedUser(user): AuthenticatedUser,
    Json(payload): Json<CreatePageRequest>,
) -> Result<Json<CmsPageResponse>, AppError> {
    state.cap_resolver.require(&user, Capability::PostsCreate).await?;
    ...
}
```

**Decision:** start with runtime helper (simpler, works today); revisit typed extractor если const generics on enum stabilizes.

**Rejected:** Macro-based `#[require_capability(PostsCreate)]` attribute — adds proc-macro dep, complicates IDE intellisense; not justified for Phase A handful of endpoints.

### D6 — Resolve capabilities lifecycle: lazy in handler, not in middleware

**Decision:** Capability resolution NOT в middleware (auth middleware just decodes JWT + injects `AuthenticatedUser`). Capability resolve invoked в RequireCapability extractor or helper inside handler.

**Why:**
- Avoids DB roundtrip для requests которые не требуют capability (e.g. /health)
- Middleware stays cheap (HMAC verify only, no IO)
- Per-handler explicit declaration of required capability — visible в code review

**Rejected:** Resolve в `auth_middleware` upfront → wastes DB lookups on no-capability requests.
**Rejected:** Resolve в `tenant_resolver` middleware → couples concerns (tenant ≠ user authz).

### D7 — RLS POLICY на INSERT: WITH CHECK clause

```sql
DROP POLICY IF EXISTS rls_cms_pages_tenant_isolation ON cms_pages;
CREATE POLICY rls_cms_pages_tenant_isolation
    ON cms_pages
    USING (tenant_id = current_setting('app.current_tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('app.current_tenant_id', true)::uuid);
```

**Why:**
- `USING` clause = read-side filter (already в 0001 migration)
- `WITH CHECK` = write-side validation (Postgres rejects INSERT/UPDATE если new row violates)
- Hard wall: даже forged `tenant_id` в payload → SQL error 23514, surfaced as AppError::TenantMismatch

**Rejected:** Application-only check (current SITE1 pattern) — single layer, bypassable if app code has bug.

### D8 — Users table: RLS enabled, tenant-scoped

```sql
ALTER TABLE users ADD COLUMN IF NOT EXISTS tenant_id uuid REFERENCES tenants(id);
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_hash varchar(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS display_name varchar(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS status varchar(20) NOT NULL DEFAULT 'active';

-- Backfill tenant_id для existing seeded users (will be done в migration with safe default)
UPDATE users SET tenant_id = '11111111-1111-1111-1111-111111111111' WHERE tenant_id IS NULL;
ALTER TABLE users ALTER COLUMN tenant_id SET NOT NULL;

ALTER TABLE users ENABLE ROW LEVEL SECURITY;
CREATE POLICY users_tenant_isolation ON users
    USING (tenant_id = current_setting('app.current_tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('app.current_tenant_id', true)::uuid);
GRANT SELECT, INSERT, UPDATE ON users TO ax_app_role;
```

**Caveat:** PgUserRepository нужен read access user'а до tenant resolve — pattern:
- `find_by_id_for_auth(user_id)` использует **service connection** (BYPASSRLS via separate role) или query without `SET LOCAL`
- В Phase A: simpler — use `site1_admin_role` (BYPASSRLS) для auth resolve; switch to `ax_app_role` for tenant-scoped operations later
- Alternative: store tenant_id в JWT claims → set tenant ctx → then query user with RLS active

**Decision:** option B (use tenant_id from JWT to set ctx before user query). Mirrors SITE1 token-trust pattern.

### D9 — AppError extensions for auth

```rust
enum AppError {
    // existing variants ...
    InvalidToken,           // 401 — JWT decode/verify failed
    TokenExpired,           // 401 — exp в прошлом
    MissingCapability(Capability),  // 403 — user lacks specific capability
}
```

JSON codes:
- `INVALID_TOKEN` (401)
- `TOKEN_EXPIRED` (401)
- `MISSING_CAPABILITY` (403) with `{ code, required: "posts:create" }`

### D10 — Capability namespace + naming

String format `<resource>:<action>[:<modifier>]`:
- `posts:create`
- `posts:edit`
- `posts:publish`
- `posts:delete`

Rust enum (Phase A scope — 4 variants, well under §2.8 budget):
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    PostsCreate,
    PostsEdit,
    PostsPublish,
    PostsDelete,
}
```

**Rejected:** sub-enum nesting (Posts(PostsCap)) — premature given 4 variants. Switch когда total > 12.

### D11 — Test fixture: extend TestContext::setup

Add `TestContext::setup_with_auth(...)` returning `(TestContext, AuthFixture)` where `AuthFixture` contains seeded `(user_id, role_id, capabilities, jwt_token)`. Reuse в integration tests.

---

## Reversal cost

| Component | Reversal | Time |
|-----------|----------|------|
| Caddy POST route | comment out + `caddy reload` | < 5s |
| AX server | stop service | < 5s |
| Migration 0002 | DROP TABLE user_roles, role_capabilities, capabilities, roles; ALTER users DROP COLUMN | < 1 min |
| User data | preserved if rollback prior to widespread admin use | n/a |

**Total: < 5 min для full rollback to read-path state.**

---

## Dependencies on previous decisions

- ADR-001 D1 (four-layer crates) — Capability + AuthenticatedUser типы в common; UserRepository trait в application; PgUserRepository + JwtVerifier в infrastructure
- ADR-001 D2 (RLS strategy) — extend `WITH CHECK` to cover INSERT
- ADR-001 D6 (caching) — moka future, same pattern as tenant cache
- ENTITY.md §7 — RBAC + auth chapter

---

## Sign-off

**By Author:** content complete. Ready for PLAN-002 + implementation.

**By Reviewer:** [ ] pending review
