//! `nas2-application` · L2 use cases + port traits · ENTITY §2, §8.
//!
//! Contains: port traits (`PostRepository`, `UserRepository`, ...),
//! use cases (commands + queries), capability check helpers, hook
//! registry interface.

#![forbid(unsafe_code)]

pub mod ports;
pub mod queries;
// pub mod commands;
// pub mod services;
// pub mod hooks;
// pub mod capability;
