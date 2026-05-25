//! pgmq adapter — implements `application::ports::Queue` against the
//! `pgmq` Postgres extension (ENTITY §3.8 transactional queue tier).
//!
//! ## Pool wiring (ENTITY §3.4.2)
//!
//! Constructor takes the **worker pool** (`PgPool`). Queue traffic MUST NOT
//! share the HTTP pool; the contract is documented in `apps/server/src/main.rs`.
//!
//! ## TLA layers
//!
//! - **L1 Correctness** — every method is a single `SELECT pgmq.<fn>(...)`
//!   call; transactional semantics are inherited from Postgres.
//! - **L2 Performance** — `read(qty)` is one roundtrip returning N rows; no
//!   N+1. Payload is `serde_json::Value` to avoid double serialization in
//!   the request path; binary payload variant lands when needed.
//! - **L3 Scalability** — pgmq tables are partitioned by queue name on the
//!   server side; the adapter is stateless and freely cloneable.
//! - **L4 Operability** — every failure path returns `QueueError::Backend`
//!   carrying the upstream message so operators can grep server logs.
//!
//! ## Compile-time SQL choice
//!
//! Uses `sqlx::query`/`query_scalar` (runtime) rather than `sqlx::query!`.
//! Rationale: `pgmq` ships its functions in a separate schema that is not
//! present at workspace `cargo check` time, so the macro would block the
//! build on every developer machine. The pgmq function signatures are stable
//! across the supported version range; integration tests catch drift.

use async_trait::async_trait;
use nas2_application::ports::{Queue, QueueError, QueueMessage};
use serde_json::Value;
use sqlx::{postgres::PgRow, PgPool, Row};

#[derive(Clone)]
pub struct PgmqQueue {
    worker_pool: PgPool,
}

impl PgmqQueue {
    #[must_use]
    pub const fn new(worker_pool: PgPool) -> Self {
        Self { worker_pool }
    }
}

fn map_err(e: sqlx::Error) -> QueueError {
    QueueError::Backend(e.to_string())
}

#[async_trait]
impl Queue for PgmqQueue {
    async fn send(&self, queue: &str, payload: &Value) -> Result<i64, QueueError> {
        let id: i64 = sqlx::query_scalar("SELECT pgmq.send($1, $2)")
            .bind(queue)
            .bind(payload)
            .fetch_one(&self.worker_pool)
            .await
            .map_err(map_err)?;
        Ok(id)
    }

    async fn read(
        &self,
        queue: &str,
        vt_seconds: i32,
        qty: i32,
    ) -> Result<Vec<QueueMessage>, QueueError> {
        let rows: Vec<PgRow> =
            sqlx::query("SELECT msg_id, read_ct, message FROM pgmq.read($1, $2, $3)")
                .bind(queue)
                .bind(vt_seconds)
                .bind(qty)
                .fetch_all(&self.worker_pool)
                .await
                .map_err(map_err)?;
        rows.into_iter()
            .map(|r| {
                Ok(QueueMessage {
                    msg_id: r.try_get::<i64, _>("msg_id").map_err(map_err)?,
                    read_ct: r.try_get::<i32, _>("read_ct").map_err(map_err)?,
                    payload: r.try_get::<Value, _>("message").map_err(map_err)?,
                })
            })
            .collect()
    }

    async fn delete(&self, queue: &str, msg_id: i64) -> Result<bool, QueueError> {
        let ok: bool = sqlx::query_scalar("SELECT pgmq.delete($1, $2)")
            .bind(queue)
            .bind(msg_id)
            .fetch_one(&self.worker_pool)
            .await
            .map_err(map_err)?;
        Ok(ok)
    }

    async fn archive(&self, queue: &str, msg_id: i64) -> Result<bool, QueueError> {
        let ok: bool = sqlx::query_scalar("SELECT pgmq.archive($1, $2)")
            .bind(queue)
            .bind(msg_id)
            .fetch_one(&self.worker_pool)
            .await
            .map_err(map_err)?;
        Ok(ok)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    //! These tests do NOT touch Postgres; they only verify trait wiring +
    //! error mapping. End-to-end behavior is covered by
    //! `tests/pgmq_integration.rs` (#[ignore = "needs Docker + pgmq"]).

    use super::*;

    #[test]
    fn map_err_preserves_message() {
        let e = sqlx::Error::PoolTimedOut;
        let mapped = map_err(e);
        match mapped {
            QueueError::Backend(s) => assert!(s.contains("pool")),
            QueueError::NotFound { .. } => panic!("expected Backend, got NotFound"),
        }
    }
}
