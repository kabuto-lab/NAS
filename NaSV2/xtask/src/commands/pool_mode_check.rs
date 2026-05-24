//! `xtask pool-mode-check` · ENTITY §3.4.1 pre-deploy gate.
//!
//! Connects to the PgBouncer instance referenced by `DATABASE_URL_HTTP`
//! (or the `--database-url` flag) and runs `SHOW pool_mode`.  Exits 0 on
//! `transaction`, 1 otherwise.  Designed to be called from CI smoke-tests
//! and `vps:after-pull` before the server binary is rolled.

use std::time::Duration;

use eyre::{Result, WrapErr};
use sqlx::postgres::PgPoolOptions;

/// Run the pool-mode check synchronously by spinning up a single-thread
/// Tokio runtime — `xtask` itself stays a plain `fn main()`.
pub fn run(database_url: &str) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .wrap_err("tokio runtime build")?;
    rt.block_on(check(database_url))
}

async fn check(database_url: &str) -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .test_before_acquire(false)
        .connect(database_url)
        .await
        .wrap_err("connect to PgBouncer for SHOW pool_mode")?;

    let row: (String,) = sqlx::query_as("SHOW pool_mode")
        .fetch_one(&pool)
        .await
        .wrap_err("SHOW pool_mode failed — is this actually PgBouncer?")?;

    let mode = row.0.trim().to_ascii_lowercase();
    if mode == "transaction" {
        println!("xtask pool-mode-check: OK (pool_mode = {mode})");
        Ok(())
    } else {
        Err(eyre::eyre!(
            "ENTITY §3.4.1 violated: pool_mode = {mode:?}, expected `transaction`"
        ))
    }
}
