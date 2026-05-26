//! `nas2-tenant` · Tenant resolver middleware · ENTITY §3, §15.
//!
//! Today: `TenantContext` + `TenantResolver` trait + in-memory impl +
//! axum `resolve_tenant` middleware.
//!
//! Deferred to M2: `PgTenantResolver` with moka LRU (5-min TTL) and the
//! `with_tenant(pool, ctx, |tx| ...)` helper that issues
//! `set_config('app.tenant_id', $1, true)` inside the business
//! transaction for RLS propagation (ENTITY §3.5 single-RTT contract).

#![forbid(unsafe_code)]

pub mod context;
pub mod middleware;
pub mod resolver;

pub use context::TenantContext;
pub use middleware::{TenantResolverHandle, resolve_tenant};
pub use resolver::{InMemoryTenantResolver, InMemoryTenantResolverBuilder, TenantResolver};
