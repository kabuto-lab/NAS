//! Public router-level health endpoints · ENTITY §4.
//!
//! These are **presentation** health endpoints (router-level liveness).
//! Lower-level health (pool-mode) lives in `apps/server` and is
//! deliberately separate so a router fault doesn't mask a pool fault.

pub async fn live() -> &'static str {
    "ok"
}

pub async fn ready() -> &'static str {
    "ready"
}
