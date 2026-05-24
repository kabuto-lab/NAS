//! Integration tests для auth + admin POST endpoint.
//!
//! Coverage targets (VAL-002 F1-F9 + I-series):
//! - F1: valid JWT + capability → 201 + row inserted
//! - F2: no JWT → 401
//! - F5: expired JWT → 401 TOKEN_EXPIRED
//! - F6: malformed JWT → 401
//! - F4/F7: forged JWT wrong secret → 401
//! - F7 (S4): user без capability → 403
//! - F8 (S5): JWT для другого tenant → 403
//! - F9 (S9): RLS blocks spoofed tenant_id в payload (direct SQL adversarial)
//! - I4: existing read-path не сломался (smoke via /health)
//!
//! Запуск: `cargo test --test auth_test -p ax-presentation -- --ignored`.
//! Требует Docker daemon (testcontainers).

#![allow(clippy::cognitive_complexity, clippy::unused_self)]

use ax_common::TenantId;
use ax_infrastructure::Claims;
use ax_presentation::AppState;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::Value;
use sqlx::PgPool;
use testcontainers::{core::ImageExt, runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::postgres::Postgres;
use tower::ServiceExt;
use uuid::Uuid;

const JWT_SECRET: &str = "ax-test-secret-32-bytes-min-padding-xx";
const TENANT_A_SLUG: &str = "tenant-a";
const TENANT_B_SLUG: &str = "tenant-b";

#[allow(dead_code)]
struct TestContext {
    _container: ContainerAsync<Postgres>,
    admin_pool: PgPool,
    pool: PgPool,
    state: AppState,
    tenant_a: Uuid,
    tenant_b: Uuid,
    admin_a_user: Uuid,
    admin_a_role: Uuid,
    no_cap_user: Uuid,
}

#[allow(dead_code)]
impl TestContext {
    async fn setup() -> Self {
        let container = Postgres::default()
            .with_tag("16-alpine")
            .start()
            .await
            .expect("postgres container start (Docker required)");
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let admin_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

        let admin_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(&admin_url)
            .await
            .expect("admin pool");

        apply_baseline_schema(&admin_pool).await;

        // Apply migration 0001 (RLS POLICY + roles for cms_pages)
        let m1 = include_str!("../../../migrations/0001_cms_pages_expand.sql");
        sqlx::raw_sql(m1).execute(&admin_pool).await.expect("apply 0001");

        // Apply migration 0002 (users/roles/capabilities + RBAC + WITH CHECK)
        let m2 = include_str!("../../../migrations/0002_users_roles_capabilities.sql");
        sqlx::raw_sql(m2).execute(&admin_pool).await.expect("apply 0002");

        // Allow LOGIN на ax_app_role
        sqlx::raw_sql("ALTER ROLE ax_app_role WITH LOGIN PASSWORD 'ax_test_pwd'")
            .execute(&admin_pool)
            .await
            .expect("ALTER ROLE LOGIN");

        // Seed 2 tenants
        let tenant_a = Uuid::new_v4();
        let tenant_b = Uuid::new_v4();
        seed_tenant(&admin_pool, tenant_a, TENANT_A_SLUG).await;
        seed_tenant(&admin_pool, tenant_b, TENANT_B_SLUG).await;

        // Seed admin user + role + capabilities in tenant_a
        let admin_a_user = Uuid::new_v4();
        let admin_a_role = Uuid::new_v4();
        seed_user(&admin_pool, admin_a_user, tenant_a, "admin-a@example.com").await;
        seed_role(&admin_pool, admin_a_role, tenant_a, "admin", "Admin").await;
        for cap_key in ["posts:create", "posts:edit", "posts:publish", "posts:delete"] {
            seed_role_capability(&admin_pool, admin_a_role, cap_key).await;
        }
        seed_user_role(&admin_pool, admin_a_user, admin_a_role).await;

        // Seed user без capabilities (for 403 test) in tenant_a
        let no_cap_user = Uuid::new_v4();
        seed_user(&admin_pool, no_cap_user, tenant_a, "nocap-a@example.com").await;

        // App pool (RLS-enforced)
        let app_url = format!("postgres://ax_app_role:ax_test_pwd@{host}:{port}/postgres");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(&app_url)
            .await
            .expect("app pool");

        let state = AppState::new(pool.clone(), JWT_SECRET);

        Self {
            _container: container,
            admin_pool,
            pool,
            state,
            tenant_a,
            tenant_b,
            admin_a_user,
            admin_a_role,
            no_cap_user,
        }
    }

    fn sign_jwt(&self, user_id: Uuid, tenant_id: Uuid, role: &str, ttl_sec: i64) -> String {
        sign_jwt_with_secret(JWT_SECRET, user_id, tenant_id, role, ttl_sec)
    }
}

fn sign_jwt_with_secret(
    secret: &str,
    user_id: Uuid,
    tenant_id: Uuid,
    role: &str,
    ttl_sec: i64,
) -> String {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        role: role.to_owned(),
        kind: "tenant".to_owned(),
        iat: now,
        exp: now + ttl_sec,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("sign jwt")
}

async fn apply_baseline_schema(pool: &PgPool) {
    sqlx::raw_sql(
        r"
        CREATE EXTENSION IF NOT EXISTS pgcrypto;

        CREATE TABLE IF NOT EXISTS tenants (
            id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
            slug        varchar(64) NOT NULL UNIQUE,
            name        varchar(255) NOT NULL,
            status      varchar(20) NOT NULL DEFAULT 'active',
            contact_email varchar(320) NOT NULL DEFAULT 'noreply@example.com',
            timezone    varchar(64) NOT NULL DEFAULT 'UTC',
            locale      varchar(8) NOT NULL DEFAULT 'ru',
            created_at  timestamptz NOT NULL DEFAULT now(),
            updated_at  timestamptz NOT NULL DEFAULT now()
        );

        CREATE TABLE IF NOT EXISTS users (
            id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
            email       varchar(320) NOT NULL UNIQUE,
            created_at  timestamptz NOT NULL DEFAULT now()
        );

        CREATE TABLE IF NOT EXISTS cms_pages (
            id                  uuid PRIMARY KEY DEFAULT gen_random_uuid(),
            tenant_id           uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            slug                varchar(255) NOT NULL,
            locale              varchar(8) NOT NULL DEFAULT 'ru',
            title               varchar(500) NOT NULL,
            body                jsonb NOT NULL DEFAULT '[]'::jsonb,
            status              varchar(20) NOT NULL DEFAULT 'draft',
            meta_title          varchar(255),
            meta_description    text,
            cover_image_key     varchar(500),
            author_user_id      uuid REFERENCES users(id) ON DELETE SET NULL,
            published_at        timestamptz,
            created_at          timestamptz NOT NULL DEFAULT now(),
            updated_at          timestamptz NOT NULL DEFAULT now()
        );

        CREATE UNIQUE INDEX IF NOT EXISTS cms_pages_tenant_slug_locale_uniq
            ON cms_pages (tenant_id, slug, locale);
        ",
    )
    .execute(pool)
    .await
    .expect("baseline schema");
}

async fn seed_tenant(pool: &PgPool, id: Uuid, slug: &str) {
    sqlx::query("INSERT INTO tenants (id, slug, name, status) VALUES ($1, $2, $3, 'active')")
        .bind(id)
        .bind(slug)
        .bind(format!("Test {slug}"))
        .execute(pool)
        .await
        .expect("seed tenant");
}

async fn seed_user(pool: &PgPool, id: Uuid, tenant_id: Uuid, email: &str) {
    sqlx::query(
        "INSERT INTO users (id, email, tenant_id, status) VALUES ($1, $2, $3, 'active')",
    )
    .bind(id)
    .bind(email)
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("seed user");
}

async fn seed_role(pool: &PgPool, id: Uuid, tenant_id: Uuid, key: &str, display: &str) {
    sqlx::query("INSERT INTO roles (id, tenant_id, key, display_name) VALUES ($1, $2, $3, $4)")
        .bind(id)
        .bind(tenant_id)
        .bind(key)
        .bind(display)
        .execute(pool)
        .await
        .expect("seed role");
}

async fn seed_role_capability(pool: &PgPool, role_id: Uuid, capability_key: &str) {
    sqlx::query(
        "INSERT INTO role_capabilities (role_id, capability_key) VALUES ($1, $2)",
    )
    .bind(role_id)
    .bind(capability_key)
    .execute(pool)
    .await
    .expect("seed role_capability");
}

async fn seed_user_role(pool: &PgPool, user_id: Uuid, role_id: Uuid) {
    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
        .bind(user_id)
        .bind(role_id)
        .execute(pool)
        .await
        .expect("seed user_role");
}

fn build_post_body() -> Value {
    serde_json::json!({
        "slug": "test-draft-page",
        "locale": "ru",
        "title": "Test Draft",
        "body": [],
        "metaTitle": "META",
        "metaDescription": null,
        "coverImageKey": null
    })
}

async fn read_body_json(resp: axum::response::Response) -> Value {
    let body = resp.into_body();
    let bytes = to_bytes(body, 1024 * 1024).await.expect("read body");
    serde_json::from_slice(&bytes).expect("parse json")
}

// ─────────────────────────────────────────────────────────────────────────────
// F1 — valid JWT + capability → 201 + row inserted
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker + testcontainers"]
async fn t_create_page_with_valid_jwt_returns_201_and_inserts_row() {
    let ctx = TestContext::setup().await;
    let router = ax_presentation::build_router(ctx.state.clone());

    let jwt = ctx.sign_jwt(ctx.admin_a_user, ctx.tenant_a, "admin", 60);
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/cms/pages/admin")
        .header("X-Tenant-Slug", TENANT_A_SLUG)
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .body(Body::from(build_post_body().to_string()))
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    let status = resp.status();
    let body = read_body_json(resp).await;
    assert_eq!(status, StatusCode::CREATED, "expected 201, got {status}, body={body}");
    assert_eq!(body["slug"], "test-draft-page");
    assert_eq!(body["status"], "draft");
    assert_eq!(body["authorUserId"], ctx.admin_a_user.to_string());

    // Verify row inserted in DB
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM cms_pages WHERE tenant_id = $1 AND slug = 'test-draft-page'",
    )
    .bind(ctx.tenant_a)
    .fetch_one(&ctx.admin_pool)
    .await
    .unwrap();
    assert_eq!(count, 1, "row should be inserted");
}

