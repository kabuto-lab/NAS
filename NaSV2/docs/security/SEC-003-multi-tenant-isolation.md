# SEC-003 — Multi-tenant isolation (resolution, propagation, RLS)

- **Status:** Specification (forward-looking — first real repo + RLS policy land M2)
- **Date:** 2026-06-18
- **Owner:** AX•ARCHITECT (Sentinel + Adversary co-signed at first PgPostRepository day)
- **References:** ENTITY §3.3 (RLS mandatory), §3.5 (single-RTT `set_config`), §15 (multi-tenancy contract); `crates/tenant/src/`; `crates/common/src/ids.rs::TenantId`; [SEC-002](SEC-002-capability-surface.md); FM-002 + FM-010 in `memory/sentinel_init.md`

## §1 — TenantId is the boundary

`TenantId` is a `newtype` over `Ulid` (`crates/common/src/ids.rs`).
Per ENTITY §15:

- **No function** that touches tenant-scoped data may compile
  without either a `TenantId` parameter or access to a
  `TenantContext` via `axum::Extension`.
- **Tenant resolution happens once per request**
  (`crates/tenant/src/middleware.rs`) and the resolved
  `TenantContext` is inserted into `request.extensions()`.
- Downstream layers (handlers, use cases, repos) read the
  context — never re-resolve it.

## §2 — Resolution path (M1 W3 D2 middleware)

```text
incoming request
   │
   ▼
nas2_tenant::resolve_tenant   (axum middleware)
   │                          ├─→ TenantResolver port
   ▼                          │   (M1: InMemoryTenantResolver
extensions::insert(TenantContext)│    M2: PgTenantResolver + moka LRU)
   │
   ▼
handler                 ─→  use case (TenantId + CapabilitySet)
   │                            │
   ▼                            ▼
extract Tenant Extension    PostRepository (Arc<dyn>)
                                │
                                ▼
                          [M2 W6+] sqlx with set_config(tenant_id)
                                │
                                ▼
                          [M2 W6+] RLS policy filters rows
```

### Failure modes at each layer

| Layer | If TenantContext is missing | If resolver fails |
|---|---|---|
| Middleware | Resolver returns `Err(AppError::Validation)` → 400 | Resolver returns `Err(AppError::NotFound)` → 404 |
| Handler | `Extension<TenantContext>` extractor fails → axum returns 500 | n/a |
| Use case | Cannot construct call — signature requires `&TenantContext` | n/a |
| Repo | Cannot bind `set_config` without tenant — SQL fails | DB layer denies |

The handler-level 500 on missing TenantContext is deliberate — it
catches the (theoretically impossible) case where a route was
wired without the middleware. It is loud rather than silent.

## §3 — `set_config('app.tenant_id', $1, true)` contract (ENTITY §3.5)

The RLS-binding mechanism is `set_config` with the `is_local =
true` flag — the value is reset at transaction end. The contract
is:

- **Single-RTT.** ENTITY §3.5 forbids the naive two-statement
  shape (`SET LOCAL ...; SELECT ...`). Repos MUST use either a
  CTE that wraps `set_config(...)`, a prepared statement that
  returns rows after the set_config call, or a single batched
  `BEGIN ... COMMIT` with both statements.
- **`is_local = true`.** The third argument MUST be `true` so
  PgBouncer (transaction mode, ENTITY §3.4.1) cannot leak the
  setting across pooled connections.
- **Repo helper.** The future `with_tenant(pool, ctx, fn)`
  helper (M2 W6 D2) encodes the pattern so individual repo
  methods can't get it wrong.

The first real repo (M2 W6 D2 `PgPostRepository`) is the moment
the helper lands; until then, this section is a contract spec,
not evidence of implementation.

## §4 — RLS policies (forward-looking — first migration M2 W1)

ENTITY §3.3 mandates RLS for every tenant-scoped table. M2 W1's
migrations (`0002_sites.sql` … `0006_posts_rls.sql`) introduce
the first policies. Pattern:

```sql
ALTER TABLE posts ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON posts
  USING (tenant_id = current_setting('app.tenant_id', true)::uuid);
```

`current_setting('app.tenant_id', true)` returns NULL outside a
`set_config` window; the comparison `tenant_id = NULL` is always
NULL (≠ TRUE), so missing `set_config` → zero rows returned, not
all rows. Fail-closed by construction.

## §5 — Threat model

| ID    | Threat                                                | Mitigation                                                                            |
|-------|-------------------------------------------------------|---------------------------------------------------------------------------------------|
| T-3a  | Cross-tenant data leak via handler missing extension  | Handler 500s on missing `Extension<TenantContext>` (this doc §2).                     |
| T-3b  | Repo forgets `set_config(...)`                        | RLS returns zero rows (this doc §4); FM-002 proptest VAL-005 catches in CI M2 W1 D5.  |
| T-3c  | Shared cache key collision across tenants             | Cache key composition includes tenant_id (`tenant + slug + capability_hash`, §3.9.1). |
| T-3d  | PgBouncer session-mode leak of `set_config`           | `is_local = true` resets per transaction; pool-mode validator (VAL-002) refuses boot. |
| T-3e  | Resolver cache poisoning (tenant A's TenantContext served to tenant B's request) | M2 PgTenantResolver uses moka LRU keyed on signed cookie / Host header — not on user-controlled state. |
| T-3f  | Tenant deletion leaves orphan rows                    | M11+ tenant-purge runner ran in `worker_pool`, paginated by tenant_id; audit table records every drop. |
| T-3g  | Test fixture leaks production tenant_id (CI noise)    | Test fixtures construct random ULIDs; no production-tenant constants in `crates/*/tests/`. |

## §6 — Operability hooks

- `tracing::Span` carries `tenant_id` as a field on every request
  span (wired in the M1 W3 middleware).
- `metrics::counter!("tenant.resolve.failure", "reason" => ...)`
  to be wired in M2 W3 alongside the real resolver.
- `pg_stat_activity.application_name` separates http_pool /
  worker_pool / admin_pool (ADR-003); per-pool tenant breakdowns
  are derivable from log correlation.

## §7 — Open follow-ups

1. **PgTenantResolver + moka LRU** — replace `InMemoryTenantResolver`
   stub. Implements `with_tenant` helper. M2 W3 per `memory/orchestrator_init.md`
   open positions.
2. **First RLS migration** — `migrations/0006_posts_rls.sql`, M2 W1 D3.
3. **VAL-005 RLS proptest** — 1 K rounds proving cross-tenant
   isolation. M2 W1 D5 per `memory/forgemaster_init.md` bench slots.
4. **Tenant-purge runner** — M11+ scope; design ADR opens at M10
   RETRO.

## References

- `crates/common/src/ids.rs::TenantId` — newtype boundary
- `crates/tenant/src/` — middleware + resolver port
- ENTITY §3.3 (RLS), §3.5 (single-RTT), §15 (multi-tenancy contract)
- [SEC-002](SEC-002-capability-surface.md) — capability surface companion
- `memory/sentinel_init.md` FM-002 (RLS bypass), FM-010 (cache poison)
