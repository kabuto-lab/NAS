# ROLLBACK · AX v0.1.0 · cms_pages pilot Phase A

**Версия:** v0.1.0 (initial Phase A pilot deploy)
**Trigger:** любая регрессия в pilot tenant (`imperiumspa`) — perf, contract,
isolation, 5xx errors.
**Target time:** **< 5 минут** от detect до revert (per `ENTITY.md §12.6.5`).
**Reversal cost:** **LOW** per `ADR-001`.

---

## Pre-rollback checks (≤ 30 sec)

1. Sentry: подтверждение что events приходят (а не silent fail)
2. Grafana `cms-cutover-watch` dashboard: confirm regression (не false-positive
   из network noise)
3. Caddy logs: проверка что requests идут на ax-server, не куда-то ещё

---

## Rollback procedure (executable steps)

### Step 1 — Caddy revert (≤ 5 sec)

В **main Caddyfile** на production VPS закомментировать `import cms_ax_pilots`:

```caddy
spa.me, *.spa.me {
    # import cms_ax_pilots             # ROLLBACK: disabled
    reverse_proxy site1-web:3011
}
```

Apply:

```bash
sudo caddy reload --config /etc/caddy/Caddyfile
# или
docker compose -f /etc/docker/compose.yml exec caddy caddy reload
```

Caddy graceful reload drains in-flight requests. New connections — на SITE1.

**Verify:**

```bash
curl -s -o /dev/null -w "%{http_code} %{header:X-Stack}\n" \
  -H "Host: imperiumspa.spa.me" \
  https://imperiumspa.spa.me/api/v1/cms/pages/public/by-slug/about

# Expected: 200 (или 404 если pilot tenant удалил page); X-Stack header
# отсутствует (SITE1 не выставляет такой header).
```

### Step 2 — Stop AX server (≤ 5 sec)

После Caddy переключения AX уже не получает traffic. Для cleanup:

```bash
sudo systemctl stop ax-server
# или
docker compose -f /etc/docker/compose.yml stop ax-server
```

### Step 3 — Optional: revert DB changes (≤ 60 sec)

**ВНИМАНИЕ:** В большинстве cases НЕ нужно — RLS POLICY + view не блокируют
SITE1 (BYPASSRLS на site1_admin_role preserves behavior).

Revert только если: подозреваете что RLS POLICY ломает SITE1 admin queries
(symptom: SITE1 admin endpoints начали отдавать 0 rows).

```sql
-- Run as superuser против production DB:
BEGIN;
    DROP POLICY IF EXISTS rls_cms_pages_tenant_isolation ON cms_pages;
    ALTER TABLE cms_pages DISABLE ROW LEVEL SECURITY;
    DROP VIEW IF EXISTS cms_pages_v_active;
    -- ROLES (site1_admin_role, ax_app_role) оставить — нулевой cost,
    -- DROP только если явный cleanup нужен.
COMMIT;
```

**Verify:**

```sql
SELECT relname, relrowsecurity FROM pg_class WHERE relname = 'cms_pages';
-- Expected: relrowsecurity = f

SELECT 1 FROM cms_pages LIMIT 1;
-- (from SITE1 connection) — Expected: 1 row
```

### Step 4 — Post-rollback verification (≤ 60 sec)

1. `curl` на 3-5 pilot tenant pages — все возвращают 200 от SITE1
2. Sentry: убедиться что новых AX-tagged events нет (stack=ax)
3. Grafana `cms-cutover-watch`: AX request rate → 0, SITE1 → recovered
4. Manual browser test pilot tenant home + 1 published page

---

## Communication

После rollback:

1. **Сразу** — internal slack/telegram: "rollback executed for cms_pages AX,
   reason: ..., timestamp: ..."
2. **В течение 30 минут** — incident note в `docs/releases/v0.1.0/INCIDENT-<date>.md`:
   - Trigger (Sentry alert, manual notice, etc.)
   - Detection timestamp
   - Rollback timestamp
   - User-facing impact (если был)
   - Root cause hypothesis
3. **В течение 24 часов** — post-mortem с reviewer'ом RFC-001:
   - Confirmed root cause
   - Что нужно fix'нуть перед next deploy attempt
   - Update VAL-001 если найден новый failure mode

---

## Acceptance criteria — rollback "successful"

- [ ] Caddy serving 0% requests на ax-server, 100% на site1-web для pilot host
- [ ] Sentry stack=ax events stream stopped (within 1 минута)
- [ ] User-facing impact = 0 (pilot tenant pages render как до cutover)
- [ ] AX server can be safely killed (no orphan tasks)
- [ ] DB schema state: RLS либо disabled либо preserved with no SITE1 disruption
- [ ] Caddy reload не вызвал 5xx burst (verify через logs)

---

## What NOT to do during rollback

- **НЕ** force-kill ax-server до Caddy reload (in-flight requests умрут с 502)
- **НЕ** делать `DROP ROLE site1_admin_role` — SITE1 PgBouncer pool на ней
- **НЕ** revert migration `0001_cms_pages_expand.sql` через ad-hoc SQL без
  записи в `docs/releases/v0.1.0/INCIDENT-<date>.md`
- **НЕ** restart AX и пытаться "ещё раз" без identification root cause
- **НЕ** изменять Caddyfile prod вручную без отслеживания в git
  (ops/caddy/Caddyfile.snippets/ должен match prod state)

---

## Related artifacts

- `ROLLBACK_DRILL.md` — timestamps подтверждающие drill < 5 min на stage
- `INCIDENT-<date>.md` — если rollback был triggered real incident'ом
- `DECISION.md` — initial Phase A → DECISION
- `ops/caddy/Caddyfile.snippets/cms-ax-pilots.caddy` — production routing snippet
- `migrations/0001_cms_pages_expand.sql` — schema delta (reversible)
- `ADR-001 §Reversal cost`