// ─────────────────────────────────────────────────────────────────────────────
// F2 — no JWT → 401
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker + testcontainers"]
async fn t_no_jwt_returns_401() {
    let ctx = TestContext::setup().await;
    let router = ax_presentation::build_router(ctx.state.clone());

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/cms/pages/admin")
        .header("X-Tenant-Slug", TENANT_A_SLUG)
        .header("Content-Type", "application/json")
        .body(Body::from(build_post_body().to_string()))
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = read_body_json(resp).await;
    assert_eq!(body["code"], "NOT_AUTHENTICATED");
}

// ─────────────────────────────────────────────────────────────────────────────
// F5 — expired JWT → 401 TOKEN_EXPIRED
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker + testcontainers"]
async fn t_expired_jwt_returns_401() {
    let ctx = TestContext::setup().await;
    let router = ax_presentation::build_router(ctx.state.clone());

    let jwt = ctx.sign_jwt(ctx.admin_a_user, ctx.tenant_a, "admin", -10);
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/cms/pages/admin")
        .header("X-Tenant-Slug", TENANT_A_SLUG)
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .body(Body::from(build_post_body().to_string()))
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = read_body_json(resp).await;
    assert_eq!(body["code"], "TOKEN_EXPIRED");
}

// ─────────────────────────────────────────────────────────────────────────────
// F6 — malformed JWT → 401 INVALID_TOKEN
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker + testcontainers"]
async fn t_malformed_jwt_returns_401() {
    let ctx = TestContext::setup().await;
    let router = ax_presentation::build_router(ctx.state.clone());

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/cms/pages/admin")
        .header("X-Tenant-Slug", TENANT_A_SLUG)
        .header("Authorization", "Bearer not.a.real.token")
        .header("Content-Type", "application/json")
        .body(Body::from(build_post_body().to_string()))
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = read_body_json(resp).await;
    assert_eq!(body["code"], "INVALID_TOKEN");
}

