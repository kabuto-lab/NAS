//! Authentication infrastructure — JWT verification, future password hashing.
//!
//! Phase A scope: JWT verify only (HS256, shared secret with SITE1).

pub mod jwt;

pub use jwt::{Claims, JwtVerifier};
