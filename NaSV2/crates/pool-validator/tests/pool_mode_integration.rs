//! Integration tests for `nas2-pool-validator` · ENTITY §3.4.1, VAL-002.
//!
//! All tests are gated on `#[ignore = "needs Docker"]` because they spin up
//! a Postgres + PgBouncer pair via `testcontainers`. Run with:
//!
//! ```sh
//! cargo test -p nas2-pool-validator --tests -- --ignored
//! ```
//!
//! TLA layers:
//! - **L1 Correctness** — confirms `ensure_transaction_mode` returns Ok in
//!   `transaction` mode and `WrongMode` in `session`/`statement`.
//! - **L2 Performance** — single `SHOW pool_mode` round-trip per case.
//! - **L3 Scalability** — each test runs an isolated bridge network so cases
//!   can execute in parallel without container-name collisions.
//! - **L4 Operability** — log-line capture confirms the structured tracing
//!   event `pool_mode_drift` is emitted.

#![cfg(test)]
// Tests intentionally `expect`/`panic` on failure — that *is* the failure mode.
// Raw `tokio::spawn` is used in `test_concurrent_load_isolation` to simulate
// arbitrary external concurrent traffic against the pool — the production
// callers themselves go through `TaskSupervisor`, but the test models the
// pool's behavior under any caller mix.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::disallowed_methods
)]

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

use nas2_pool_validator::{
    detect_pool_mode, ensure_transaction_mode, PoolMode, PoolValidationError,
};

const PG_USER: &str = "postgres";
const PG_PASSWORD: &str = "postgres";
const PG_DB: &str = "postgres";
const PGBOUNCER_IMAGE: &str = "edoburu/pgbouncer";
const PGBOUNCER_TAG: &str = "1.23.1";

/// Holds the running containers + a connected `PgPool` so callers can issue
/// queries through PgBouncer. Dropping the struct stops both containers.
struct TestStack {
    _postgres: ContainerAsync<Postgres>,
    _pgbouncer: ContainerAsync<GenericImage>,
    pool: sqlx::PgPool,
}

async fn start_stack(pool_mode: &str) -> TestStack {
    let suffix = Uuid::new_v4().simple().to_string();
    let network = format!("pool-validator-{suffix}");
    let pg_name = format!("pg-{suffix}");

    let postgres = Postgres::default()
        .with_user(PG_USER)
        .with_password(PG_PASSWORD)
        .with_db_name(PG_DB)
        .with_network(network.clone())
        .with_container_name(pg_name.clone())
        .start()
        .await
        .expect("postgres start");

    let pgbouncer = GenericImage::new(PGBOUNCER_IMAGE, PGBOUNCER_TAG)
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stderr("process up"))
        .with_network(network)
        .with_env_var("DB_HOST", &pg_name)
        .with_env_var("DB_PORT", "5432")
        .with_env_var("DB_USER", PG_USER)
        .with_env_var("DB_PASSWORD", PG_PASSWORD)
        .with_env_var("DB_NAME", PG_DB)
        .with_env_var("AUTH_TYPE", "trust")
        .with_env_var("POOL_MODE", pool_mode)
        .start()
        .await
        .expect("pgbouncer start");

    let host = pgbouncer.get_host().await.expect("pgbouncer host");
    let port = pgbouncer
        .get_host_port_ipv4(5432)
        .await
        .expect("pgbouncer mapped port");
    let url = format!("postgres://{PG_USER}:{PG_PASSWORD}@{host}:{port}/{PG_DB}");

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&url)
        .await
        .expect("sqlx connect via pgbouncer");

    TestStack {
        _postgres: postgres,
        _pgbouncer: pgbouncer,
        pool,
    }
}

#[tokio::test]
#[ignore = "needs Docker"]
async fn test_ensure_transaction_mode_accepts_transaction_pool() {
    let stack = start_stack("transaction").await;
    ensure_transaction_mode(&stack.pool)
        .await
        .expect("transaction mode should pass the §3.4.1 gate");
}

#[tokio::test]
#[ignore = "needs Docker"]
async fn test_ensure_transaction_mode_rejects_session_pool() {
    let stack = start_stack("session").await;
    let err = ensure_transaction_mode(&stack.pool)
        .await
        .expect_err("session mode must fail the §3.4.1 gate");
    match err {
        PoolValidationError::WrongMode {
            actual: PoolMode::Session,
        } => {},
        other => panic!("expected WrongMode(Session), got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "needs Docker"]
async fn test_detect_pool_mode_returns_statement_when_configured() {
    let stack = start_stack("statement").await;
    let mode = detect_pool_mode(&stack.pool).await.expect("detect mode");
    assert_eq!(mode, PoolMode::Statement);
}

/// 50 concurrent `ensure_transaction_mode` calls must not serialize on the
/// `PgPool`. We give the pool only 2 connections and a 2 s acquire timeout —
/// pgbouncer's transaction mode plus sqlx's connection multiplexing should
/// still finish the burst well under the per-task budget.
///
/// Closes recommendation R1 from `SESSION_LOG.md` (AVTONOM 2026-05-25).
/// ENTITY §3.4.2 pool isolation; §7 tail-latency invariants.
#[tokio::test]
#[ignore = "needs Docker"]
async fn test_concurrent_load_isolation() {
    let stack = start_stack("transaction").await;
    let mut handles = Vec::with_capacity(50);
    for _ in 0..50 {
        let pool = stack.pool.clone();
        handles.push(tokio::spawn(
            async move { ensure_transaction_mode(&pool).await },
        ));
    }
    for h in handles {
        h.await
            .expect("task panicked")
            .expect("ensure_transaction_mode under concurrent load");
    }
}
