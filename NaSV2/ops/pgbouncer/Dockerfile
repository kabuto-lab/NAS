# syntax=docker/dockerfile:1.7
# ops/pgbouncer/Dockerfile — production-ready PgBouncer image for AX•CMS.
#
# ENTITY §3.4 — PgBouncer in transaction mode is the only sanctioned pool
# tier. §3.4.2 — three isolated logical pools (http / worker / admin).
#
# The base image is the well-maintained edoburu fork; pinning a major.minor
# tag keeps reproducible builds while letting Renovate bump patches.
#
# This image is the "bake" for production overrides. Local dev still uses
# `docker-compose.dev.yml` + `docker-compose.override.yml` which mount
# `pgbouncer.ini` from this directory; production swaps env via the
# deployment manifest, NOT by rebuilding configs into the image.
#
# Closes recommendation R2 from `SESSION_LOG.md` (AVTONOM 2026-05-25).
# Referenced by `docs/plans/PLAN-003-pgbouncer-split-pool.md`.

FROM edoburu/pgbouncer:1.23.1

# Default configs baked in for reproducible deploys; the runtime may override
# any file by mounting a ConfigMap/Secret at the same path.
COPY pgbouncer.ini /etc/pgbouncer/pgbouncer.ini
COPY databases.ini /etc/pgbouncer/databases.ini
COPY userlist.txt  /etc/pgbouncer/userlist.txt

# AUTH_TYPE for prod MUST be promoted to `scram-sha-256` via the deploy env
# override; the baked default keeps local-dev `trust` so contributors do not
# need to manage hashes. The override path is:
#   docker-compose.prod.yml > environment: AUTH_TYPE: scram-sha-256
EXPOSE 5432

# Healthcheck — psql is not present in the slim base; rely on TCP probe.
HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD nc -z localhost 5432 || exit 1
