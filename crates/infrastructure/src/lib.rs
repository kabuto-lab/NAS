//! `ax-infrastructure` · L4 adapter implementations · `ENTITY.md §2`
//!
//! Содержит SQLx repos, S3 client, queue (Phase B+), email (Phase B+).
//! Implements port traits из `ax-application::ports`.

pub mod persistence;
pub mod tenant;

pub use tenant::PgTenantResolver;
