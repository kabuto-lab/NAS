//! pgmq integration — send → read → delete + archive round-trip.
//!
//! All tests are gated `#[ignore = "needs Docker + pgmq"]`. Run explicitly:
//!
//! ```bash
//! PGMQ_IMAGE=ghcr.io/tembo-io/pgmq:latest \
//!   cargo test -p nas2-infrastructure --tests pgmq_integration -- --ignored
//! ```
//!
//! ### Image selection
//!
//! Stock `postgres:16` does NOT ship pgmq. Override `PGMQ_IMAGE` to point at
//! a Tembo or self-built image where the extension is preinstalled. When the
//! env var is missing, the tests skip with a `println!` rather than failing
//! — they exist primarily to validate the adapter compiles against the
//! `Queue` port.

#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::env;

use nas2_application::ports::Queue;
use nas2_infrastructure::queue::PgmqQueue;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use testcontainers::{
    core::{ContainerPort, WaitFor},
    runners::AsyncRunner,
    GenericImage, ImageExt,
};

const QUEUE: &str = "ax_integration_test";

async fn boot_pgmq() -> Option<(testcontainers::ContainerAsync<GenericImage>, PgmqQueue)> {
    let image_ref = env::var("PGMQ_IMAGE").ok()?;
    let (image_name, tag) = image_ref.split_once(':').unwrap_or((&image_ref, "latest"));
    let image = GenericImage::new(image_name, tag)
        .with_exposed_port(ContainerPort::Tcp(5432))
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_PASSWORD", "pgmq")
        .with_env_var("POSTGRES_USER", "pgmq")
        .with_env_var("POSTGRES_DB", "pgmq");

    let container = image.start().await.expect("start pgmq container");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("expose 5432");
    let url = format!("postgres://pgmq:pgmq@127.0.0.1:{port}/pgmq");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect to pgmq postgres");
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pgmq CASCADE")
        .execute(&pool)
        .await
        .expect("create extension pgmq");
    sqlx::query("SELECT pgmq.create($1)")
        .bind(QUEUE)
        .execute(&pool)
        .await
        .expect("create test queue");

    Some((container, PgmqQueue::new(pool)))
}

#[tokio::test]
#[ignore = "needs Docker + pgmq image (set PGMQ_IMAGE env)"]
async fn send_read_delete_round_trip() {
    let Some((_container, q)) = boot_pgmq().await else {
        println!("PGMQ_IMAGE not set — skipping pgmq integration test");
        return;
    };
    let id = q
        .send(QUEUE, &json!({"job": "thumb", "w": 320}))
        .await
        .expect("send");
    assert!(id > 0);

    let batch = q.read(QUEUE, 30, 10).await.expect("read");
    assert_eq!(batch.len(), 1);
    let msg = &batch[0];
    assert_eq!(msg.msg_id, id);
    assert_eq!(msg.read_ct, 1);
    assert_eq!(msg.payload["job"], "thumb");

    assert!(q.delete(QUEUE, id).await.expect("delete"));
    let after = q.read(QUEUE, 30, 10).await.expect("read after delete");
    assert!(after.is_empty(), "queue should be empty after delete");
}

#[tokio::test]
#[ignore = "needs Docker + pgmq image (set PGMQ_IMAGE env)"]
async fn send_read_archive_round_trip() {
    let Some((_container, q)) = boot_pgmq().await else {
        println!("PGMQ_IMAGE not set — skipping pgmq integration test");
        return;
    };
    let id = q
        .send(QUEUE, &json!({"job": "archive-me"}))
        .await
        .expect("send");
    let batch = q.read(QUEUE, 30, 1).await.expect("read");
    assert_eq!(batch.len(), 1);
    assert!(q.archive(QUEUE, id).await.expect("archive"));
}
