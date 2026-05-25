//! Infrastructure queue adapters — concrete implementations of the
//! `application::ports::Queue` port.
//!
//! Currently only the pgmq (Postgres) adapter is wired; NATS JetStream lands
//! when high-volume eventing arrives (ENTITY §3.8).

pub mod pgmq;

pub use pgmq::PgmqQueue;
