//! `ax-common` · Cross-crate shared types · `ENTITY.md §5`
//!
//! Содержит типы которые используются всеми слоями (L1-L4):
//! - [`TenantId`], [`TenantContext`] — multi-tenancy идентификация
//! - [`UserId`], [`RequestId`] — bounded identifiers
//! - [`AppError`] — централизованный error enum с HTTP mapping
//! - [`Page<T>`] — keyset cursor pagination (см. `ENTITY.md §11.6` — `OFFSET` запрещён)

pub mod tenant;
pub mod ids;
pub mod error;
pub mod page;

pub use tenant::{TenantId, TenantContext, TenantStatus};
pub use ids::{UserId, RequestId};
pub use error::{AppError, NotFoundDetail};
pub use page::Page;
