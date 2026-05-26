# SEC-002 — Capability surface (issuance, consumption, cache binding)

- **Status:** Specification (forward-looking — RBAC tables land M2 W3)
- **Date:** 2026-06-18
- **Owner:** AX•ARCHITECT (Sentinel + Adversary co-signed at first JWT day)
- **References:** ENTITY §14, §3.9.1; `crates/domain/src/capability.rs`; [SEC-001](SEC-001-sanitization-pipeline.md); [SEC-003](SEC-003-multi-tenant-isolation.md); FM-006 in `memory/sentinel_init.md`

## §1 — Enumerated capabilities (M1 surface)

The `Capability` enum (`crates/domain/src/capability.rs`) is the
complete authoritative list. New capabilities require a domain change
+ an `xtask capability-coverage` re-run.

| Enum variant         | Wire string         | Semantic                                                                             |
|----------------------|---------------------|--------------------------------------------------------------------------------------|
| `CmsPageRead`        | `cms.page.read`     | Read a published page (and drafts the holder owns); the M1 W3/W4 handler gates here. |
| `CmsPagePublish`     | `cms.page.publish`  | Transition `Draft|Scheduled → Published` via the M2 W6+ write path.                  |
| `CmsPageDraft`       | `cms.page.draft`    | Create / save Draft; the editor surface uses this.                                   |
| `MediaUpload`        | `media.upload`      | Upload through the M3 W3+ media pipeline.                                            |
| `ManageUsers`        | `manage.users`      | Issue/revoke roles, invite users (admin shell, M5 W4).                               |
| `ManageSite`         | `manage.site`       | Site-wide settings (themes, sanitization profile, custom post types).                |

Wire strings follow WordPress's `current_user_can` dot-convention
(migration familiarity for plugin authors). The strings are stable;
the enum variants may be reordered without wire impact because the
mapping is in `Capability::as_str` + `Capability::try_from_str`.

### M2+ commerce + CRM capabilities (per MPD-001)

Per `docs/governance/master-plan-diffs/MPD-001-commerce-crm-pivot.md`,
the enum gains nine additional variants in M2 W6 D1 during the
domain restructure:

- `commerce.product.read`, `commerce.product.publish`, `commerce.product.draft`
- `commerce.order.read`, `commerce.order.fulfill`
- `crm.contact.read`, `crm.contact.write`
- `crm.lead.read`, `crm.lead.write`

This doc updates at that landing.

## §2 — Issuance (who hands capabilities out)

- **Admin role.** A user with `ManageUsers` is the only principal
  allowed to construct or assign a role. The `admin@nas2-dev.local`
  seed (CLI `db seed-dev`) bootstraps the first one.
- **Role grants.** Roles are tenant-scoped collections of
  capabilities. A role is *issued* by `INSERT INTO roles
  (tenant_id, …)` and *assigned* by `INSERT INTO user_roles
  (user_id, role_id, …)`. The migration that lands these tables is
  scheduled for M2 W2 D2 (`migrations/0007_roles.sql`).
- **Capabilities cannot be assigned to a user directly.** Always
  via a role — Sentinel-mandated indirection so revocation is
  one statement.

## §3 — Consumption (who checks capabilities)

- **HTTP handlers.** Every state-changing handler under
  `crates/presentation/src/api/**` MUST carry either a
  `caps.require(<cap>)` marker or `no_capability_required:
  <reason>` opt-out (see `xtask capability-coverage`, W4 D2).
- **Use cases.** `crates/application/src/queries/get_published_page_by_slug.rs`
  takes a `&CapabilitySet` parameter and gates the read before
  the repo call. This is the **defense-in-depth layer** above
  RLS (which gates at the row level via `set_config(...)`).
- **Workers.** pgmq consumers run inside `TaskCategory::Queue`;
  their capability gating is implicit (any payload that lands in
  the queue was authorized at write time). Worker code MUST NOT
  re-derive capabilities from JWTs or external sources.

## §4 — Cache-key composition (ENTITY §3.9.1)

The L1 / L2 cache key for any rendered output is:

```text
key = sha256(
        tenant_id_ulid
      ||  slug
      ||  capability_hash       // CapabilitySet::cache_key_hash(seed)
)
```

`capability_hash` ensures **two different principals visiting the
same `tenant + slug` get different cache slots when their
capability sets differ** — so an unauthenticated visitor never
receives a cached response that was constructed for an editor
with draft-visibility.

`CapabilitySet` wraps `BTreeSet<Capability>` so iteration order
is deterministic. The hash uses `DefaultHasher` for now;
cross-process stability (Dragonfly L2 fan-out with non-Rust
consumers) requires `xxh3_64` — slot is the M10 cache fan-out
ADR per `memory/orchestrator_init.md` §Inheritance map.

## §5 — Threat model

| ID    | Threat                                          | Mitigation                                                                              |
|-------|-------------------------------------------------|-----------------------------------------------------------------------------------------|
| T-2a  | Compromised role gains broader capability       | One-statement revocation (`UPDATE user_roles SET active=false`); audit trail in `user_role_events`. |
| T-2b  | Stolen JWT replayed after logout                | FM-006 in `memory/sentinel_init.md`: `jti` cache in moka (1 h TTL).                     |
| T-2c  | New handler ships without `caps.require()`      | `cargo xtask capability-coverage` exits 1 in CI on missing marker (W4 D2).              |
| T-2d  | Editor visibility leaked to anonymous user      | Cache key includes `capability_hash` (this doc §4).                                     |
| T-2e  | Plugin author elevates effective caps           | M9 W1 D1 RFC-009 must define a per-plugin `granted: Vec<Capability>` filter at install. |
| T-2f  | RLS bypass when handler forgets `set_config(...)`| Defense in depth — capability gate denies before the repo call (use case layer).        |

## §6 — Operability hooks

- `cargo xtask capability-coverage` — W4 D2 implementation gates
  every new handler at CI time.
- `metrics::counter!("auth.cap.denied", "cap" => cap.as_str())` —
  to be wired in the M2 W3 JWT day. Tracks per-capability denial
  rate; a spike on a previously-quiet capability is an
  Adversary-tier signal.
- `tracing::field::display(cap)` — every `caps.require()` call
  records the capability in the active span for log correlation.

## §7 — Open follow-ups

1. **JWT-derived caps extraction** — `extract_caps_for_today` is
   a stub (`crates/presentation/src/caps.rs`); real `jsonwebtoken`
   parse lands M2 W3 per `memory/orchestrator_init.md` open
   positions.
2. **`xxhash-rust` ADR** for cross-process cache-key stability —
   M10 W3 with Dragonfly L2 fan-out.
3. **MPD-001 capabilities** — nine commerce + CRM variants land
   M2 W6 D1.

## References

- `crates/domain/src/capability.rs` — implementation
- `crates/presentation/src/caps.rs` — `extract_caps_for_today` stub
- `xtask/src/commands/capability_coverage.rs` — CI gate (W4 D2)
- [SEC-001](SEC-001-sanitization-pipeline.md) — write-time sanitization
- [SEC-003](SEC-003-multi-tenant-isolation.md) — tenant isolation companion
- ENTITY §14 (capability model), §3.9.1 (cache-key composition)
