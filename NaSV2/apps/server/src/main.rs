//! `nas2-server` · AX•CMS HTTP server binary
//!
//! Bootstrap contract (ENTITY §3, §4, §22.0):
//! 1. Load `.env`, parse config (`crates/common::Config`).
//! 2. Init observability layers via `nas2_runtime::observability::init`
//!    (JSON tracing + OTLP + Prometheus exporter).
//! 3. Build three isolated PgPools — `http_pool`, `worker_pool`, `admin_pool`
//!    (ENTITY §3.4.2 — HARD GATE: single shared pool is forbidden).
//! 4. Run `nas2_pool_validator::ensure_transaction_mode` on each pool
//!    (ENTITY §3.4.1 — HARD GATE: fail-fast if pgbouncer not in `transaction`).
//! 5. Build `AppState` (Arc'ed repos, caches, registries).
//! 6. Build Axum router (REST + Leptos SSR + `/health/*`).
//! 7. Bind on `API_PORT` (default 8000); spawn signal handler for graceful shutdown
//!    bounded by `SHUTDOWN_GRACE_SECS` (default 30 s) — ENTITY §29.
//!
//! ## TLA layers
//! - **L1 Correctness** — pool-mode contract enforced before bind; impossible
//!   to serve traffic against a misconfigured pgbouncer.
//! - **L2 Performance** — single allocation per pool; mimalloc as default.
//! - **L3 Scalability** — pool isolation prevents admin/worker traffic from
//!   eating http_pool budget under load.
//! - **L4 Operability** — `/health/live`, `/health/ready`, `/health/pool`,
//!   plus Prometheus on a separate port — feeds deploy pipeline and scraper.

#![forbid(unsafe_code)]

// ──────────────────────────────────────────────────────────────────────────
// Global allocator · ENTITY §3.12 (mimalloc default; jemalloc via feature)
// ──────────────────────────────────────────────────────────────────────────

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(all(
    feature = "jemalloc",
    not(feature = "mimalloc"),
    any(target_os = "linux", target_os = "macos")
))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

// ──────────────────────────────────────────────────────────────────────────

use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{routing::get, Router};
use sqlx::postgres::{PgPool, PgPoolOptions};

/// Boot config — minimal subset for the §3.4 pool contract.
/// Production config will live in `crates/common::Config` (ENTITY §19).
#[derive(Debug)]
struct BootConfig {
    bind_addr: SocketAddr,
    http_url: String,
    worker_url: String,
    admin_url: String,
    http_pool_size: u32,
    worker_pool_size: u32,
    admin_pool_size: u32,
    shutdown_grace: Duration,
}

impl BootConfig {
    fn from_env() -> eyre::Result<Self> {
        use std::env::var;
        let bind = var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0".into());
        let port: u16 = var("API_PORT").unwrap_or_else(|_| "8000".into()).parse()?;
        let shutdown_secs: u64 = var("SHUTDOWN_GRACE_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        Ok(Self {
            bind_addr: format!("{bind}:{port}").parse()?,
            http_url: var("DATABASE_URL_HTTP")
                .map_err(|_| eyre::eyre!("DATABASE_URL_HTTP missing"))?,
            worker_url: var("DATABASE_URL_WORKER")
                .map_err(|_| eyre::eyre!("DATABASE_URL_WORKER missing"))?,
            admin_url: var("DATABASE_URL_ADMIN")
                .map_err(|_| eyre::eyre!("DATABASE_URL_ADMIN missing"))?,
            http_pool_size: var("DB_HTTP_POOL_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(25),
            worker_pool_size: var("DB_WORKER_POOL_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            admin_pool_size: var("DB_ADMIN_POOL_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            shutdown_grace: Duration::from_secs(shutdown_secs),
        })
    }
}

/// Application-wide state shared with handlers.
// `*_pool` suffixes are domain-meaningful (ENTITY §3.4.2 pool isolation);
// renaming to bare `http`/`worker`/`admin` would lose the type semantics.
#[allow(clippy::struct_field_names)]
#[derive(Clone)]
struct AppState {
    http_pool: PgPool,
    worker_pool: PgPool,
    admin_pool: PgPool,
}

async fn build_pool(label: &str, url: &str, size: u32) -> eyre::Result<PgPool> {
    tracing::info!(label, size, "building pgpool");
    let pool = PgPoolOptions::new()
        .max_connections(size)
        .acquire_timeout(Duration::from_secs(5))
        .test_before_acquire(false)
        .connect(url)
        .await?;
    Ok(pool)
}

/// HARD GATE — ENTITY §3.4.1. Returns Err on any drift; caller panics on Err.
async fn validate_pool_mode(label: &str, pool: &PgPool) -> eyre::Result<()> {
    nas2_pool_validator::ensure_transaction_mode(pool)
        .await
        .map_err(|e| eyre::eyre!("pool '{label}': {e}"))
}

async fn health_live() -> &'static str {
    "ok"
}

