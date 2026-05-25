-- 0001_pgmq_bootstrap.sql
-- ENTITY §3.8 — bootstrap pgmq + create the three core AX queues.
--
-- pgmq tables are created under schema `pgmq`. Each `pgmq.create(name)` call
-- materializes:
--   pgmq.q_<name>       — live queue
--   pgmq.a_<name>       — archive
-- Idempotent: pgmq.create is a no-op when the queue already exists.
--
-- Applied via the admin_pool (ENTITY §3.4.2) — runs DDL.

CREATE EXTENSION IF NOT EXISTS pgmq CASCADE;

SELECT pgmq.create('ax_image_jobs');
SELECT pgmq.create('ax_email_outbox');
SELECT pgmq.create('ax_search_reindex');
