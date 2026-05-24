//! User aggregate + value objects.
//!
//! Spec: `ENTITY.md §7`, `ADR-002 §D8`.
//!
//! User является tenant-scoped aggregate с invariants:
//! - email унікален global (NOT per-tenant — соответствует SITE1)
//! - status ∈ {active, suspended, archived}
//! - tenant_id NOT NULL

pub mod aggregate;

pub use aggregate::{User, UserError, UserStatus};
