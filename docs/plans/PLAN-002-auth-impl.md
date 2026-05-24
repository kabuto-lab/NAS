# PLAN-002 · Auth migration implementation — execution steps

| Field | Value |
|-------|-------|
| **Status** | Active (executing in AVTONOM session 2026-05-25 00:27) |
| **Date** | 2026-05-25 |
| **Author** | Claude (AVTONOM session) |
| **Phase** | P3 Execution (`ENTITY.md §2.5`) |
| **Dependent on** | RFC-002 + ADR-002 sign-off (deferred to user; AVTONOM executes under delegated authority) |
| **Step count** | 36 (matches session-plan §Implementation order) |
| **Atomic commit-cell discipline** | per `ENTITY.md §2.7` |

---

## Phase 1 — Planning docs (~2.5h) — Steps 1-5

| Step | File | Action | Spine? |
|------|------|--------|--------|
| 1 | `docs/rfc/RFC-002-auth-migration.md` | new | non-spine |
| 2 | `docs/adr/ADR-002-auth-architecture.md` | new | non-spine |
| 3 | `docs/plans/PLAN-002-auth-impl.md` | new (this file) | non-spine |
| 4 | `docs/validations/VAL-002-auth.md` | new | non-spine |
| 5 | **commit** | `docs(ax): RFC-002 + ADR-002 + PLAN-002 + VAL-002 — auth migration foundation` | — |

**Done when:** 4 files exist; commit landed; no Cargo touched.

---

## Phase 2 — common + domain (~3h) — Steps 6-13

| Step | File | Action |
|------|------|--------|
| 6 | `crates/common/src/capability.rs` | new — Capability enum (4 variants Phase A) + key() str method |
| 7 | `crates/common/src/role.rs` | new — RoleKey(String) newtype |
| 8 | `crates/common/src/error.rs` | edit — add InvalidToken, TokenExpired, MissingCapability(Capability) variants + IntoResponse mapping |
| 9 | `crates/common/src/lib.rs` | edit — re-export capability, role |
| 10 | `crates/domain/src/user/mod.rs` | new — module decl |
| 11 | `crates/domain/src/user/aggregate.rs` | new — User aggregate + reconstitute() + invariants (email validation, status check) |
| 12 | unit tests | add — capability key roundtrip, RoleKey parse, User invariants violation cases |
| 13 | **commit** | `feat(ax/common,domain): Capability + Role + User domain types + AppError auth variants` |

**Spine touch warning:** `crates/common/src/error.rs` — non-spine per CLAUDE.md (Cargo.toml + schema = spine; source files = non-spine). Proceeds without ok.

---

## Phase 3 — migration + infrastructure (~2.5h) — Steps 14-17

| Step | File | Action |
|------|------|--------|
| 14 | `migrations/0002_users_roles_capabilities.sql` | new — additive schema, RLS, GRANTs, verification queries, rollback recipe |
| 15 | `crates/infrastructure/src/persistence/users_repo.rs` | new — PgUserRepository (find_by_id, find_by_id_with_capabilities) |
| 16 | `crates/infrastructure/src/auth/mod.rs` + `jwt.rs` | new — Claims struct + JwtVerifier (jsonwebtoken crate) |
| 16.5 | `crates/infrastructure/src/lib.rs` | edit — re-export auth, users_repo |
| 16.6 | `crates/infrastructure/Cargo.toml` | edit — add `jsonwebtoken = "9"` dep |
| 17 | **commit** | `feat(ax): migration 0002 users/roles/capabilities + PgUserRepository + JwtVerifier` |

**Spine warning:** `apps/server/src/main.rs` НЕ trog'аем здесь — Phase 4 wire-up.

---

## Phase 4 — application + presentation (~3.5h) — Steps 18-26

| Step | File | Action |
|------|------|--------|
| 18 | `crates/application/src/ports/user.rs` | new — UserRepository trait |
| 18.5 | `crates/application/src/ports/mod.rs` | edit — re-export |
| 19 | `crates/application/src/use_cases/auth/mod.rs` + `verify_jwt.rs` | new — VerifyJwt use case |
| 20 | `crates/application/src/use_cases/auth/resolve_capabilities.rs` | new — moka cache + CapabilityResolver |
| 20.5 | `crates/application/src/use_cases/mod.rs` | edit |
| 21 | `crates/presentation/src/middleware/auth.rs` | new — auth_middleware: extract Bearer → verify → inject AuthenticatedUser |
| 22 | `crates/presentation/src/middleware/capability.rs` | new — RequireCapability helper / extractor |
| 22.5 | `crates/presentation/src/middleware/mod.rs` | edit — re-export |
| 23 | `crates/presentation/src/api/cms_admin_handlers.rs` | new — POST handler create_page_admin |
| 23.5 | `crates/presentation/src/api/mod.rs` | edit — re-export |
| 24 | `crates/presentation/src/router.rs` | edit — wire POST route + auth middleware layer (after tenant_resolver) |
| 25 | `crates/presentation/src/app_state.rs` | edit — add jwt_verifier, user_repo, capability_resolver fields |
| 26 | **commit** | `feat(ax): auth middleware + capability extractor + POST cms_pages admin endpoint` |

**Spine warning:** `apps/server/src/main.rs` — minimal touch only (read JWT_SECRET env). If touched: document как AI-Default skip-spine in SESSION_LOG.

Also: `crates/application/src/use_cases/cms/mod.rs` not'oched.

---

## Phase 5 — integration tests (~2h) — Steps 27-28

