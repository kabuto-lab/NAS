//! `TaskSupervisor` — bounded spawn governance · ENTITY §4.8, §29
//!
//! **The only place** in the workspace where `tokio::spawn` is permitted; all
//! other async tasks MUST go through [`TaskSupervisor::spawn`]. Enforced via
//! `xtask magic-check` (regex scan for raw `tokio::spawn`) and clippy
//! `disallowed-methods` at the workspace boundary.
//!
//! ## Design
//!
//! Each supervisor owns:
//! - a [`TaskCategory`] tag (used for tracing fields and failure-domain isolation)
//! - a [`CancellationToken`] propagated to every spawned task
//! - a [`TaskTracker`] that counts in-flight tasks for [`drain`](Self::drain)
//!
//! ## TLA layers
//!
//! - **L1 Correctness** — `drain` is bounded by a hard deadline; cancellation
//!   is cooperative, not pre-emptive, so callers MUST honor the token.
//! - **L2 Performance** — one `Arc` per supervisor (TaskTracker is internally
//!   `Arc`); no per-spawn allocation beyond the future itself.
//! - **L3 Scalability** — one supervisor per failure domain isolates blast
//!   radius (a runaway image worker cannot starve HTTP).
//! - **L4 Operability** — every spawn is wrapped in a `tracing::Span` carrying
//!   `category`, `name`, and a per-task UUID for correlation across logs.

use std::{fmt, time::Duration};

use thiserror::Error;
use tokio::task::JoinHandle;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::Instrument as _;
use uuid::Uuid;

/// Failure-domain classification for a [`TaskSupervisor`].
///
/// One supervisor per category isolates blast radius: a stuck image worker
/// cannot starve HTTP, a dead-lettered email job cannot back up search indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskCategory {
    Http,
    Queue,
    Image,
    Report,
    Email,
    SearchIndex,
}

impl TaskCategory {
    /// Static string form used as a `tracing` field value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Queue => "queue",
            Self::Image => "image",
            Self::Report => "report",
            Self::Email => "email",
            Self::SearchIndex => "search_index",
        }
    }
}

impl fmt::Display for TaskCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned by [`TaskSupervisor::drain`] when the deadline expires.
#[derive(Debug, Error)]
pub enum DrainError {
    #[error("supervisor '{category}' drain timed out after {elapsed:?} with {in_flight} task(s) still running")]
    Timeout {
        category: TaskCategory,
        elapsed: Duration,
        in_flight: usize,
    },
}

/// Bounded spawn governor for a single failure domain.
///
/// Clone-cheap: the underlying [`TaskTracker`] and [`CancellationToken`] are
/// reference-counted, so child handlers may hold their own clones without
/// affecting drain semantics.
#[derive(Debug, Clone)]
pub struct TaskSupervisor {
    category: TaskCategory,
    cancel: CancellationToken,
    tracker: TaskTracker,
}

impl TaskSupervisor {
    /// Create a fresh supervisor for the given failure domain.
    #[must_use]
    pub fn new(category: TaskCategory) -> Self {
        Self {
            category,
            cancel: CancellationToken::new(),
            tracker: TaskTracker::new(),
        }
    }

    /// The failure domain this supervisor governs.
    #[must_use]
    pub const fn category(&self) -> TaskCategory {
        self.category
    }

    /// Clone of the supervisor's cancellation token.
    ///
    /// Spawned tasks already receive this token via the span; this accessor
    /// exists for tasks that want to propagate cancellation into child
    /// primitives (e.g. `tokio::select!` arms, gRPC streams).
    #[must_use]
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Spawn a future under this supervisor.
    ///
    /// The future runs inside a `tracing::Span` carrying `category`, `name`,
    /// and a unique `task_id` for correlation. The supervisor's
    /// [`TaskTracker`] counts this task until completion.
    ///
    /// Returns a [`TaskHandle`] wrapping the underlying [`JoinHandle`].
    pub fn spawn<F>(&self, name: &'static str, fut: F) -> TaskHandle
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let task_id = Uuid::new_v4();
        let category = self.category;
        let span = tracing::info_span!(
            "supervised_task",
            category = category.as_str(),
            name,
            task_id = %task_id,
        );
        let inner = self.tracker.spawn(fut.instrument(span));
        TaskHandle {
            inner,
            task_id,
            category,
            name,
        }
    }

