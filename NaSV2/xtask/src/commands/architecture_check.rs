//! `xtask architecture-check` — enforce hex-layer dependency direction.
//!
//! ENTITY §2.6 / §18.1: the directed acyclic layer graph is
//! `common → domain → application → infrastructure → presentation`. Layers
//! deeper in the stack MUST NOT depend on shallower ones, and `domain` in
//! particular has a hard whitelist (purely-typed, runtime-free crate).
//!
//! ## Severity
//!
//! - **HARD** (exit 1): any forbidden dep in `nas2-domain`.
//! - **WARN** (stderr only, exit 0): `nas2-presentation` depending on
//!   `nas2-infrastructure`. The current workspace explicitly carries this
//!   coupling pending the Phase-B DI-factory refactor (see
//!   `crates/presentation/Cargo.toml` comment). The warning prevents the
//!   debt from growing without blocking unrelated PRs.
//!
//! ## TLA layers
//!
//! - **L1 Correctness** — uses `cargo metadata --no-deps` so the workspace
//!   manifest is the single source of truth (no `walkdir` heuristics).
//! - **L2 Performance** — one external process invocation, parsed once.
//! - **L3 Scalability** — runs in CI per-PR; the cost is bounded by the
//!   number of workspace members, not by source lines.
//! - **L4 Operability** — every violation prints the violating crate, the
//!   offending dep, and the ENTITY clause being enforced.

use std::process::Command;

use eyre::{eyre, Result, WrapErr};
use serde::Deserialize;

/// `nas2-domain` may depend ONLY on this set (plus internal workspace crates
/// it has been allow-listed for). Anything else is a HARD failure.
const DOMAIN_ALLOWED_DEPS: &[&str] = &[
    "serde",
    "uuid",
    "chrono",
    "garde",
    "thiserror",
    // internal — currently none; if `nas2-common` becomes a domain dep it
    // belongs here.
    "nas2-common",
];

/// Crates that are unconditionally forbidden inside `nas2-domain` (defense
/// in depth — the whitelist already covers this, but explicit denial means
/// future audits grep this list).
const DOMAIN_FORBIDDEN_DEPS: &[&str] = &[
    "tokio", "sqlx", "axum", "reqwest", "sentry", "tracing", "hyper",
];

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    id: String,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    name: String,
    kind: Option<String>, // None == "normal", Some("dev"|"build")
}

pub fn run() -> Result<()> {
    let meta = load_metadata()?;
    let in_workspace = |pkg: &Package| meta.workspace_members.contains(&pkg.id);

    let mut hard_violations: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    for pkg in meta.packages.iter().filter(|p| in_workspace(p)) {
        for d in pkg.dependencies.iter().filter(|d| is_normal(d)) {
            check_domain_rules(pkg, d, &mut hard_violations);
            check_presentation_rules(pkg, d, &mut warnings);
        }
    }

    for w in &warnings {
        eprintln!("WARN: {w}");
    }
    for v in &hard_violations {
        eprintln!("ERROR: {v}");
    }

    if hard_violations.is_empty() {
        println!(
            "architecture-check: ok ({} crate(s) inspected, {} warning(s))",
            meta.workspace_members.len(),
            warnings.len()
        );
        Ok(())
    } else {
        Err(eyre!(
            "architecture-check: {} hard violation(s) — see stderr",
            hard_violations.len()
        ))
    }
}

fn load_metadata() -> Result<Metadata> {
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .wrap_err("invoking `cargo metadata`")?;
    if !out.status.success() {
        return Err(eyre!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    // xtask is a build-tool (cold path); `serde_json` is fine here. ENTITY
    // §3.11 forbids serde_json only on request hot paths.
    #[allow(clippy::disallowed_methods)]
    let m: Metadata =
        serde_json::from_slice(&out.stdout).wrap_err("parsing cargo metadata JSON")?;
    Ok(m)
}

fn is_normal(d: &Dependency) -> bool {
    // cargo_metadata serializes "normal" deps with kind == None.
    d.kind.is_none()
}

fn check_domain_rules(pkg: &Package, dep: &Dependency, out: &mut Vec<String>) {
    if pkg.name != "nas2-domain" {
        return;
    }
    if DOMAIN_FORBIDDEN_DEPS.contains(&dep.name.as_str()) {
        out.push(format!(
            "ENTITY §2.6: `nas2-domain` MUST NOT depend on `{}` (forbidden runtime crate)",
            dep.name
        ));
        return;
    }
    let allowed = DOMAIN_ALLOWED_DEPS.contains(&dep.name.as_str());
    if !allowed {
        out.push(format!(
            "ENTITY §2.6: `nas2-domain` may only depend on the allow-list \
             {DOMAIN_ALLOWED_DEPS:?}; `{}` is not on it",
            dep.name
        ));
    }
}

fn check_presentation_rules(pkg: &Package, dep: &Dependency, out: &mut Vec<String>) {
    if pkg.name == "nas2-presentation" && dep.name == "nas2-infrastructure" {
        out.push(
            "ENTITY §2.6: `nas2-presentation` SHOULD NOT depend on \
             `nas2-infrastructure` directly (Phase-B DI refactor pending; \
             current waiver documented in crates/presentation/Cargo.toml)"
                .to_owned(),
        );
    }
}
