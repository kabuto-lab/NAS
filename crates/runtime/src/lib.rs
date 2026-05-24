//! `ax-runtime` · TaskSupervisor + concurrency primitives · `ENTITY.md §4.8`
//!
//! Запрещает raw `tokio::spawn` вне этого crate (enforced clippy disallowed-methods).
//! Все long-running async tasks идут через `TaskSupervisor.spawn(...)`.
//!
//! Phase A scope — пустой stub; реальная implementation в Phase B+ когда появятся
//! background workers (queue consumer, image processing, report generation).

use tokio_util::sync::CancellationToken;

/// Category для runtime isolation per `ENTITY.md §10.5`.
#[derive(Clone, Copy, Debug)]
pub enum TaskCategory {
    /// HTTP request handling — main runtime
    Http,
    /// Queue workers (pgmq consumer)
    Queue,
    /// CPU-heavy: image processing, PDF rendering
    Image,
    /// Background reports
    Report,
}

/// `TaskSupervisor` — единственный entry-point для spawn'а tasks (Phase B+).
///
/// Phase A: stub, не используется. В Phase B заменяется на полную реализацию
/// с tracking, cancellation, graceful drain.
#[derive(Debug)]
pub struct TaskSupervisor {
    category: TaskCategory,
    cancel: CancellationToken,
}

impl TaskSupervisor {
    #[must_use]
    pub fn new(category: TaskCategory) -> Self {
        Self {
            category,
            cancel: CancellationToken::new(),
        }
    }

    #[must_use]
    pub const fn category(&self) -> TaskCategory {
        self.category
    }

    #[must_use]
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    pub fn cancel(&self) {
        self.cancel.cancel();
    }
}
