//! `nas2-domain` · L3 pure domain types · ENTITY §2, §7
//!
//! Содержит: `Site`, `Post` (с CustomPostType), `User`/`Role`/`Capability`,
//! `Taxonomy`/`Term`, `Media`, `Block` (typed enum для content blocks).
//!
//! **ZERO зависимостей** кроме serde/uuid/chrono/garde — enforced
//! `cargo xtask architecture-check`.
//!
//! Implementation pending. См. ENTITY §7 domain model, §11 block editor.

#![forbid(unsafe_code)]

// pub mod site;
// pub mod post;
// pub mod user;
// pub mod role;
// pub mod capability;
// pub mod taxonomy;
// pub mod media;
// pub mod block;
