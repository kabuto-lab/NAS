//! `ax-domain` · L3 pure domain types · `ENTITY.md §2`
//!
//! Содержит aggregates, value objects, domain events.
//! ZERO зависимостей кроме serde / uuid / chrono / garde — enforced
//! `cargo xtask architecture-check`.

pub mod cms;
pub mod user;
