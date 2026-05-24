//! `ax-common` · Cross-crate shared types · `ENTITY.md §5`
//!
//! Содержит типы которые используются всеми слоями (L1-L4):
//! - [`TenantId`], [`TenantContext`] — multi-tenancy идентификация
//! - [`UserId`], [`RequestId`] — bounded identifiers
//! - [`AppError`] — централизованный error enum с HTTP mapping
//! - [`Page<T>`] — keyset cursor pagination (см. `ENTITY.md §11.6` — `OFFSET` запрещён)

pub mod capability;
pub mod error;
pub mod ids;
pub mod page;
pub mod role;
pub mod tenant;

pub use capability::{Capability, CapabilitySet, UnknownCapability};
pub use error::{AppError, NotFoundDetail};
pub use ids::{RequestId, UserId};
pub use page::Page;
pub use role::{Role, RoleKey, RoleKeyError};
pub use tenant::{TenantContext, TenantId, TenantStatus};
