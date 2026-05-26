//! Query use cases — read-side composition of port traits.
//!
//! Commands (write-side) live in `crates/application/src/commands` (to
//! be created). Use cases are the *only* place where domain types are
//! composed with capability checks; handlers MUST go through a use case
//! rather than calling a repository directly.

pub mod get_published_page_by_slug;

pub use get_published_page_by_slug::GetPublishedPageBySlug;
