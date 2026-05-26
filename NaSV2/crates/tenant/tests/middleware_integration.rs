//! Integration tests for `resolve_tenant` middleware.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::Extension,
    http::{Request, StatusCode, header},
    middleware::from_fn_with_state,
    routing::get,
};
use nas2_common::{SiteId, TenantId};
use nas2_tenant::{
    InMemoryTenantResolver, TenantContext, TenantResolverHandle, resolve_tenant,
};
use tower::ServiceExt;

async fn echo_tenant(Extension(ctx): Extension<TenantContext>) -> String {
    format!("{}:{}", ctx.tenant_id, ctx.site_id)
}

fn build_app(resolver: TenantResolverHandle) -> Router {
    Router::new()
        .route("/", get(echo_tenant))
        .layer(from_fn_with_state(resolver, resolve_tenant))
}

#[tokio::test]
async fn inserts_context_for_known_host() {
    let tid = TenantId::new_v4();
    let sid = SiteId::new_v4();
    let resolver: TenantResolverHandle = Arc::new(
        InMemoryTenantResolver::builder()
            .insert("example.com", TenantContext::new(tid, sid))
            .build(),
    );
    let app = build_app(resolver);
    let req = Request::builder()
        .uri("/")
        .header(header::HOST, "example.com")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    let s = std::str::from_utf8(&body).unwrap();
    assert!(
        s.starts_with(&format!("{tid}")),
        "echo body starts with tenant_id: got {s}"
    );
}

#[tokio::test]
async fn returns_400_when_host_missing() {
    let resolver: TenantResolverHandle =
        Arc::new(InMemoryTenantResolver::default());
    let app = build_app(resolver);
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn returns_404_when_host_unmapped() {
    let resolver: TenantResolverHandle =
        Arc::new(InMemoryTenantResolver::default());
    let app = build_app(resolver);
    let req = Request::builder()
        .uri("/")
        .header(header::HOST, "nope.example.com")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
