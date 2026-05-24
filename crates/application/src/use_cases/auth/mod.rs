//! Auth use cases — capability resolution (cached).
//!
//! Note: JWT verification itself остаётся в infrastructure (`JwtVerifier`) и
//! вызывается middleware напрямую. Wrapper use case был бы pure-pass-through —
//! не оправдывает ceremony. См. AI-Default в SESSION_LOG.

pub mod resolve_capabilities;

pub use resolve_capabilities::CapabilityResolver;
