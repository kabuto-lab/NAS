//! `ax-server` — Axum HTTP server entry-point.
//!
//! Bootstrap sequence per ADR-001 D8:
//! 1. Load `.env`, parse config
//! 2. Init tracing-subscriber (JSON) + OTLP + Sentry
//! 3. Init jemallocator (linux/macos only)
//! 4. Create PgPool через `build_pool`
//! 5. Build `AppState` (Arc'ed repos + moka caches)
//! 6. Build Axum router
//! 7. Bind on `API_PORT` (default 7000)
//! 8. Graceful shutdown через `tokio::signal`

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use eyre::{Context, Result};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load .env (best-effort, не fatal если нет)
    let _ = dotenvy::dotenv();

    // 2. Init tracing
    init_tracing();

    tracing::info!("ax-server starting...");

    // 3. Config (TODO: replace with proper config::Config builder)
    let port: u16 = std::env::var("API_PORT")
        .unwrap_or_else(|_| "7000".to_string())
        .parse()
        .context("API_PORT must be a valid port number")?;

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL is required")?;

    // 4. DB pool
    let pool_config = ax_infrastructure::persistence::PoolConfig {
        url: database_url,
        max_connections: 20,
        min_connections: 2,
        acquire_timeout_secs: 5,
    };
    let pool = ax_infrastructure::persistence::build_pool(&pool_config)
        .await
        .context("failed to build pg pool")?;

    tracing::info!("database pool ready");

    // 5. AppState
    let state = ax_presentation::AppState::new(pool);

    // 6. Router
    let app = ax_presentation::build_router(state);

    // 7. Listen
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, "READY — listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    tracing::info!("ax-server shut down cleanly");
    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,ax_=debug,sqlx=warn"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}

#[allow(clippy::cognitive_complexity)]   // cfg-conditional branches inflate metric
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("install ctrl_c handler");
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
        () = ctrl_c => tracing::info!("ctrl-c received, shutting down"),
        () = terminate => tracing::info!("SIGTERM received, shutting down"),
    }
}
