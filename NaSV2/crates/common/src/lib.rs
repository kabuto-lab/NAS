//! `nas2-common` · L0 cross-crate types · ENTITY §5.
//!
//! Public surface kept *minimal* by design — every type here is consumed
//! by every downstream layer, so additions ripple. New shapes need an RFC.

#![forbid(unsafe_code)]

pub mod ids;
pub mod page;

pub use ids::{MediaId, PostId, RequestId, RoleId, SiteId, TenantId, UserId};
pub use page::Page;
