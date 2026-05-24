//! `GET /api/v1/version` — build metadata endpoint.

use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct VersionResponse {
    pub version: &'static str,
    pub git_sha: &'static str,
    pub build_time: &'static str,
}

#[axum::debug_handler]
pub async fn handler() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
        git_sha: option_env!("GIT_SHA").unwrap_or("dev"),
        build_time: option_env!("BUILD_TIME").unwrap_or("unknown"),
    })
}