// ─────────────────────────────────────────────────────────────────────────────
// F4/S7 — forged JWT (wrong secret) → 401 INVALID_TOKEN
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker + testcontainers"]
async fn t_forged_jwt_wrong_secret_returns_401() {
    let ctx = TestContext::setup().await;
    let router = ax_presentation::build_router(ctx.state.clone());

    let jwt = sign_jwt_with_secret(
        "different-secret-32-bytes-padding-xx",
        ctx.admin_a_user,
        ctx.tenant_a,
        "admin",
        60,
    );
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/cms/pages/admin")
        .header("X-Tenant-Slug", TENANT_A_SLUG)
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .body(Body::from(build_post_body().to_string()))
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = read_body_json(resp).await;
    assert_eq!(body["code"], "INVALID_TOKEN");
}

// ─────────────────────────────────────────────────────────────────────────────
// F7/S4 — user без posts:create capability → 403 MISSING_CAPABILITY
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker + testcontainers"]
async fn t_user_without_capability_returns_403() {
    let ctx = TestContext::setup().await;
    let router = ax_presentation::build_router(ctx.state.clone());

    let jwt = ctx.sign_jwt(ctx.no_cap_user, ctx.tenant_a, "client", 60);
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/cms/pages/admin")
        .header("X-Tenant-Slug", TENANT_A_SLUG)
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .body(Body::from(build_post_body().to_string()))
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = read_body_json(resp).await;
    assert_eq!(body["code"], "MISSING_CAPABILITY");
    assert_eq!(body["required"], "posts:create");
}