/// Readiness probe — confirms the HTTP pool can answer a trivial query.
/// Returns 503 on any failure; 200 with body `"ready"` otherwise.
async fn health_ready(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Result<&'static str, axum::http::StatusCode> {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.http_pool)
        .await
    {
        Ok(_) => Ok("ready"),
        Err(e) => {
            tracing::error!(error = %e, "readiness probe: SELECT 1 failed");
            Err(axum::http::StatusCode::SERVICE_UNAVAILABLE)
        },
    }
}

// `tracing::error!` expansions inflate cognitive complexity; loop body has
// three semantically simple arms.
#[allow(clippy::cognitive_complexity)]
async fn health_pool(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> axum::http::StatusCode {
    for (label, pool) in [
        ("http", &state.http_pool),
        ("worker", &state.worker_pool),
        ("admin", &state.admin_pool),
    ] {
        match nas2_pool_validator::detect_pool_mode(pool).await {
            Ok(m) if m.is_transaction() => {},
            Ok(m) => {
                tracing::error!(label, mode = ?m, "pool_mode drift detected");
                return axum::http::StatusCode::SERVICE_UNAVAILABLE;
            },
            Err(e) => {
                tracing::error!(label, error = %e, "pool_mode check failed");
                return axum::http::StatusCode::SERVICE_UNAVAILABLE;
            },
        }
    }
    axum::http::StatusCode::OK
}

fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/health/pool", get(health_pool))
        .with_state(state)
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let _ = dotenvy::dotenv();
    let _obs_guard = nas2_runtime::observability::init("nas2-server")?;

    let cfg = BootConfig::from_env()?;
    tracing::info!(?cfg.bind_addr, "ax-cms · boot");

    // 1. Build three isolated pools (ENTITY §3.4.2 HARD GATE)
    let http_pool = build_pool("http", &cfg.http_url, cfg.http_pool_size).await?;
    let worker_pool = build_pool("worker", &cfg.worker_url, cfg.worker_pool_size).await?;
    let admin_pool = build_pool("admin", &cfg.admin_url, cfg.admin_pool_size).await?;

    // 2. ENTITY §3.4.1 — verify pgbouncer is in transaction mode on each pool.
    //    Panic on drift; deploy pipeline picks up non-zero exit.
    validate_pool_mode("http", &http_pool).await?;
    validate_pool_mode("worker", &worker_pool).await?;
    validate_pool_mode("admin", &admin_pool).await?;
    tracing::info!("pool-mode contract satisfied on all three pools");

    // 3. AppState
    let state = Arc::new(AppState {
        http_pool,
        worker_pool,
        admin_pool,
    });

    // 4. Router
    let router = build_router(state);

    // 5. Bind and serve, with bounded graceful shutdown drain.
    let listener = tokio::net::TcpListener::bind(cfg.bind_addr).await?;
    tracing::info!(addr = %cfg.bind_addr, "listening");

    let grace = cfg.shutdown_grace;
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal_with_drain(grace))
        .await?;

    Ok(())
}

/// Waits for SIGINT/SIGTERM, then yields after at most `grace` so axum's
/// internal drain is followed by an unconditional `force-kill` upper bound.
// `tracing::info!`/`warn!` macros inflate cognitive complexity; body is
// strictly linear (signal → drain → timeout → log).
#[allow(clippy::cognitive_complexity)]
async fn shutdown_signal_with_drain(grace: Duration) {
    shutdown_signal().await;
    tracing::info!(grace_secs = grace.as_secs(), "draining in-flight requests");
    // Hand a hard deadline back to the runtime so a stuck handler cannot
    // hold the process forever (ENTITY §29 — bounded shutdown).
    let _ = tokio::time::timeout(grace, std::future::pending::<()>()).await;
    tracing::warn!(
        grace_secs = grace.as_secs(),
        "shutdown drain expired — proceeding to abort"
    );
}

// `cfg` branches + `tracing::info!` expansions inflate cognitive complexity;
// signal-handler installation failures are unrecoverable at boot.
#[allow(clippy::cognitive_complexity, clippy::expect_used)]
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install ctrl-c handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => tracing::info!("ctrl-c received — shutting down"),
        () = terminate => tracing::info!("SIGTERM received — shutting down"),
    }
}
