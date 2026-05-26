//! Router smoke tests — verify health endpoints come up with tenant
//! middleware mounted and route through known/unknown hosts correctly.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// mockall::mock! expands to code that uses std::sync::Mutex (ENTITY §8.7
// disallowed in production; acceptable in tests).
#![allow(clippy::disallowed_types)]

use std::{collections::HashMap, sync::Arc};

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use mockall::mock;
use nas2_application::ports::PostRepository;
use nas2_common::{Page, PostId, SiteId, TenantId};
use nas2_domain::{Post, PostSlug};
use nas2_presentation::{AppState, build_router};
use nas2_tenant::{InMemoryTenantResolver, TenantContext};
use tower::ServiceExt;

// The application crate's MockPostRepository is `#[cfg(test)]`-only and
// not visible across crates. Local mock for integration tests.
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

fn build_state_with_host(host: &str) -> AppState {
    let tid = TenantId::new_v4();
    let sid = SiteId::new_v4();
    let mut map = HashMap::new();
    map.insert(host.to_owned(), TenantContext::new(tid, sid));
    let resolver = Arc::new(InMemoryTenantResolver::new(map));
    AppState::new(Arc::new(MockRepo::new()), resolver)
}

#[tokio::test]
async fn live_returns_200_with_known_host() {
    let app = build_router(build_state_with_host("example.com"));
    let req = Request::builder()
        .uri("/health/live")
        .header(header::HOST, "example.com")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn ready_returns_200_with_known_host() {
    let app = build_router(build_state_with_host("example.com"));
    let req = Request::builder()
        .uri("/health/ready")
        .header(header::HOST, "example.com")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn unknown_host_returns_404_from_resolver() {
    let app = build_router(build_state_with_host("example.com"));
    let req = Request::builder()
        .uri("/health/live")
        .header(header::HOST, "other.example.com")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
