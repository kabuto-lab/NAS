# WEEK-07 · Jul 6–10, 2026 (M2 W3)

> Theme: **JwtVerifier + caps extraction + first CRUD endpoint**
> Goals: G3 (JWT) + G4 start

## Daily slots

| Date | Day | File | Scope summary | Est h |
|---|---|---|---|---:|
| 2026-07-06 | Mon | `daily/2026-07-06.md` | `nas2-application::auth::JwtVerifier` trait + `HmacJwtVerifier` impl; HMAC-SHA256 key from env; iat/exp validation | 4 |
| 2026-07-07 | Tue | `daily/2026-07-07.md` | `nas2-presentation::auth::jwt_middleware` — replaces W4 D1 thread-local stub; cookie + Bearer paths; jti replay cache (moka 1h) | 4 |
| 2026-07-08 | Wed | `daily/2026-07-08.md` | `PgUserRepository::find_by_email` + `find_by_id` + `verify_password` (argon2 from workspace); integration tests | 4 |
| 2026-07-09 | Thu | `daily/2026-07-09.md` | `POST /api/v1/auth/login` — body { email, password } → JWT (HttpOnly + SameSite=Strict cookie + body for clients); rate-limit via tower-governor | 4 |
| 2026-07-10 | Fri | `daily/2026-07-10.md` | `GET /api/v1/pages` (paginated list) + capability gate + oneshot integration tests for 200/401/403 | 4 |

## Dependencies satisfied entering W7

- PgPostRepository real and wired in AppState (W6)
- 5 migrations applied
- User aggregate + Email VO + Role + Capability shipped in M1
- argon2 in workspace deps

## Definition of done (W7)

- [ ] `JwtVerifier::verify(token) -> Result<JwtClaims, AppError>`
  with claims `{ sub: UserId, tenant: TenantId, caps: Vec<String>,
  exp, iat, jti }`
- [ ] `jwt_middleware`: Bearer + cookie; injects `CapabilitySet` into
  `request.extensions()`; missing→401, expired→401, bad sig→401,
  replay→401 (jti seen within window)
- [ ] `caps::extract_caps_for_today()` retired or rerouted to the
  new extension; `extract_caps_for_test()` retained for unit tests
- [ ] `POST /api/v1/auth/login` returns 200 + Set-Cookie + body;
  401 on wrong creds; 429 on rate-limit
- [ ] `GET /api/v1/pages` returns paginated list of Published posts
  for the resolved tenant + site; 401 if no JWT; 403 if missing
  `cms.page.read`
- [ ] All test paths use real DB (#[ignore]) or MockRepo (fast)
- [ ] Capability-coverage marker present on every new handler

## Carry-over (W7 → W8)

If JWT replay-cache design churns past W7 D3 → moka size + TTL
become a follow-up in next-month bootstrap; ship single-key path
this month, key rotation = next-month ADR.

## W7-specific risks

| Risk | Mitigation |
|---|---|
| JWT key compromise → tokens replayable indefinitely | env-loaded key + short exp (1h default) + jti replay window |
| Cookie + Bearer dual paths cause confusion | document precedence: Bearer > Cookie; tests cover both independently |
| `tower-governor` doesn't tag rate-limits per tenant | M2 ships per-IP only; per-tenant rate-limit = M3 issue |
| `argon2` verification is slow → login latency spike | argon2 params tuned (m=19456, t=2, p=1) — ~100 ms acceptable; cache verification result is NOT done (security) |
