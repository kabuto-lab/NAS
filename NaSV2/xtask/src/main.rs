//! `cargo xtask` · AX•CMS build automation
//!
//! ENTITY §18 (workspace hygiene), §6.2 (PGO/BOLT), §6.3 (nightly bench).
//!
//! ## Command surface (spine — ENTITY §12)
//!
//! ### Hygiene gates (block merge)
//! - `architecture-check`     — verify hex-layer dependency direction
//! - `magic-check`            — ban macros >50 LOC, `lazy_static!` outside registry, raw `tokio::spawn`
//! - `capability-coverage`    — every HTTP handler calls `caps.require()` (ENTITY §14)
//! - `check-planning-refs`    — commit msg references RFC/ADR/PLAN/VAL (ENTITY §13)
//! - `extension-audit`        — extension hook usage is documented
//! - `pool-mode-check`        — pre-deploy `SHOW pool_mode` against staging PgBouncer
//!
//! ### Performance pipeline (ENTITY §6.2, §7)
//! - `pgo-build`              — profile-generate → bench → profile-use
//! - `bolt-optimize`          — apply BOLT post-link layout to the PGO binary
//! - `bench-runner`           — run criterion suite, write JSON report
//! - `alloc-budget`           — dhat heap snapshot vs `docs/perf/baseline.json`
//! - `query-budget`           — EXPLAIN ANALYZE deltas vs baseline

#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use eyre::Result;

mod commands;

#[derive(Parser)]
#[command(name = "xtask", version, about = "AX•CMS build automation")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    // ── hygiene gates ──────────────────────────────────────────────────────
    ArchitectureCheck,
    MagicCheck,
    CapabilityCoverage,
    CheckPlanningRefs {
        #[arg(long, default_value = "HEAD")]
        commit: String,
    },
    ExtensionAudit,
    PoolModeCheck {
        #[arg(long, env = "DATABASE_URL_HTTP")]
        database_url: String,
    },

    // ── performance pipeline (ENTITY §6.2) ─────────────────────────────────
    PgoBuild,
    BoltOptimize,
    BenchRunner {
        #[arg(long, default_value = "all")]
        suite: String,
    },
    AllocBudget,
    QueryBudget,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::ArchitectureCheck => stub("architecture-check"),
        Cmd::MagicCheck => stub("magic-check"),
        Cmd::CapabilityCoverage => stub("capability-coverage"),
        Cmd::CheckPlanningRefs { .. } => stub("check-planning-refs"),
        Cmd::ExtensionAudit => stub("extension-audit"),
        Cmd::PoolModeCheck { database_url } => commands::pool_mode_check::run(&database_url),
        Cmd::PgoBuild => stub("pgo-build"),
        Cmd::BoltOptimize => stub("bolt-optimize"),
        Cmd::BenchRunner { suite } => commands::bench_runner::run(&suite),
        Cmd::AllocBudget => stub("alloc-budget"),
        Cmd::QueryBudget => stub("query-budget"),
    }
}

fn stub(name: &str) -> Result<()> {
    println!("xtask {name}: stub — implementation pending (ENTITY §18, §6.2)");
    Ok(())
}
