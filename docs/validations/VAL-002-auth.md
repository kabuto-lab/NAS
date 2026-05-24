# VAL-002 · Auth migration verification criteria

| Field | Value |
|-------|-------|
| **Status** | Active |
| **Date** | 2026-05-25 |
| **Author** | Claude (AVTONOM session) |
| **Phase** | P4 Verification (`ENTITY.md §2.5`) |
| **Dependent on** | PLAN-002 step completion |

---

## Functional (F-series, 9 criteria)

| ID | Criterion | Test | Pass condition |
|----|-----------|------|----------------|
| F1 | POST `/api/v1/cms/pages/admin` с valid JWT + tenant header + PostsCreate cap → 201 Created | `auth_test::t_create_page_with_valid_jwt_returns_201_and_inserts_row` | response status 201; response body includes `id`, `slug`, `status="draft"`; row visible в `SELECT FROM cms_pages WHERE id = response.id` |
| F2 | No Authorization header → 401 | `auth_test::t_no_jwt_returns_401` | status 401; body `{ "code": "NOT_AUTHENTICATED" }` |
| F3 | Authorization header не Bearer scheme → 401 | inline assertion в t_no_jwt | status 401; body INVALID_TOKEN или NOT_AUTHENTICATED |
| F4 | JWT signed with wrong secret → 401 | `t_forged_jwt_wrong_secret_returns_401` | status 401; body `{ "code": "INVALID_TOKEN" }` |
| F5 | JWT expired (exp в прошлом) → 401 | `t_expired_jwt_returns_401` | status 401; body `{ "code": "TOKEN_EXPIRED" }` (или INVALID_TOKEN — both acceptable) |
| F6 | JWT malformed (не три части base64) → 401 | `t_malformed_jwt_returns_401` | status 401; body INVALID_TOKEN |
| F7 | Valid JWT но user без PostsCreate cap → 403 | `t_user_without_capability_returns_403` | status 403; body `{ "code": "MISSING_CAPABILITY", "required": "posts:create" }` |
| F8 | Valid JWT для user другого tenant → 403 или 404 | `t_jwt_for_other_tenant_returns_403_or_404` | status 403 (TENANT_MISMATCH) — tenant from JWT не matches X-Tenant-Slug; либо 404 если tenant-resolve fails first |
| F9 | RLS rejects spoofed tenant_id (если ever attempted via raw SQL test) | `t_rls_blocks_spoofed_tenant_id_in_payload` | RLS POLICY raises 42501 / 23514; AppError::Database wraps |

---

## Isolation (I-series, 4 criteria)

| ID | Criterion | Test | Pass condition |
|----|-----------|------|----------------|
| I1 | Inserted page tenant_id = current_setting (always) | DB query post-insert | row.tenant_id = JWT.tenant_id |
| I2 | Cross-tenant token + correct header → 403 (no row created for either tenant) | F8 + DB inspection | count(*) WHERE tenant_id = either-tenant = 0 |
| I3 | Same slug в разных tenant'ах — оба success | extension test (manual smoke) | 2 INSERTs не conflict (unique index = (tenant_id, slug, locale)) |
| I4 | Existing read-path (cms_pages_test.rs) — без regression | `cargo test --test cms_pages_test -- --ignored` | 6/6 green |

---

## Performance (P-series, 3 criteria — Phase A scope)

| ID | Criterion | Measurement | Pass condition |
|----|-----------|-------------|----------------|
| P1 | JWT verify latency | `tracing` span "auth.verify_jwt" duration | p50 < 1ms, p99 < 5ms (HMAC HS256 = constant-time) |
| P2 | Capability resolve cache hit latency | span "auth.capability.resolve" with cache_hit=true | p50 < 100µs |
| P3 | Capability resolve cache miss latency | span "auth.capability.resolve" with cache_hit=false | p99 < 50ms (single JOIN query) |

P4-P13 deferred to Phase A2 benchmark task (oha + dhat).

---

## Allocation (A-series, deferred)

A1-A6 deferred — dhat profiling — Phase B.

---

## Quality (Q-series, 5 criteria)

| ID | Criterion | Check | Pass condition |
|----|-----------|-------|----------------|
| Q1 | clippy zero warnings | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| Q2 | rustfmt clean | `cargo fmt --check` | exit 0 |
| Q3 | All unit tests green | `cargo test --workspace --lib` | green |
| Q4 | Architecture boundaries hold | `cargo xtask architecture-check` | exit 0 |
| Q5 | cargo-deny green | `cargo deny check` | exit 0 (no new advisories from jsonwebtoken dep) |

---

## Operational (O-series, 4 criteria)

| ID | Criterion | Check | Pass condition |
|----|-----------|-------|----------------|
| O1 | ax-server starts с new binary, no panics | journal/stdout inspection | server logs "listening on 0.0.0.0:7710" within 1s |
| O2 | `/health` still 200 | `curl http://localhost:7710/health` | 200 + JSON `{ status: "ok" }` |
| O3 | Existing read-path `/api/v1/cms/pages/public/by-slug/home` still 200 | manual curl | 200 + JSON page |
| O4 | New POST endpoint reachable | `curl -X POST` with valid JWT | 201 |

---

## Security (S-series, 3 criteria)

| ID | Criterion | Test | Pass condition |
|----|-----------|------|----------------|
| Sec1 | JWT_SECRET не logged | grep tracing JSON output | no occurrence of secret string |
| Sec2 | Password hashes не returned via API | check response JSON schema | no `password_hash` field anywhere |
| Sec3 | JWT not stored в DB logs / response | check both | no token leakage в structured logs |

---

## Documentation (D-series, 3 criteria)

| ID | Criterion | Location | Pass condition |
|----|-----------|----------|----------------|
| D1 | RFC-002 sign-off section | docs/rfc/RFC-002-auth-migration.md | exists, marked pending |
| D2 | ADR-002 decision table | docs/adr/ADR-002-auth-architecture.md | D1-D11 documented |
| D3 | Curl smoke commands в SESSION_LOG | barbie/SESSION_LOG.md | reproducible commands listed |

---

## Definition of Done (consolidated)

- [ ] All F1-F9 tests pass (Phase 5 done)
- [ ] All I1-I4 tests pass (regression check via cms_pages_test)
- [ ] Q1-Q5 quality gates green
- [ ] O1-O4 operational checks green
- [ ] Sec1-Sec3 security checks green
- [ ] D1-D3 documentation в place
- [ ] 6-10 commits with `AI-Assisted: Claude Code` trailer
- [ ] SESSION_LOG.md updated с final report

---

## Test infrastructure additions

Extend `crates/infrastructure/tests/common/mod.rs` (или create) with:
```rust
pub struct AuthFixture {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub role_id: Uuid,
    pub capabilities: Vec<String>,
    pub jwt: String,
}

impl TestContext {
    pub async fn with_auth(self, capabilities: &[&str]) -> (Self, AuthFixture) { ... }
}
```

Helper `sign_test_jwt(user_id, tenant_id, role, secret, ttl_seconds)` for test convenience.
