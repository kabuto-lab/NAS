//! Per-request tenant context · ENTITY §15.
//!
//! Resolved once per request by `resolve_tenant` middleware and
//! propagated to handlers via `request.extensions().insert(...)`.
//!
//! Holding a `TenantContext` is the type-level proof that the request
//! has been associated with a tenant — handlers requiring it as an
//! `axum::Extension` fail at extract-time if the middleware was not
//! mounted.

use nas2_common::{SiteId, TenantId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub site_id: SiteId,
}

impl TenantContext {
    #[must_use]
    pub const fn new(tenant_id: TenantId, site_id: SiteId) -> Self {
        Self { tenant_id, site_id }
    }
}
