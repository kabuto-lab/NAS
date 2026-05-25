# WEEK-08 · Jul 13–17, 2026 (M2 W4)

> Theme: **CRUD finish + CSRF + ADR-009 revisions + RETRO**
> Goals: G4 finish + G5

## Daily slots

| Date | Day | File | Scope summary | Est h |
|---|---|---|---|---:|
| 2026-07-13 | Mon | `daily/2026-07-13.md` | `POST /api/v1/pages` (admin create) — body validation via garde + slug uniqueness + capability `cms.page.draft` | 4 |
| 2026-07-14 | Tue | `daily/2026-07-14.md` | `PATCH /api/v1/pages/:slug` — partial update via PATCH semantics + status FSM gate + `cms.page.publish` for status change | 4 |
| 2026-07-15 | Wed | `daily/2026-07-15.md` | `DELETE /api/v1/pages/:slug` (soft via PostStatus::Archived) + CSRF nonce middleware + double-submit token | 4 |
| 2026-07-16 | Thu | `daily/2026-07-16.md` | ADR-009 revisions/autosaves; RFC-005 csrf-strategy; RFC-006 jwt-auth-foundation; verify all 5 endpoints + 401/403/400 matrix tests | 4 |
| 2026-07-17 | Fri | `daily/2026-07-17.md` | RETRO-2026-07 + `avtonom-month-bootstrap-2026-07.md` generation (uses MONTH-SKELETON-03 as seed) | 3 |

## Dependencies satisfied entering W8

- JwtVerifier + jwt_middleware live (W7)
- PgPostRepository wired + integration-tested (W6)
- `GET /api/v1/pages` list endpoint shipped (W7 D5)
- `POST /api/v1/auth/login` shipped (W7 D4)

## Definition of done (W8)

- [ ] All 5 Posts CRUD endpoints live: GET single + list, POST,
      PATCH, DELETE
- [ ] Each endpoint covered by ≥ 3 oneshot integration tests
      (happy + at least 2 error paths)
- [ ] CSRF middleware: double-submit token on POST/PATCH/DELETE
      admin endpoints; cookie + header validation; cross-origin
      POST without token → 403
- [ ] `cms.page.draft` capability gate on POST; `cms.page.publish`
      on PATCH for status transitions; `manage.site` on DELETE
- [ ] ADR-009 written: revisions table shape + autosave 30s
      contract + revert semantics (impl deferred to M3 G5)
- [ ] RFC-005 written: CSRF strategy (SameSite=Strict + double-submit
      + same-origin assumption)
- [ ] RFC-006 written: JWT auth foundation (key management,
      rotation TODO for next month)
- [ ] RETRO-2026-07.md + `avtonom-month-bootstrap-2026-07.md`
      committed
- [ ] Cumulative WP parity ~14%

## Carry-over (W8 → M3)

Any unfinished item moves into M3 W1 prompt's `## CARRY-OVER from
prior month` block. RFC orphans (RFC-002, PLAN-002) remain
deliberately deferred — track in next-month AUDIT A3.

## W8-specific risks

| Risk | Mitigation |
|---|---|
| `PATCH /api/v1/pages/:slug` partial update payload ambiguity | use `Option<T>` fields + `serde(skip_serializing_if = "Option::is_none")`; missing field = unchanged |
| Slug uniqueness race on POST | DB unique constraint catches; map sqlx error 23505 → AppError::Conflict |
| CSRF cookie + double-submit token feels janky for SPAs | header `X-CSRF-Token` mirrors cookie; SPA reads cookie via cookie-parser, echoes in header; tests confirm |
| Deletion via Archive doesn't free slug for reuse | tomorrow's reuse: slug-versioning ADR; M2 closes Archive as terminal |
| RETRO drifts into "fix it now" mode | reserve W8 D5 strictly for RETRO + bootstrap; no feature edits |
