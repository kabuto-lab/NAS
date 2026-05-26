//! `nas2-domain` · L3 pure domain types · ENTITY §2, §7.
//!
//! **Allow-list (ENTITY §2.6):** `serde`, `uuid`, `chrono`, `garde`,
//! `thiserror`, `nas2-common`. No async. No IO. No tokio.
//!
//! Enforced at CI time by `cargo xtask architecture-check`.

#![forbid(unsafe_code)]

pub mod site;
// pub mod post;          // W2 D2
// pub mod user;          // W2 D3
// pub mod role;          // W2 D3
// pub mod capability;    // W2 D3
// pub mod block;         // W2 D2 (placeholder) + W2 D4 (real)
// pub mod taxonomy;      // deferred to month 2
// pub mod media;         // deferred

pub use site::{Site, SiteSlug};
