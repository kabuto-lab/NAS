//! `nas2-domain` · L3 pure domain types · ENTITY §2, §7.
//!
//! **Allow-list (ENTITY §2.6):** `serde`, `uuid`, `chrono`, `garde`,
//! `thiserror`, `nas2-common`. No async. No IO. No tokio.
//!
//! Enforced at CI time by `cargo xtask architecture-check`.

#![forbid(unsafe_code)]

pub mod block;
pub mod capability;
pub mod post;
pub mod role;
pub mod site;
pub mod user;
// pub mod taxonomy;      // deferred to month 2
// pub mod media;         // deferred

pub use block::Block;
pub use capability::{Capability, CapabilitySet};
pub use post::{CustomPostType, Post, PostSlug, PostStatus};
pub use role::Role;
pub use site::{Site, SiteSlug};
pub use user::{Email, User};
