//! Port traits — application-layer abstractions over infrastructure adapters.
//!
//! Adapters live in `crates/infrastructure`; tests substitute mocks here.
//! ENTITY §2 (hex layers), §8 (port discipline).

pub mod queue;

pub use queue::{Queue, QueueError, QueueMessage};
