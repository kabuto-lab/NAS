//! `cargo xtask` — custom build automation per `ENTITY.md §2.6`.
//!
//! Subcommands:
//! - `architecture-check`: verify cross-crate dependency boundaries (L1-L4)
//! - `magic-check`: ban `macro_rules!` outside common/macros, raw `tokio::spawn`, etc.
//! - `check-planning-refs`: verify commits / PRs reference RFC/ADR/PLAN/VAL
//! - `alloc-budget` (stub): compare dhat snapshots vs baselines
//! - `query-budget` (stub): EXPLAIN ANALYZE regression check

use clap::{Parser, Subcommand};
use eyre::Result;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "xtask", version, about = "AX build automation")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Verify 4-layer architecture boundaries per ENTITY.md §2.
    ArchitectureCheck,
    /// Verify NO MAGIC policy per ENTITY.md §2.7.
    MagicCheck,
    /// Verify commit / PR has RFC/ADR/PLAN/VAL refs per §2.5.
    CheckPlanningRefs {
        #[arg(long, default_value = "HEAD")]
        commit: String,
    },
    /// Compare dhat snapshot vs baseline per §11.5 (stub).
    AllocBudget,
    /// EXPLAIN ANALYZE regression vs baseline per §11.6 (stub).
    QueryBudget,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::ArchitectureCheck => architecture_check(),
        Cmd::MagicCheck => magic_check(),
        Cmd::CheckPlanningRefs { commit } => check_planning_refs(&commit),
        Cmd::AllocBudget => alloc_budget(),
        Cmd::QueryBudget => query_budget(),
    }
}

fn workspace_root() -> Result<PathBuf> {
    Ok(std::env::current_dir()?)
}

// ───────────────────────────────────────────────────────────────
// architecture-check — verify crate-to-crate dependency boundaries
// ───────────────────────────────────────────────────────────────

fn architecture_check() -> Result<()> {
    let root = workspace_root()?;
    let crates_dir = root.join("crates");
    let mut violations: Vec<String> = Vec::new();

    // Rules per ENTITY.md §2 + ADR-001 D1
    let rules: &[(&str, &[&str], &[&str])] = &[
        // (crate_name, forbidden_deps_substrings, required_deps_substrings)
        (
            "domain",
            &["sqlx", "axum", "tokio", "tower", "reqwest", "sentry", "tracing-opentelemetry"],
            &[],
        ),
        (
            "application",
            &["sqlx", "axum", "tower", "tower-http"],
            &[],
        ),
        // presentation cannot direct-depend on sqlx (only via application)
        (
            "presentation",
            &["sqlx"],
            &[],
        ),
    ];

    for (crate_name, forbidden, required) in rules {
        let manifest = crates_dir.join(crate_name).join("Cargo.toml");
        if !manifest.exists() {
            violations.push(format!("missing Cargo.toml: {}", manifest.display()));
            continue;
        }
        let content = std::fs::read_to_string(&manifest)?;

        for f in *forbidden {
            // Allow if commented out
            for line in content.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with('#') {
                    continue;
                }
                if trimmed.contains(f)
                    && !trimmed.contains("# allow:")
                    && !trimmed.starts_with("//")
                {
                    // Heuristic: dependency entry (e.g., `sqlx = ...`)
                    if trimmed.starts_with(f)
                        || trimmed.starts_with(&format!("\"{f}\""))
                        || trimmed.contains(&format!("name = \"{f}\""))
                    {
                        violations.push(format!(
                            "crate `{crate_name}` has forbidden dependency `{f}`: {}",
                            trimmed
                        ));
                    }
                }
            }
        }
        let _ = required; // placeholder for required-deps checks
    }

    if violations.is_empty() {
        println!("architecture-check: OK ({} crates checked)", rules.len());
        Ok(())
    } else {
        for v in &violations {
            eprintln!("VIOLATION: {v}");
        }
        eyre::bail!("architecture-check failed: {} violations", violations.len())
    }
}

// ───────────────────────────────────────────────────────────────
// magic-check — verify NO MAGIC policy
// ───────────────────────────────────────────────────────────────

fn magic_check() -> Result<()> {
    let root = workspace_root()?;
    let mut violations: Vec<String> = Vec::new();

    // Forbidden patterns per ENTITY.md §2.7
    let macro_rules_re = regex::Regex::new(r"^\s*macro_rules!").unwrap();
    let tokio_spawn_re = regex::Regex::new(r"\btokio::spawn\b").unwrap();
    let std_thread_spawn_re = regex::Regex::new(r"\bstd::thread::spawn\b").unwrap();
    let arc_rwlock_re = regex::Regex::new(r"Arc<RwLock<").unwrap();

    let walker = walkdir::WalkDir::new(&root).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        !matches!(
            name.as_ref(),
            "target" | ".git" | "node_modules" | ".sqlx" | "istori"
        )
    });

    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let rel = path.strip_prefix(&root).unwrap_or(path);
        let rel_str = rel.to_string_lossy();

        // Allow runtime crate for tokio::spawn (only allowed location)
        let in_runtime_supervisor = rel_str.contains("crates/runtime");
        // Allow common/macros for macro_rules!
        let in_common_macros = rel_str.contains("crates/common/macros");

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (lineno, line) in content.lines().enumerate() {
            // Skip comments
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }

            if macro_rules_re.is_match(line) && !in_common_macros {
                violations.push(format!(
                    "{}:{}: macro_rules! outside crates/common/macros (ENTITY §2.7)",
                    rel_str,
                    lineno + 1
                ));
            }
            if tokio_spawn_re.is_match(line) && !in_runtime_supervisor {
                violations.push(format!(
                    "{}:{}: tokio::spawn outside runtime/supervisor (ENTITY §4.8)",
                    rel_str,
                    lineno + 1
                ));
            }
            if std_thread_spawn_re.is_match(line) {
                violations.push(format!(
                    "{}:{}: std::thread::spawn — use tokio::task::spawn_blocking instead",
                    rel_str,
                    lineno + 1
                ));
            }
            if arc_rwlock_re.is_match(line) {
                // Only warn — require comment with justification on adjacent line
                eprintln!(
                    "WARN: {}:{}: Arc<RwLock<...>> — consider justification comment",
                    rel_str,
                    lineno + 1
                );
            }
        }
    }

    if violations.is_empty() {
        println!("magic-check: OK");
        Ok(())
    } else {
        for v in &violations {
            eprintln!("VIOLATION: {v}");
        }
        eyre::bail!("magic-check failed: {} violations", violations.len())
    }
}

// ───────────────────────────────────────────────────────────────
// check-planning-refs — verify RFC/ADR/PLAN/VAL refs in commit message
// ───────────────────────────────────────────────────────────────

fn check_planning_refs(commit: &str) -> Result<()> {
    // Stub: in CI, this would `git log <commit> --format=%B | grep -E '(RFC|ADR|PLAN|VAL)-[0-9]+'`
    // For now, just succeed when called from xtask.
    println!(
        "check-planning-refs: stub (commit={commit}); CI integration TODO per §2.5"
    );
    Ok(())
}

fn alloc_budget() -> Result<()> {
    println!("alloc-budget: stub — TODO per ENTITY.md §11.5 (Phase B integration with dhat-rs)");
    Ok(())
}

fn query_budget() -> Result<()> {
    println!("query-budget: stub — TODO per ENTITY.md §11.6 (EXPLAIN ANALYZE regression)");
    Ok(())
}
