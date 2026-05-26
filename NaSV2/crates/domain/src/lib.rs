//! `nas2-domain` · L3 pure domain types · ENTITY §2, §7.
//!
//! **Allow-list (ENTITY §2.6):** `serde`, `uuid`, `chrono`, `garde`,
//! `thiserror`, `nas2-common`. No async. No IO. No tokio.
//!
//! Enforced at CI time by `cargo xtask architecture-check`.

#![forbid(unsafe_code)]

pub mod block;
pub mod post;
pub mod site;
// pub mod user;          // W2 D3
// pub mod role;          // W2 D3
// pub mod capability;    // W2 D3
// pub mod taxonomy;      // deferred to month 2
// pub mod media;         // deferred

pub use block::Block;
pub use post::{CustomPostType, Post, PostSlug, PostStatus};
pub use site::{Site, SiteSlug};
