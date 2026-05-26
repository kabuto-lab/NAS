//! Port traits — application-layer abstractions over infrastructure adapters.
//!
//! Adapters live in `crates/infrastructure`; tests substitute mocks here.
//! ENTITY §2 (hex layers), §8 (port discipline).

pub mod media_repository;
pub mod post_repository;
pub mod queue;
pub mod site_repository;
pub mod user_repository;

pub use media_repository::MediaRepository;
pub use post_repository::PostRepository;
pub use queue::{Queue, QueueError, QueueMessage};
pub use site_repository::SiteRepository;
pub use user_repository::UserRepository;

#[cfg(test)]
pub use media_repository::MockMediaRepository;
#[cfg(test)]
pub use post_repository::MockPostRepository;
#[cfg(test)]
pub use site_repository::MockSiteRepository;
#[cfg(test)]
pub use user_repository::MockUserRepository;
