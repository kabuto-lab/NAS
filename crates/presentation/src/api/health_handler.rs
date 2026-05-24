//! Health endpoints — `/health` liveness, `/health/ready` readiness.
//!
//! Per audit §12.6: SITE1 use case + AX extensions (git_sha exposed).

use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app_state::AppState;

static STARTED_AT: AtomicU64 = AtomicU64::new(0);

fn ensure_start_time() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = STARTED_AT.compare_exchange(0, now, Ordering::SeqCst, Ordering::SeqCst);
    STARTED_AT.load(Ordering::SeqCst)
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub db: &'static str,
    pub uptime: u64,
    pub version: &'static str,
    pub git_sha: &'static str,
    pub env: String,
    pub timestamp: String,
}

#[axum::debug_handler]
pub async fn liveness(State(_state): State<AppState>) -> Json<HealthResponse> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let uptime = now.saturating_sub(ensure_start_time());

    // TODO: DB ping (когда AppState exposes pool ref)
    Json(HealthResponse {
        ok: true,
        db: "unknown",
        uptime,
        version: env!("CARGO_PKG_VERSION"),
        git_sha: option_env!("GIT_SHA").unwrap_or("dev"),
        env: std::env::var("APP_ENV").unwrap_or_else(|_| "development".into()),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

#[axum::debug_handler]
pub async fn readiness(State(_state): State<AppState>) -> Result<&'static str, StatusCode> {
    // TODO: SELECT 1 ping + S3 HEAD check + pgmq probe (Phase B)
    Ok("ok")
}
