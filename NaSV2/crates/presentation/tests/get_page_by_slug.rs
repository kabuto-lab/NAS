//! End-to-end happy-path test for `GET /api/v1/pages/:slug`.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// mockall::mock! generates std::sync::Mutex use; acceptable in tests.
#![allow(clippy::disallowed_types)]
// serde_json::from_slice in tests is acceptable (cold path); production
// hot paths use simd-json / sonic-rs per ENTITY §3.11.
#![allow(clippy::disallowed_methods)]

use std::{collections::HashMap, sync::Arc};

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::Utc;
use mockall::mock;
use nas2_application::ports::PostRepository;
use nas2_common::{Page, PostId, SiteId, TenantId};
use nas2_domain::{Post, PostSlug, PostStatus};
use nas2_presentation::{AppState, build_router};
use nas2_tenant::{InMemoryTenantResolver, TenantContext};
use tower::ServiceExt;

mock! {
    pub Repo {}
    #[async_trait::async_trait]
    impl PostRepository for Repo {
        async fn find_by_slug(
            &self,
            tenant: TenantId,
            site: SiteId,
            slug: &PostSlug,
        ) -> Result<Option<Post>, nas2_common::AppError>;
        async fn find_by_id(
            &self,
            tenant: TenantId,
            id: PostId,
        ) -> Result<Option<Post>, nas2_common::AppError>;
        async fn list_published(
            &self,
            tenant: TenantId,
            site: SiteId,
            page: u32,
            per_page: u32,
        ) -> Result<Page<Post>, nas2_common::AppError>;
        async fn insert(&self, post: &Post) -> Result<(), nas2_common::AppError>;
        async fn update(&self, post: &Post) -> Result<(), nas2_common::AppError>;
    }
}

fn sample_post(slug: &str, status: PostStatus, tenant: TenantId, site: SiteId) -> Post {
    Post {
        id: PostId::new_v4(),
        site_id: site,
        tenant_id: tenant,
        slug: PostSlug::try_new(slug).unwrap(),
        title: "Hello".into(),
        status,
        published_at: matches!(status, PostStatus::Published).then(Utc::now),
        blocks: Vec::new(),
        custom_type: None,
    }
}

fn build_state(host: &str, repo: Arc<MockRepo>) -> AppState {
    let tid = TenantId::new_v4();
    let sid = SiteId::new_v4();
    let mut map = HashMap::new();
    map.insert(host.to_owned(), TenantContext::new(tid, sid));
    let resolver = Arc::new(InMemoryTenantResolver::new(map));
    AppState::new(repo, resolver)
}

#[tokio::test]
async fn returns_200_for_published_page() {
    let host = "example.com";
    let mut repo = MockRepo::new();
    repo.expect_find_by_slug().returning(|t, s, _slug| {
        Ok(Some(sample_post("hello", PostStatus::Published, t, s)))
    });
    let app = build_router(build_state(host, Arc::new(repo)));
    let req = Request::builder()
        .uri("/api/v1/pages/hello")
        .header(header::HOST, host)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        v.get("slug").and_then(serde_json::Value::as_str),
        Some("hello"),
        "JSON body must carry the requested slug"
    );
}

#[tokio::test]
async fn returns_403_when_caps_missing() {
    use nas2_domain::CapabilitySet;
    use nas2_presentation::caps::with_caps_for_test;

    let host = "example.com";
    let repo = MockRepo::new(); // no expectations → handler must short-circuit before repo
    let app = build_router(build_state(host, Arc::new(repo)));

    // Hold the guard across the request — drop after oneshot awaits.
    let _guard = with_caps_for_test(CapabilitySet::new());
    let req = Request::builder()
        .uri("/api/v1/pages/hello")
        .header(header::HOST, host)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        v.get("code").and_then(serde_json::Value::as_str),
        Some("forbidden")
    );
    assert!(
        v.get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .contains("cms.page.read"),
        "message must name the missing capability: {v:?}"
    );
}

#[tokio::test]
async fn returns_404_when_slug_not_found() {
    let host = "example.com";
    let mut repo = MockRepo::new();
    repo.expect_find_by_slug().returning(|_, _, _| Ok(None));
    let app = build_router(build_state(host, Arc::new(repo)));

    let req = Request::builder()
        .uri("/api/v1/pages/missing")
        .header(header::HOST, host)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        v.get("code").and_then(serde_json::Value::as_str),
        Some("not_found")
    );
}

#[tokio::test]
async fn returns_400_when_slug_invalid_chars() {
    let host = "example.com";
    let repo = MockRepo::new(); // never called — PostSlug::try_new fails first
    let app = build_router(build_state(host, Arc::new(repo)));

    // BadSlug%21 → "BadSlug!" after percent-decode. PostSlug::try_new
    // rejects uppercase + '!'.
    let req = Request::builder()
        .uri("/api/v1/pages/BadSlug%21")
        .header(header::HOST, host)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        v.get("code").and_then(serde_json::Value::as_str),
        Some("validation_failed")
    );
}
