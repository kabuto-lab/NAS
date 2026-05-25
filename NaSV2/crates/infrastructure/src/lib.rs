//! `nas2-infrastructure` · L4 adapter implementations · ENTITY §2, §4
//!
//! Содержит: SQLx repositories (PostRepository impl, UserRepository impl, ...),
//! S3/local storage adapter, pgmq queue adapter, email (lettre) adapter,
//! Postgres FTS search adapter, moka cache adapter.
//!
//! Все public API НЕ возвращают `sqlx::Row`, `PgPool`, `Pg*` — только domain
//! aggregates и port trait responses. Enforced via xtask architecture-check.
//!
//! Implementation pending.

#![forbid(unsafe_code)]

pub mod queue;
// pub mod persistence;
// pub mod storage;
// pub mod email;
// pub mod search;
// pub mod cache;