    /// Initiate graceful drain.
    ///
    /// Triggers the cancellation token, closes the tracker (no new spawns
    /// admitted), and waits for outstanding tasks up to `timeout`. On
    /// expiry, returns [`DrainError::Timeout`] carrying the in-flight count
    /// at the moment of the deadline; the caller is responsible for the
    /// subsequent forced-abort policy (ENTITY §29 — bounded shutdown).
    ///
    /// # Errors
    /// Returns [`DrainError::Timeout`] if outstanding tasks fail to complete
    /// within `timeout`.
    pub async fn drain(self, timeout: Duration) -> Result<(), DrainError> {
        self.cancel.cancel();
        self.tracker.close();
        match tokio::time::timeout(timeout, self.tracker.wait()).await {
            Ok(()) => Ok(()),
            Err(_) => Err(DrainError::Timeout {
                category: self.category,
                elapsed: timeout,
                in_flight: self.tracker.len(),
            }),
        }
    }
}

/// Handle to a single supervised task. Mostly transparent over [`JoinHandle`].
#[derive(Debug)]
pub struct TaskHandle {
    inner: JoinHandle<()>,
    task_id: Uuid,
    category: TaskCategory,
    name: &'static str,
}

impl TaskHandle {
    #[must_use]
    pub const fn task_id(&self) -> Uuid {
        self.task_id
    }

    #[must_use]
    pub const fn category(&self) -> TaskCategory {
        self.category
    }

    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Best-effort abort; tasks that have already completed are unaffected.
    pub fn abort(&self) {
        self.inner.abort();
    }

    /// Await task completion.
    ///
    /// # Errors
    /// Propagates [`tokio::task::JoinError`] from the underlying handle.
    pub async fn join(self) -> Result<(), tokio::task::JoinError> {
        self.inner.await
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use super::*;

    #[tokio::test]
    async fn test_spawn_and_drain_completes() {
        let sup = TaskSupervisor::new(TaskCategory::Queue);
        let counter = Arc::new(AtomicUsize::new(0));
        for _ in 0..10 {
            let c = Arc::clone(&counter);
            sup.spawn("noop", async move {
                tokio::time::sleep(Duration::from_millis(5)).await;
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        sup.drain(Duration::from_secs(1))
            .await
            .expect("drain must succeed within budget");
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn test_drain_times_out_on_stuck_task() {
        let sup = TaskSupervisor::new(TaskCategory::Image);
        sup.spawn("stuck", async move {
            // Intentionally ignores cancel_token to simulate misbehavior.
            std::future::pending::<()>().await;
        });
        let err = sup
            .drain(Duration::from_millis(50))
            .await
            .expect_err("stuck task must trip the timeout");
        match err {
            DrainError::Timeout {
                category,
                in_flight,
                ..
            } => {
                assert_eq!(category, TaskCategory::Image);
                assert_eq!(in_flight, 1);
            },
        }
    }

    #[tokio::test]
    async fn test_cancel_token_propagates() {
        let sup = TaskSupervisor::new(TaskCategory::SearchIndex);
        let observed_cancel = Arc::new(AtomicUsize::new(0));
        let token = sup.cancel_token();
        let obs = Arc::clone(&observed_cancel);
        sup.spawn("respects-cancel", async move {
            token.cancelled().await;
            obs.fetch_add(1, Ordering::SeqCst);
        });
        // Give the task a tick to park on `cancelled().await`.
        tokio::time::sleep(Duration::from_millis(5)).await;
        sup.drain(Duration::from_millis(200))
            .await
            .expect("respectful task must observe cancel and complete");
        assert_eq!(observed_cancel.load(Ordering::SeqCst), 1);
    }
}
