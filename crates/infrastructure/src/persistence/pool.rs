//! PostgreSQL connection pool factory.
//!
//! AX подключается через PgBouncer (см. `ENTITY.md §3` — PgBouncer ОБЯЗАТЕЛЬНО в
//! `transaction` pool mode для корректной работы RLS via `SET LOCAL`).

use eyre::{Context, Result};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

/// Pool configuration. Loaded from env в `apps/server/src/main.rs`.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
}

/// Build a connection pool. Tests connectivity on startup.
///
/// # Errors
/// - DB unreachable
/// - Invalid `DATABASE_URL`
pub async fn build_pool(config: &PoolConfig) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
        .connect(&config.url)
        .await
        .context("connecting to PostgreSQL")?;

    // Smoke test
    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .context("DB smoke test SELECT 1 failed")?;

    tracing::info!(
        max_connections = config.max_connections,
        "PostgreSQL pool ready"
    );

    Ok(pool)
}
