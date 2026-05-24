-- Migration: 0002_users_roles_capabilities.sql
-- Stage: AX Phase A extension — auth foundation (users tenant_id + roles + capabilities)
-- Owner: AX (canonical for new tables roles / capabilities / role_capabilities / user_roles)
-- AX role: full RBAC + RLS-enforced INSERT on cms_pages
--
-- References: ADR-002 §D3 (capability storage), §D7 (WITH CHECK), §D8 (users RLS).
-- This migration is EXPAND-ONLY: ALTER ADD COLUMN, CREATE TABLE, ALTER POLICY (no DROP, no ALTER TYPE).
-- Reversal cost: DROP new tables + ALTER POLICY revert + DROP added columns < 1 min.
--
-- N-1 compat: SITE1 на старой schema продолжает работать — site1_admin_role BYPASSRLS,
-- new tables не используются SITE1 (никакого FK обратно). Existing cms_pages POLICY
-- USING-only фактически уже работает для INSERT (Postgres использует USING fallback
-- для WITH CHECK если последний не задан); explicit WITH CHECK добавляется для clarity.

-- ─────────────────────────────────────────────────────────────────────────────
-- 1. users table extensions (additive)
-- ─────────────────────────────────────────────────────────────────────────────

ALTER TABLE users ADD COLUMN IF NOT EXISTS tenant_id uuid REFERENCES tenants(id) ON DELETE CASCADE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_hash varchar(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS display_name varchar(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS status varchar(20) NOT NULL DEFAULT 'active';
ALTER TABLE users ADD COLUMN IF NOT EXISTS updated_at timestamptz NOT NULL DEFAULT now();

CREATE INDEX IF NOT EXISTS users_tenant_idx ON users(tenant_id);

-- Backfill tenant_id для existing seeded users to default tenant if exists.
-- Phase A: tenant_id остаётся nullable — Phase B tightens после full backfill discipline.
DO $$
DECLARE
    default_tenant uuid;
BEGIN
    SELECT id INTO default_tenant FROM tenants WHERE slug = 'imperiumspa' LIMIT 1;
    IF default_tenant IS NOT NULL THEN
        UPDATE users SET tenant_id = default_tenant WHERE tenant_id IS NULL;
    END IF;
END $$;

-- ─────────────────────────────────────────────────────────────────────────────
-- 2. users RLS — enable + POLICY (USING + WITH CHECK)
-- ─────────────────────────────────────────────────────────────────────────────

ALTER TABLE users ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS users_tenant_isolation ON users;
CREATE POLICY users_tenant_isolation
    ON users
    USING (tenant_id = current_setting('app.current_tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('app.current_tenant_id', true)::uuid);

COMMENT ON POLICY users_tenant_isolation ON users IS
    'Tenant-scoped read+write isolation. NULL tenant_id rows hidden from RLS (legacy backfill safety).';

-- ─────────────────────────────────────────────────────────────────────────────
-- 3. roles (tenant-scoped)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS roles (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    key          varchar(64) NOT NULL,
    display_name varchar(128) NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, key)
);

CREATE INDEX IF NOT EXISTS roles_tenant_idx ON roles(tenant_id);

ALTER TABLE roles ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS roles_tenant_isolation ON roles;
CREATE POLICY roles_tenant_isolation
    ON roles
    USING (tenant_id = current_setting('app.current_tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('app.current_tenant_id', true)::uuid);

-- ─────────────────────────────────────────────────────────────────────────────
-- 4. capabilities registry (global, append-only — no RLS)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS capabilities (
    key         varchar(128) PRIMARY KEY,
    description text,
    created_at  timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE capabilities IS
    'Global capability registry. Append-only (DROP capability_key requires explicit rollback). No RLS — read-only по сути.';

-- ─────────────────────────────────────────────────────────────────────────────
-- 5. role_capabilities (many-to-many через roles, RLS-gated через roles join)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS role_capabilities (
    role_id        uuid NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    capability_key varchar(128) NOT NULL REFERENCES capabilities(key),
    PRIMARY KEY (role_id, capability_key)
);

CREATE INDEX IF NOT EXISTS role_capabilities_cap_idx ON role_capabilities(capability_key);

-- No RLS on role_capabilities — join through roles table which has RLS.
-- ax_app_role SELECT-only access; JOIN naturally tenant-filtered via roles.

-- ─────────────────────────────────────────────────────────────────────────────
-- 6. user_roles (many-to-many)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS user_roles (
    user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id    uuid NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    granted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, role_id)
);

CREATE INDEX IF NOT EXISTS user_roles_role_idx ON user_roles(role_id);

-- ─────────────────────────────────────────────────────────────────────────────
-- 7. cms_pages POLICY — explicit WITH CHECK for write-path
-- ─────────────────────────────────────────────────────────────────────────────

-- Postgres использует USING как fallback для WITH CHECK если последний не задан,
-- то есть существующая POLICY уже фактически блокирует cross-tenant INSERT.
-- Explicit WITH CHECK добавляется для clarity + защиты от accidental ALTER POLICY,
-- которое может потерять fallback behavior.
ALTER POLICY rls_cms_pages_tenant_isolation ON cms_pages
    WITH CHECK (tenant_id = current_setting('app.current_tenant_id', true)::uuid);

-- ─────────────────────────────────────────────────────────────────────────────
-- 8. GRANTs — ax_app_role gains INSERT/UPDATE on cms_pages + read on auth tables
-- ─────────────────────────────────────────────────────────────────────────────

GRANT SELECT, INSERT, UPDATE ON cms_pages TO ax_app_role;
GRANT SELECT, INSERT, UPDATE ON users TO ax_app_role;
GRANT SELECT ON roles TO ax_app_role;
GRANT SELECT ON capabilities TO ax_app_role;
GRANT SELECT ON role_capabilities TO ax_app_role;
GRANT SELECT ON user_roles TO ax_app_role;

-- site1_admin_role (BYPASSRLS) — full access для seed scripts + administrative ops
GRANT ALL ON roles TO site1_admin_role;
GRANT ALL ON capabilities TO site1_admin_role;
GRANT ALL ON role_capabilities TO site1_admin_role;
GRANT ALL ON user_roles TO site1_admin_role;

-- ─────────────────────────────────────────────────────────────────────────────
-- 9. Seed canonical capabilities (matches Capability enum in ax-common)
-- ─────────────────────────────────────────────────────────────────────────────

INSERT INTO capabilities (key, description) VALUES
    ('posts:create',  'Create new draft pages/posts in cms_pages'),
    ('posts:edit',    'Update existing pages/posts'),
    ('posts:publish', 'Publish draft pages to public'),
    ('posts:delete',  'Soft-delete (archive) pages')
ON CONFLICT (key) DO NOTHING;

-- ─────────────────────────────────────────────────────────────────────────────
-- 10. Verification queries (manual, post-migration)
-- ─────────────────────────────────────────────────────────────────────────────
--
-- RLS state:
--   SELECT relname, relrowsecurity FROM pg_class
--   WHERE relname IN ('users','roles','cms_pages','role_capabilities','user_roles');
--   -- expect: users=t, roles=t, cms_pages=t, role_capabilities=f, user_roles=f
--
-- Policies:
--   SELECT polname, polcmd, qual::text, with_check::text FROM pg_policy
--   WHERE polrelid::regclass::text IN ('users','roles','cms_pages');
--
-- Capabilities seeded:
--   SELECT key FROM capabilities ORDER BY key;
--   -- expect: posts:create, posts:delete, posts:edit, posts:publish
--
-- Tenant-isolated insert sanity (as ax_app_role):
--   BEGIN;
--   SELECT set_config('app.current_tenant_id', '<tenant_uuid>', true);
--   INSERT INTO cms_pages (tenant_id, slug, locale, title, status)
--     VALUES ('<different_tenant_uuid>', 't', 'ru', 'fake', 'draft');
--   -- expect: ERROR 42501 — new row violates row-level security policy
--   ROLLBACK;

-- ─────────────────────────────────────────────────────────────────────────────
-- Rollback recipe (NOT executed automatically — manual on incident)
-- ─────────────────────────────────────────────────────────────────────────────
--
-- 1. Caddy: comment out POST route + reload (< 5s)
-- 2. systemd: stop ax-server (< 5s)
-- 3. SQL revert (if RBAC interaction broken):
--      DROP TABLE IF EXISTS user_roles;
--      DROP TABLE IF EXISTS role_capabilities;
--      DROP TABLE IF EXISTS capabilities;
--      DROP TABLE IF EXISTS roles;
--      DROP POLICY IF EXISTS users_tenant_isolation ON users;
--      ALTER TABLE users DISABLE ROW LEVEL SECURITY;
--      -- Leave column additions + cms_pages POLICY WITH CHECK (cost-neutral)
