---
name: sentinel-init
description: Sentinel initial state at 2026-05-26 Adoption Pass — open failure modes, incident hypotheses, rollback readiness
metadata:
  type: project
---

# SENTINEL — Init Dossier (2026-05-26 Adoption Pass)

## Open failure modes (across roadmap)

| ID | Failure mode | Detector | Recovery | First-affected day |
|---|---|---|---|---|
| FM-001 | PgBouncer pool-mode drift (anything but `transaction`) | startup validator + `/health/pool` | refuse boot; alert; restore PgBouncer config | every day |
| FM-002 | RLS bypass on a tenant-scoped table | VAL-005 proptest | revoke the broken role; patch policy | M2 W1 D5+ |
| FM-003 | libvips OOM on large image (> 100 MB) | `RequestBodyLimitLayer` + worker memory cap | reject upload at boundary; alert if recurrent | M3 W3 D1+ |
| FM-004 | pgmq vt re-delivery duplicates variant rows | idempotency key on `(media_id, variant_kind, theme_hash)` + `ON CONFLICT DO NOTHING` | none required (idempotent); detect via duplicate rate metric | M3 W2 D4+ |
| FM-005 | Autosave bumps `posts.updated_at` and breaks cache | invariant test in VAL-008 | revert revision write; patch handler | M3 W4 D3+ |
| FM-006 | JWT replay (token reuse after logout) | `jti` cache in moka (1 h TTL) | invalidate session; force re-login | M2 W3 D2+ |
| FM-007 | CSRF double-submit token leak via `Referer` | SameSite=Strict + HttpOnly handling | invalidate cookie; force re-login | M2 W4 D3+ |
| FM-008 | Comment XSS via insufficient ammonia allow-list | stricter allow-list at M6; fuzz tests | re-sanitize affected comments | M6 W1 D4+ |
| FM-009 | Plugin WASM escape (sandbox bypass) | wasmtime capability tokens; Adversary M9 | revoke plugin; quarantine tenant data | M9 W1 D3+ |
| FM-010 | Edge cache poison (stale tenant data served to wrong user) | `tenant + slug + capability_hash` key composition; cache-purge fan-out | manual purge by tenant; incident postmortem | M10 W3 D1+ |
| FM-011 | Editor substrate (Leptos 0.7 → 1.0) breaks mid-year | dependabot watch on `leptos` major release | substrate-port spike at next RETRO | M3 W3+ (substrate adoption) |
| FM-012 | Plugin-contributed EditableBlock XSS via plugin block render | ammonia sanitize-on-write at plugin-published-content boundary | adversary review at ADR-014 | M9 W3+ |

## Threat surfaces (current)

| Surface | Status |
|---|---|
| Public HTTP endpoints | only `/health/*` (3 routes) — trivial surface |
| Auth | not yet implemented (lands M2 W3) |
| Admin UI | not yet implemented (lands M5 W4+) |
| Plugin host | not yet implemented (lands M9) |
| Upload endpoint | not yet implemented (lands M3 W3 D1) |
| Edge worker | not yet implemented (lands M12 W4) |

## Rollback readiness

- `migrations/` is forward-only (ENTITY §17). Rollback requires explicit `ROLLBACK-NNN-*.md` doc per migration.
- No production deploy yet — all rollbacks are local.
- Blue-green binary deployment plan locks in M12.

## Observability gaps

| Gap | Plan |
|---|---|
| No `tracing` spans on health endpoints | acceptable; not on hot path |
| No metrics on pool acquisition latency | add at M2 W2 (first content endpoint) |
| No alert rules wired (Prometheus → Alertmanager) | M11 W4 (production prep) |
| No `dhat` heap snapshots scheduled | M12 W3 (PGO + BOLT pass needs them) |

## Constitutional position

The Sentinel is forbidden from emitting "no concerns" verdicts (`CONSTITUTION.md §2.3`). On a day with genuinely no Sentinel-class surface, the verdict is `Council: SENTINEL skipped — reason: <…>` — explicit, never implicit.
