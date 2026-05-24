# ROLLBACK_DRILL · v0.1.0 · timestamped exercise

**Status:** **PENDING** — drill ещё не выполнен.

Per `ENTITY.md §12.6.6 G11`: rollback drill < 5 min — обязательный validation
gate перед Phase A → Phase B переходом. Этот файл коммитится с timestamps
после реального drill'а в stage окружении.

---

## Drill protocol

### Setup (выполняется однократно перед drill)

1. Stage VPS с identical config к production (PgBouncer transaction mode, etc.)
2. AX server задеплоен и serving pilot tenant requests
3. Caddy snippet `cms-ax-pilots.caddy` active
4. Continuous load generator: `oha -z 600s -c 100 -H "X-Tenant-Slug: imperiumspa" \
   https://imperiumspa.stage.spa.me/api/v1/cms/pages/public/by-slug/about`
5. Open: Sentry, Grafana dashboard, Caddy logs tail

### Trigger (intentional)

Один из (имитация realistic regression):

- (A) Kill ax-server: `sudo systemctl stop ax-server` — Caddy health probe должен
  detect down upstream в ≤ 30s
- (B) Inject 50% 5xx errors через middleware feature flag (если есть)
- (C) Simulate latency spike: `tc qdisc add dev eth0 root netem delay 500ms`

### Execute (timed — main measurement)

| Step | Action | Operator | Target time |
|------|--------|----------|-------------|
| 1 | Detect via Grafana alert / Sentry / monitoring | On-call | T+0:00 |
| 2 | Acknowledge alert, начало drill | On-call | T+0:00 |
| 3 | SSH в production-equivalent VPS, edit main Caddyfile (comment out `import cms_ax_pilots`) | On-call | T+0:30 |
| 4 | `caddy reload` | On-call | T+1:00 |
| 5 | `curl` verification — 5 pilot pages return 200 from SITE1 (X-Stack header absent) | On-call | T+1:30 |
| 6 | Sentry: confirm AX event stream stopped | On-call | T+2:00 |
| 7 | `systemctl stop ax-server` | On-call | T+2:30 |
| 8 | Post-drill verify oha load test continues serving (0 5xx через SITE1) | On-call | T+3:00 |
| 9 | Drill complete, document timestamps | On-call | T+3:30 |

**Target:** complete steps 1-9 в **≤ 5:00** wall-clock time.

---

## Drill results

### Drill #1 (PENDING)

| Step | Timestamp | Notes |
|------|-----------|-------|
| 1 — Detect | _pending_ | _trigger used: (A/B/C)_ |
| 2 — Ack | _pending_ | |
| 3 — Caddyfile edit | _pending_ | |
| 4 — caddy reload | _pending_ | |
| 5 — curl verify | _pending_ | _Outputs from `curl -w "%{http_code}\n"`: 200, 200, 200, 200, 200_ |
| 6 — Sentry stopped | _pending_ | |
| 7 — ax-server stop | _pending_ | |
| 8 — load test recovery | _pending_ | |
| 9 — Complete | _pending_ | |

**Total time:** _T+m:ss_
**Status:** ⏳ pending / ✅ pass (≤ 5 min) / ❌ fail (> 5 min)
**Reviewer:** _user_
**Sign-off date:** _pending_

### Drill #2 (recommended — re-run после Phase A → B gate)

(template — populate when needed)

---

## Failure modes observed

(populated после реальных drill'ов; включает любые surprise behaviors)

- _none yet_

---

## Required acceptance для VAL-001 gate G11

- [ ] At least 1 successful drill < 5:00
- [ ] Drill #1 timestamps documented above
- [ ] No 5xx errors during Caddy reload (verified в logs)
- [ ] Sentry confirms event stream stop
- [ ] Post-drill load test on SITE1 baseline serves cleanly

Без этого ROLLBACK_DRILL.md gate G11 в VAL-001 = ❌ red и **Phase A → B gate
blocked**.
