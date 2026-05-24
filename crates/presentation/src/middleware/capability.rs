//! Capability check — helper for handlers, not a middleware layer.
//!
//! Per ADR-002 §D6 (lazy resolve in handler), capability check is invoked
//! inside the handler after RequireTenant + RequireAuthenticated extractors,
//! using the shared `CapabilityResolver` (moka-cached).
//!
//! Usage:
//! ```ignore
//! state.capability_resolver
//!     .require(&ctx, user.user_id, Capability::PostsCreate)
//!     .await?;
//! ```

// Module intentionally tiny — keeps capability check pattern documented in one place.
// Re-export for ergonomics.

pub use ax_application::use_cases::auth::CapabilityResolver;
pub use ax_common::Capability;
