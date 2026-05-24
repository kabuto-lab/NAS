-- AX dev setup: baseline schema + migration 0001 + seed tenant/page
-- One-shot bootstrap для `cargo run -p ax-server`

-- 1. pgcrypto for gen_random_uuid()
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- 2. Baseline schema (subset SITE1 0000)
CREATE TABLE IF NOT EXISTS tenants (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          varchar(64) NOT NULL UNIQUE,
    name          varchar(255) NOT NULL,
    status        varchar(20) NOT NULL DEFAULT 'active',
    contact_email varchar(320) NOT NULL DEFAULT 'noreply@example.com',
    timezone      varchar(64) NOT NULL DEFAULT 'UTC',
    locale        varchar(8) NOT NULL DEFAULT 'ru',
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS users (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email      varchar(320) NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS cms_pages (
    id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    slug             varchar(255) NOT NULL,
    locale           varchar(8) NOT NULL DEFAULT 'ru',
    title            varchar(500) NOT NULL,
    body             jsonb NOT NULL DEFAULT '[]'::jsonb,
    status           varchar(20) NOT NULL DEFAULT 'draft',
    meta_title       varchar(255),
    meta_description text,
    cover_image_key  varchar(500),
    author_user_id   uuid REFERENCES users(id) ON DELETE SET NULL,
    published_at     timestamptz,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS cms_pages_tenant_slug_locale_uniq
    ON cms_pages (tenant_id, slug, locale);
CREATE INDEX IF NOT EXISTS cms_pages_tenant_status_idx
    ON cms_pages (tenant_id, status);
CREATE INDEX IF NOT EXISTS cms_pages_tenant_published_idx
    ON cms_pages (tenant_id, published_at DESC NULLS LAST)
    WHERE status = 'published';

-- 3. Migration 0001 — view + RLS + roles (mirrors barbie/ax/migrations/0001_cms_pages_expand.sql)
CREATE OR REPLACE VIEW cms_pages_v_active
WITH (security_invoker = true) AS
SELECT id, tenant_id, slug, locale, title, body, status,
       meta_title, meta_description, cover_image_key,
       author_user_id, published_at, created_at, updated_at
FROM cms_pages
WHERE status = 'published';

ALTER TABLE cms_pages ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS rls_cms_pages_tenant_isolation ON cms_pages;
CREATE POLICY rls_cms_pages_tenant_isolation
    ON cms_pages
    USING (tenant_id = current_setting('app.current_tenant_id', true)::uuid);

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'site1_admin_role') THEN
        CREATE ROLE site1_admin_role NOLOGIN;
    END IF;
END $$;
GRANT ALL ON cms_pages TO site1_admin_role;
ALTER ROLE site1_admin_role BYPASSRLS;

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ax_app_role') THEN
        CREATE ROLE ax_app_role NOLOGIN;
    END IF;
END $$;
GRANT SELECT ON cms_pages_v_active TO ax_app_role;
GRANT SELECT ON cms_pages TO ax_app_role;
GRANT SELECT ON tenants TO ax_app_role;
ALTER ROLE ax_app_role WITH LOGIN PASSWORD 'ax_dev_pw';

-- 4. Seed: 1 tenant + 1 published page
INSERT INTO tenants (id, slug, name, status)
VALUES ('11111111-1111-1111-1111-111111111111', 'imperiumspa', 'Imperium Spa (AX dev)', 'active')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO cms_pages (tenant_id, slug, locale, title, status, published_at, body)
VALUES (
    '11111111-1111-1111-1111-111111111111',
    'home',
    'ru',
    'Welcome to AX',
    'published',
    now(),
    '[{"type":"hero","data":{"title":"Hello from AX","subtitle":"Rust-powered CMS"}}]'::jsonb
)
ON CONFLICT (tenant_id, slug, locale) DO NOTHING;

-- 5. Migration 0002 — auth foundation (users tenant_id + roles/capabilities/RBAC)
-- Idempotent — safe to re-run. See barbie/ax/migrations/0002_users_roles_capabilities.sql
-- for the canonical version (this block must stay in sync — diff after edits to 0002).

-- 5.1 users table extensions
ALTER TABLE users ADD COLUMN IF NOT EXISTS tenant_id uuid REFERENCES tenants(id) ON DELETE CASCADE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_hash varchar(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS display_name varchar(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS status varchar(20) NOT NULL DEFAULT 'active';
ALTER TABLE users ADD COLUMN IF NOT EXISTS updated_at timestamptz NOT NULL DEFAULT now();
CREATE INDEX IF NOT EXISTS users_tenant_idx ON users(tenant_id);

UPDATE users SET tenant_id = '11111111-1111-1111-1111-111111111111'
WHERE tenant_id IS NULL;

ALTER TABLE users ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS users_tenant_isolation ON users;
CREATE POLICY users_tenant_isolation ON users
    USING (tenant_id = current_setting('app.current_tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('app.current_tenant_id', true)::uuid);

-- 5.2 roles + capabilities + joins
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
CREATE POLICY roles_tenant_isolation ON roles
    USING (tenant_id = current_setting('app.current_tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('app.current_tenant_id', true)::uuid);

CREATE TABLE IF NOT EXISTS capabilities (
    key         varchar(128) PRIMARY KEY,
    description text,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS role_capabilities (
    role_id        uuid NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    capability_key varchar(128) NOT NULL REFERENCES capabilities(key),
    PRIMARY KEY (role_id, capability_key)
);
CREATE INDEX IF NOT EXISTS role_capabilities_cap_idx ON role_capabilities(capability_key);

CREATE TABLE IF NOT EXISTS user_roles (
    user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id    uuid NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    granted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, role_id)
);
CREATE INDEX IF NOT EXISTS user_roles_role_idx ON user_roles(role_id);

-- 5.3 cms_pages POLICY: add WITH CHECK explicitly
ALTER POLICY rls_cms_pages_tenant_isolation ON cms_pages
    WITH CHECK (tenant_id = current_setting('app.current_tenant_id', true)::uuid);

-- 5.4 GRANTs
GRANT SELECT, INSERT, UPDATE ON cms_pages TO ax_app_role;
GRANT SELECT, INSERT, UPDATE ON users TO ax_app_role;
GRANT SELECT ON tenants TO ax_app_role;
GRANT SELECT ON roles TO ax_app_role;
GRANT SELECT ON capabilities TO ax_app_role;
GRANT SELECT ON role_capabilities TO ax_app_role;
GRANT SELECT ON user_roles TO ax_app_role;
GRANT ALL ON roles, capabilities, role_capabilities, user_roles TO site1_admin_role;

-- 5.5 Capability registry seed (matches Capability enum in ax-common)
INSERT INTO capabilities (key, description) VALUES
    ('posts:create',  'Create new draft pages/posts in cms_pages'),
    ('posts:edit',    'Update existing pages/posts'),
    ('posts:publish', 'Publish draft pages to public'),
    ('posts:delete',  'Soft-delete (archive) pages')
ON CONFLICT (key) DO NOTHING;

-- 6. Dev seed: 1 admin user + 1 admin role with all 4 caps в imperiumspa tenant
-- Password hash = argon2 of 'Admin123!ChangeMe' (precomputed; do NOT use in prod).
-- Generated with: argon2 'Admin123!ChangeMe' -e (defaults).
-- Verification: argon2-cli + 'Admin123!ChangeMe' against this hash should succeed.

INSERT INTO users (id, email, tenant_id, password_hash, display_name, status)
VALUES (
    '22222222-2222-2222-2222-222222222222',
    'admin@imperiumspa.dev',
    '11111111-1111-1111-1111-111111111111',
    '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHR0ZXN0c2FsdA$bnzGo3aIc8FmYpf/ujM2VdAU8sxxC2vM6vk7RUC/SXk',
    'AX Dev Admin',
    'active'
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO roles (id, tenant_id, key, display_name)
VALUES (
    '33333333-3333-3333-3333-333333333333',
    '11111111-1111-1111-1111-111111111111',
    'admin',
    'Administrator'
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO role_capabilities (role_id, capability_key) VALUES
    ('33333333-3333-3333-3333-333333333333', 'posts:create'),
    ('33333333-3333-3333-3333-333333333333', 'posts:edit'),
    ('33333333-3333-3333-3333-333333333333', 'posts:publish'),
    ('33333333-3333-3333-3333-333333333333', 'posts:delete')
ON CONFLICT (role_id, capability_key) DO NOTHING;

INSERT INTO user_roles (user_id, role_id) VALUES
    ('22222222-2222-2222-2222-222222222222', '33333333-3333-3333-3333-333333333333')
ON CONFLICT (user_id, role_id) DO NOTHING;

-- 7. Verify
SELECT 'tenants' as "table", count(*) FROM tenants
UNION ALL SELECT 'users', count(*) FROM users
UNION ALL SELECT 'roles', count(*) FROM roles
UNION ALL SELECT 'capabilities', count(*) FROM capabilities
UNION ALL SELECT 'role_capabilities', count(*) FROM role_capabilities
UNION ALL SELECT 'user_roles', count(*) FROM user_roles
UNION ALL SELECT 'cms_pages', count(*) FROM cms_pages
UNION ALL SELECT 'cms_pages_v_active', count(*) FROM cms_pages_v_active;

-- 8. Dev test JWT (manual smoke):
-- Generate with: cargo xtask sign-jwt \
--     --user 22222222-2222-2222-2222-222222222222 \
--     --tenant 11111111-1111-1111-1111-111111111111 \
--     --role admin --secret "$JWT_SECRET" --ttl 3600
-- Then test:
-- curl -X POST http://localhost:7710/api/v1/cms/pages/admin \
--      -H "Authorization: Bearer $JWT" \
--      -H "X-Tenant-Slug: imperiumspa" \
--      -H "Content-Type: application/json" \
--      -d '{"slug":"first-draft","locale":"ru","title":"First Draft","body":[]}'
-- Expect: 201 Created + JSON body
