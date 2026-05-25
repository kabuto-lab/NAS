//! `nas2-runtime` · `TaskSupervisor` + concurrency primitives · ENTITY §4.8, §29
//!
//! **Единственное место** где разрешён `tokio::spawn`. Все async tasks вне этого
//! crate'а должны идти через `TaskSupervisor::spawn(...)`. Enforced clippy
//! disallowed-methods + xtask magic-check.
//!
//! Содержит:
//! - `TaskSupervisor` — bounded spawn с tracking, cancellation token, graceful drain
//! - `TaskCategory` enum: Http | Queue | Image | Report | Email | SearchIndex
//! - Failure domain isolation per category (§29)
//!
//! Implementation pending.

#![forbid(unsafe_code)]

pub mod observability;
pub mod supervisor;

pub use supervisor::{DrainError, TaskCategory, TaskHandle, TaskSupervisor};
