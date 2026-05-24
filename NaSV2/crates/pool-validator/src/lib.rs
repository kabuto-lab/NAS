//! `nas2-pool-validator` · AX•CMS
//!
//! **Purpose** — implement the §3.4.1 Pool-Mode Contract.
//!
//! The server MUST refuse to boot if PgBouncer is not in `transaction` mode,
//! because RLS via `set_config('app.tenant_id', ..., true)` requires that the
//! `SET` and the `SELECT` share the same backend connection — a guarantee
//! only `transaction` (or `session`) mode provides, and `session` mode breaks
//! connection multiplexing.
//!
//! ## TLA layers
//! - **L1 Correctness** — query result compared against literal `"transaction"`.
//! - **L2 Performance** — single roundtrip at boot; not on hot path.
//! - **L3 Scalability** — one call per `PgPool` (3 pools × N instances).
//! - **L4 Operability** — emits `event=pool_mode_drift` on failure; exposed
//!   on `/health/pool` via [`PoolMode::is_transaction`].

#![forbid(unsafe_code)]

use sqlx::{Pool, Postgres};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolMode {
    Transaction,
    Session,
    Statement,
    Unknown(String),
}

impl PoolMode {
    #[must_use]
    pub const fn is_transaction(&self) -> bool {
        matches!(self, Self::Transaction)
    }

    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "transaction" => Self::Transaction,
            "session" => Self::Session,
            "statement" => Self::Statement,
            other => Self::Unknown(other.to_owned()),
        }
    }
}

#[derive(Debug, Error)]
pub enum PoolValidationError {
    #[error("pgbouncer pool_mode is {actual:?}, expected `transaction` (ENTITY §3.4.1)")]
    WrongMode { actual: PoolMode },

    #[error("query SHOW pool_mode failed: {0}")]
    Query(#[from] sqlx::Error),
}

/// Issues `SHOW pool_mode;` against the given pool and returns the parsed result.
///
/// Use [`ensure_transaction_mode`] when you want a fail-fast boot check.
pub async fn detect_pool_mode(pool: &Pool<Postgres>) -> Result<PoolMode, PoolValidationError> {
    // `SHOW pool_mode;` is a PgBouncer admin-protocol-compatible statement that
    // PgBouncer answers on the `pgbouncer` virtual database. When connected to
    // an application database, PgBouncer transparently relays the equivalent.
    let row: (String,) = sqlx::query_as("SHOW pool_mode").fetch_one(pool).await?;
    Ok(PoolMode::from_str(&row.0))
}

/// Hard gate (§3.4.1). Returns `Ok(())` only if `pool_mode == transaction`.
///
/// Call this from `apps/server/src/main.rs` before binding the listener.
/// On failure the caller MUST refuse to boot and emit a `severity=critical`
/// trace event before exiting non-zero.
// `tracing::error!` macro expansion inflates cognitive complexity above the
// workspace budget of 15; the function body is otherwise trivial (one branch).
#[allow(clippy::cognitive_complexity)]
pub async fn ensure_transaction_mode(pool: &Pool<Postgres>) -> Result<(), PoolValidationError> {
    let mode = detect_pool_mode(pool).await?;
    if mode.is_transaction() {
        tracing::info!(pool_mode = ?mode, "pool-mode contract satisfied");
        Ok(())
    } else {
        tracing::error!(
            event = "pool_mode_drift",
            severity = "critical",
            actual = ?mode,
            expected = "transaction",
            "ENTITY §3.4.1 violated — refusing to boot"
        );
        Err(PoolValidationError::WrongMode { actual: mode })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_modes() {
        assert!(PoolMode::from_str("transaction").is_transaction());
        assert!(!PoolMode::from_str("session").is_transaction());
        assert!(!PoolMode::from_str("statement").is_transaction());
        assert!(matches!(PoolMode::from_str("weird"), PoolMode::Unknown(_)));
    }

    #[test]
    fn case_and_whitespace_tolerant() {
        assert!(PoolMode::from_str("  Transaction  ").is_transaction());
        assert!(PoolMode::from_str("TRANSACTION").is_transaction());
    }
}