| Step | File | Action |
|------|------|--------|
| 27 | `crates/infrastructure/tests/auth_test.rs` | new — 5+ #[tokio::test] #[ignore] integration tests, testcontainers + JWT |
| 28 | **commit** | `test(ax): integration tests — JWT auth + capability check + RLS-enforced INSERT` |

**Test list (S1-S9 from RFC §Success):**
- `t_create_page_with_valid_jwt_returns_201_and_inserts_row` (S1)
- `t_no_jwt_returns_401` (S2)
- `t_expired_jwt_returns_401` (S3)
- `t_user_without_capability_returns_403` (S4)
- `t_jwt_for_other_tenant_returns_403_or_404` (S5)
- `t_malformed_jwt_returns_401` (S6)
- `t_forged_jwt_wrong_secret_returns_401` (S7)
- `t_rls_blocks_spoofed_tenant_id_in_payload` (S9) — adversarial test

---

## Phase 6 — dev seed + smoke (~1h) — Steps 29-32

| Step | File | Action |
|------|------|--------|
| 29 | `_ax_dev_setup.sql` | edit — INSERT roles + capabilities + role_capabilities + user with argon2 hash + user_roles |
| 30 | `xtask/src/main.rs` (or new bin script) | new — `cargo xtask sign-jwt --user <uuid> --tenant <uuid>` |
| 31 | document curl smoke commands in SESSION_LOG | — |
| 32 | **commit** | `chore(ax): seed test admin user + role + capabilities + sign-jwt xtask` |

**Spine warning:** `_ax_dev_setup.sql` — non-spine but heavily used. Edit defensively (preserve existing seed).

---

## Phase 7 — Final report (~30m) — Steps 33-36

| Step | Action |
|------|--------|
| 33 | Kill bg ax-server (bwdg2jj95), rebuild release, restart |
| 34 | Verify endpoints respond: `curl /health`, `curl /api/v1/cms/pages/public/by-slug/home -H 'X-Tenant-Slug: imperiumspa'` |
| 35 | Update `barbie/SESSION_LOG.md` final report top-of-file |
| 36 | **commit (ES repo, not ax/):** `docs(barbie): SESSION_LOG — AVTONOM AX auth migration 2026-05-25` |

---

## Dependency graph

```
1 → 2 → 3 → 4 → 5 (commit)
                 ↓
6 → 7 → 8 → 9 → 10 → 11 → 12 → 13 (commit)
                                  ↓
14 → 15 → 16 → 17 (commit)
                  ↓
18 → 19 → 20 → 21 → 22 → 23 → 24 → 25 → 26 (commit)
                                            ↓
27 → 28 (commit)
       ↓
29 → 30 → 31 → 32 (commit)
                  ↓
33 → 34 → 35 → 36 (commit, ES repo)
```

---

## Validation gates per phase

| After | Run | Pass criterion |
|-------|-----|---------------|
| Step 13 | `cargo check --workspace` | exit 0 |
| Step 13 | `cargo test -p ax-common -p ax-domain --lib` | green |
| Step 17 | `cargo check --workspace` | exit 0 |
| Step 26 | `cargo check --workspace --all-targets` | exit 0 |
| Step 26 | `cargo clippy --workspace --all-targets -- -D warnings` | zero warnings |
| Step 26 | `cargo test --workspace --lib` | green |
| Step 28 | `cargo test --test auth_test -p ax-infrastructure -- --ignored` | 5+ green |
| Step 28 | `cargo test --test cms_pages_test -p ax-infrastructure -- --ignored` | 6/6 (regression check) |
| Step 32 | Manual curl POST → 201, psql `SELECT * FROM cms_pages WHERE status='draft'` → row visible | — |
| Step 35 | SESSION_LOG.md updated с full report | — |

---

## File touch list (consolidated)

**New files (16):**
- docs/rfc/RFC-002-auth-migration.md
- docs/adr/ADR-002-auth-architecture.md
- docs/plans/PLAN-002-auth-impl.md
- docs/validations/VAL-002-auth.md
- crates/common/src/capability.rs
- crates/common/src/role.rs
- crates/domain/src/user/mod.rs
- crates/domain/src/user/aggregate.rs
- crates/application/src/ports/user.rs
- crates/application/src/use_cases/auth/mod.rs
- crates/application/src/use_cases/auth/verify_jwt.rs
- crates/application/src/use_cases/auth/resolve_capabilities.rs
- crates/infrastructure/src/persistence/users_repo.rs
- crates/infrastructure/src/auth/mod.rs
- crates/infrastructure/src/auth/jwt.rs
- crates/presentation/src/middleware/auth.rs
- crates/presentation/src/middleware/capability.rs
- crates/presentation/src/api/cms_admin_handlers.rs
- migrations/0002_users_roles_capabilities.sql
- crates/infrastructure/tests/auth_test.rs

**Edited files (~10):**
- crates/common/src/lib.rs (re-export)
- crates/common/src/error.rs (add variants)
- crates/domain/src/lib.rs (re-export user mod)
- crates/application/src/ports/mod.rs (re-export user)
- crates/application/src/use_cases/mod.rs (re-export auth)
- crates/infrastructure/src/lib.rs (re-export auth, users_repo)
- crates/infrastructure/Cargo.toml (add jsonwebtoken)
- crates/presentation/src/middleware/mod.rs (re-export auth, capability)
- crates/presentation/src/api/mod.rs (re-export admin handlers)
- crates/presentation/src/router.rs (wire route)
- crates/presentation/src/app_state.rs (add fields)
- apps/server/src/main.rs (only if env wiring необходим for AppState::new)
- _ax_dev_setup.sql (seed)
- xtask/src/main.rs (sign-jwt subcommand)

---

## Sign-off

**By Author:** ready to execute Phase 2 immediately.