// ─────────────────────────────────────────────────────────────────────────────
// F8/S5 — JWT для другого tenant → 403 TENANT_OWNERSHIP_MISMATCH
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker + testcontainers"]
async fn t_jwt_for_other_tenant_returns_403() {
    let ctx = TestContext::setup().await;
    let router = ax_presentation::build_router(ctx.state.clone());

    // Sign JWT bound to tenant_b, but send request for tenant_a (header)
    let jwt = ctx.sign_jwt(ctx.admin_a_user, ctx.tenant_b, "admin", 60);
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/cms/pages/admin")
        .header("X-Tenant-Slug", TENANT_A_SLUG)
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .body(Body::from(build_post_body().to_string()))
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = read_body_json(resp).await;
    assert_eq!(body["code"], "TENANT_OWNERSHIP_MISMATCH");
}

// ─────────────────────────────────────────────────────────────────────────────
// F9/S9 — RLS blocks adversarial direct-SQL spoof
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker + testcontainers"]
async fn t_rls_blocks_spoofed_tenant_id_via_direct_sql() {
    let ctx = TestContext::setup().await;

    // As ax_app_role: set ctx tenant_a, attempt INSERT с tenant_b — должен fail
    let result = sqlx::query(
        "BEGIN;
         SELECT set_config('app.current_tenant_id', $1, true);
         INSERT INTO cms_pages (tenant_id, slug, locale, title, status)
           VALUES ($2, 'spoof-attempt', 'ru', 'spoof', 'draft');
         COMMIT;",
    )
    .bind(ctx.tenant_a.to_string())
    .bind(ctx.tenant_b)
    .execute(&ctx.pool)
    .await;

    assert!(result.is_err(), "RLS WITH CHECK should reject cross-tenant INSERT");

    // Verify нет такой row
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM cms_pages WHERE slug = 'spoof-attempt'",
    )
    .fetch_one(&ctx.admin_pool)
    .await
    .unwrap();
    assert_eq!(count, 0, "spoofed row should NOT have been inserted");
}

// ─────────────────────────────────────────────────────────────────────────────
// I4 — read-path regression check (cms_pages public read still works)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker + testcontainers"]
async fn t_health_endpoint_still_works() {
    let ctx = TestContext::setup().await;
    let router = ax_presentation::build_router(ctx.state.clone());

    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// ─────────────────────────────────────────────────────────────────────────────
// Smoke test — without Docker — sign + verify roundtrip
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn smoke_sign_verify_jwt_roundtrip() {
    let secret = "x".repeat(32);
    let user_id = Uuid::new_v4();
    let tenant_id = TenantId::new(Uuid::new_v4()).0;
    let jwt = sign_jwt_with_secret(&secret, user_id, tenant_id, "admin", 60);
    let v = ax_infrastructure::JwtVerifier::new(&secret);
    let claims = v.verify(&jwt).expect("verify roundtrip");
    assert_eq!(claims.sub, user_id.to_string());
    assert_eq!(claims.tenant_id, tenant_id.to_string());
}
