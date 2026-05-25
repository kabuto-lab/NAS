//! `Queue` port — abstraction over a transactional message queue.
//!
//! The production adapter is the pgmq adapter in `crates/infrastructure/queue`.
//! Tests use [`mockall`]-derived doubles (added per-test).
//!
//! ## TLA layers
//!
//! - **L1 Correctness** — `send` either commits its row or returns Err; no
//!   half-state. `delete`/`archive` are explicit, never implicit on read.
//! - **L2 Performance** — `read(qty)` is one roundtrip; callers batch.
//! - **L3 Scalability** — pgmq sits on Postgres, sharing the §3.4.2
//!   `worker_pool` budget; no extra connection pool to size.
//! - **L4 Operability** — `vt_seconds` (visibility timeout) is explicit so
//!   ops can tune redelivery without code changes.

use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;

/// A message read from a queue together with its delivery metadata.
#[derive(Debug, Clone)]
pub struct QueueMessage {
    /// pgmq-assigned per-queue message id (monotonic per queue).
    pub msg_id: i64,
    /// Delivery attempt count (pgmq tracks this server-side).
    pub read_ct: i32,
    /// Application payload.
    pub payload: Value,
}

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("queue '{queue}' not found")]
    NotFound { queue: String },

    #[error("queue backend error: {0}")]
    Backend(String),
}

/// Transactional message queue port.
///
/// Implementations MUST be Send + Sync so they can live in `AppState`.
#[async_trait]
pub trait Queue: Send + Sync {
    /// Append a JSON payload to `queue`. Returns the pgmq message id.
    async fn send(&self, queue: &str, payload: &Value) -> Result<i64, QueueError>;

    /// Read up to `qty` messages, holding each invisible for `vt_seconds`.
    async fn read(
        &self,
        queue: &str,
        vt_seconds: i32,
        qty: i32,
    ) -> Result<Vec<QueueMessage>, QueueError>;

    /// Permanently delete a previously-read message. Returns `true` if removed.
    async fn delete(&self, queue: &str, msg_id: i64) -> Result<bool, QueueError>;

    /// Move a message to the archive table (kept for audit / DLQ inspection).
    async fn archive(&self, queue: &str, msg_id: i64) -> Result<bool, QueueError>;
}

#[cfg(test)]
mod tests {
    //! Compile-time check: the trait is dyn-safe so it can be held as a
    //! `Box<dyn Queue>` in `AppState` for test/prod substitution.

    use super::*;

    #[allow(dead_code)]
    fn _assert_object_safe(_q: &dyn Queue) {}
}
