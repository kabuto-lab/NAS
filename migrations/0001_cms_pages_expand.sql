-- Migration: 0001_cms_pages_expand.sql
-- Stage: AX Phase A pilot — cms_pages read-path
-- Owner: SITE1 (canonical writer для cms_pages таблицы; не изменяется этой миграцией)
-- AX role: read-only через cms_pages_v_active + RLS
--
-- References: ADR-001 §D2 (RLS implementation strategy), bridge/03 §2 schema delta.
-- This migration is EXPAND-ONLY: no DROP, no ALTER TYPE. Reversible (DROP VIEW +
-- DISABLE RLS + revert Caddy = < 5 min per ENTITY.md §12.6.5).
--
-- N-1 compat: SITE1 на старой схеме читает cms_pages без проблем — site1_admin_role
-- BYPASSRLS, view дополнительный, POLICY не влияет на SITE1 connections.

-- ─────────────────────────────────────────────────────────────────────────────
-- 1. Read-only view — published-only срез для AX
-- ─────────────────────────────────────────────────────────────────────────────

CREATE OR REPLACE VIEW cms_pages_v_active AS
SELECT
    id,
    tenant_id,
    slug,
    locale,
    title,
    body,
    status,
    meta_title,
    meta_description,
    cover_image_key,
    author_user_id,
    published_at,
    created_at,
    updated_at
FROM cms_pages
WHERE status = 'published';

COMMENT ON VIEW cms_pages_v_active IS
    'Published-only view of cms_pages. Used by AX read-path. RLS applies via base table policy.';

-- ─────────────────────────────────────────────────────────────────────────────
-- 2. RLS — enable on base table + policy
-- ─────────────────────────────────────────────────────────────────────────────

ALTER TABLE cms_pages ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS rls_cms_pages_tenant_isolation ON cms_pages;

CREATE POLICY rls_cms_pages_tenant_isolation
    ON cms_pages
    USING (tenant_id = current_setting('app.current_tenant_id', true)::uuid);

COMMENT ON POLICY rls_cms_pages_tenant_isolation ON cms_pages IS
    'Enforces tenant_id = current_setting(app.current_tenant_id). Requires SET LOCAL in transaction; PgBouncer must be in transaction pool mode.';

-- ─────────────────────────────────────────────────────────────────────────────
-- 3. Roles — separation between SITE1 (BYPASS) and AX (enforced)
-- ─────────────────────────────────────────────────────────────────────────────

-- SITE1 admin role — bypasses RLS to preserve current behavior bit-for-bit.
-- IF the role already exists in production, this is a no-op (CREATE ROLE IF EXISTS).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'site1_admin_role') THEN
        CREATE ROLE site1_admin_role NOLOGIN;
    END IF;
END
$$;

GRANT ALL ON cms_pages TO site1_admin_role;
ALTER ROLE site1_admin_role BYPASSRLS;

-- AX application role — RLS enforced (NO BYPASSRLS).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ax_app_role') THEN
        CREATE ROLE ax_app_role NOLOGIN;
    END IF;
END
$$;

GRANT SELECT ON cms_pages_v_active TO ax_app_role;
GRANT SELECT ON cms_pages TO ax_app_role;
-- DELIBERATELY NO `ALTER ROLE ax_app_role BYPASSRLS` — RLS must apply.

-- ─────────────────────────────────────────────────────────────────────────────
-- 4. Verification queries (run manually post-migration)
-- ─────────────────────────────────────────────────────────────────────────────
--
-- Verify RLS is enabled:
--   SELECT relname, relrowsecurity FROM pg_class WHERE relname = 'cms_pages';
--   -- expect: relrowsecurity = t
--
-- Verify POLICY exists:
--   SELECT polname FROM pg_policy WHERE polrelid = 'cms_pages'::regclass;
--   -- expect: rls_cms_pages_tenant_isolation
--
-- Verify roles:
--   SELECT rolname, rolbypassrls FROM pg_roles
--   WHERE rolname IN ('site1_admin_role', 'ax_app_role');
--   -- expect: site1_admin_role = t, ax_app_role = f
--
-- N-1 compat (run from SITE1 connection):
--   SELECT 1 FROM cms_pages LIMIT 1;
--   -- expect: returns a row (BYPASSRLS works)
--
-- Tenant isolation (run as ax_app_role):
--   BEGIN;
--   SET LOCAL app.current_tenant_id = '<tenant-uuid>';
--   SELECT id FROM cms_pages WHERE tenant_id = '<other-tenant-uuid>';
--   COMMIT;
--   -- expect: 0 rows (POLICY blocks even with explicit cross-tenant WHERE)

-- ─────────────────────────────────────────────────────────────────────────────
-- Rollback recipe (NOT executed automatically — manual on incident)
-- ─────────────────────────────────────────────────────────────────────────────
--
-- 1. Caddy: comment out `import cms_ax_pilots` + `caddy reload` (< 5s)
-- 2. systemd: `systemctl stop ax-server` (5s)
-- 3. SQL revert (if RLS interaction broken):
--      DROP POLICY rls_cms_pages_tenant_isolation ON cms_pages;
--      ALTER TABLE cms_pages DISABLE ROW LEVEL SECURITY;
--      DROP VIEW cms_pages_v_active;
--    -- ROLES kept (zero cost; only DROP if explicit cleanup needed)
