# M3 · W1 · 2026-07-20 → 2026-07-24 · ADR-010 editor strategy + Media model

> **Master plan rows (`WP-PLAN-12-MONTH.html` M3 W1 cells):**
> All five cells fall under the W1 banner `ADR-010 (editor strategy) + Media model`.
>
> **Carry-over from W0 (RETRO-2026-07 · 2026-07-17):**
> M2 exit clean — `PgPostRepository`, RLS proptest 1000 rounds 0 leaks, JWT + login, 5 CRUD + CSRF, ADR-009 revisions concept ratified. Repo at HEAD `<RETRO-sha>`. WP-parity ~14%.

---

## Week shape

| Date | DoW | Goal | One-line scope | Architect | Sr.Dev |
|---|---|---|---|---|---|
| 2026-07-20 | **Mon** | G1·P1 | ADR-010 block-editor strategy — *option study* (Gutenberg-wasm / Leptos / JSON-textarea) | [`mon/architect.md`](2026-07-20-mon/architect.md) | [`mon/senior-dev.md`](2026-07-20-mon/senior-dev.md) |
| 2026-07-21 | Tue | G1·P2 | ADR-010 finalization + RFC-007 media model + ADR-011 storage adapter | _planned_ | _planned_ |
| 2026-07-22 | Wed | G2·P1 | `Media` aggregate (`Mime`, `checksum`, `variants: Vec`, `alt`, `caption`) — `crates/domain` | _planned_ | _planned_ |
| 2026-07-23 | Thu | G2·P2 | `PgMediaRepository` (`find` / `list_for_tenant` / `insert`) — `crates/infrastructure` | _planned_ | _planned_ |
| 2026-07-24 | Fri | G2·P3 | Migrations `0009_media.sql` + `0010_media_variants.sql` + `0011_rls_media.sql` | _planned_ | _planned_ |

---

## Inter-day data flow

```
Mon ─┐
     │ ADR-010 option study (3 candidates compared by matrix)
     ▼
Tue ─┐
     │ ADR-010 ratified (decision committed) + RFC-007 (media data model)
     │ + ADR-011 (storage adapter contract)
     ▼
Wed ─┐
     │ Media aggregate code lands in crates/domain
     │ (mirrors RFC-007 shape)
     ▼
Thu ─┐
     │ PgMediaRepository in crates/infrastructure
     │ (consumes Media aggregate + ADR-011 StorageAdapter trait)
     ▼
Fri ─┐
     │ Three migrations land
     │ (schema enforces the Wed aggregate shape; RLS isolates by tenant)
     │ → W2 entering state: domain + repo + schema all aligned
     ▼
```

---

## Spine-touch ledger this week

| Day | File | Spine? | Reason |
|---|---|---|---|
| Mon | `docs/adr/ADR-010-block-editor-strategy.md` (NEW) | non-spine | new ADR |
| Tue | same file (finalize) + `docs/rfc/RFC-007-media-model.md` (NEW) + `docs/adr/ADR-011-media-storage-adapter.md` (NEW) | non-spine | new docs |
| Wed | `crates/domain/src/lib.rs` + `crates/domain/src/media.rs` (NEW) | non-spine | feature code |
| Thu | `crates/infrastructure/src/lib.rs` + `crates/infrastructure/src/media/` (NEW) + `crates/infrastructure/Cargo.toml` *(add `sha2`, `mime` deps if needed)* | non-spine `Cargo.toml`? **conditional spine** — verify in Wed prep; if dep additions are required, that bumps to a Thu spine mini-edit. | feature code |
| Fri | `migrations/0009_media.sql` (NEW) + `migrations/0010_media_variants.sql` (NEW) + `migrations/0011_rls_media.sql` (NEW) + `docs/rollback/ROLLBACK-0009.md` / `-0010.md` / `-0011.md` (NEW) | non-spine (additive migrations per §12) | schema landing |

ENTITY.md / CLAUDE.md / workspace `Cargo.toml` / `xtask/src/main.rs` / `apps/server/src/main.rs` / clippy.toml / rustfmt.toml / deny.toml / docker-compose.dev.yml / .env.example — **none touched** this week.

---

## Verification rhythm

Each day runs `V1..V4` per `HOW-TO-RUN.md §10`:

```
V1  cargo check --workspace --all-targets
V2  cargo fmt --all
V3  cargo clippy --workspace --all-targets -- -D warnings
V4  cargo test --workspace --lib --no-fail-fast
```

W1 D1 (Mon) is docs-only, so V1..V4 should pass identically to W0 close.
W1 D5 (Fri) additionally runs `cargo run --bin nas2-cli -- db migrate` against testcontainers PG and asserts RLS by proptest (lighter than VAL-005 — full 1000-round proptest is scheduled in M3 W4 follow-up).

---

## Carry-over to W2 (2026-07-27 entering state)

By W1 D5 EOD:

- `crates/domain::Media` exists with derived shapes.
- `crates/infrastructure::PgMediaRepository` compiles and has 2 happy-path integration tests.
- Schema 0009/0010/0011 applied; RLS verified on a cross-tenant probe.
- `ADR-010` ratified; W2 D1 (libvips worker scaffold) consumes the variant-matrix decision (thumb 320 / medium 768 / large 1280 / original).
- `ADR-011` ratified; W3 D3 (`StorageAdapter` trait) implements its contract literally.
- `RFC-007` ratified; future W3 D4 `infer`-based MIME sniffing extends the trust-on-write contract.

If any item above is red, W2 D1 begins with a CARRY-OVER repair pass.
