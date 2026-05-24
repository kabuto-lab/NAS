//! `xtask bench-runner` · ENTITY §6.3 nightly benchmark gate.
//!
//! Runs `cargo bench --workspace --message-format=json`, harvests each
//! benchmark's mean estimate from `target/criterion/**/new/estimates.json`,
//! and compares the numbers against `docs/perf/baseline.json`.
//!
//! Behaviour:
//! - If `docs/perf/baseline.json` is missing: write the current numbers as
//!   the new baseline, exit 0.  First-run bootstrap.
//! - If present and any benchmark regressed by more than
//!   `REGRESSION_THRESHOLD_PCT` (default 5%): exit 1.
//! - Otherwise: exit 0 with a per-bench diff report on stdout.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

const BASELINE_RELATIVE: &str = "docs/perf/baseline.json";
const REGRESSION_THRESHOLD_PCT: f64 = 5.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    /// Map: `<bench-id>` → mean estimate, nanoseconds.
    pub benches: BTreeMap<String, f64>,
}

pub fn run(suite: &str) -> Result<()> {
    let workspace_root = workspace_root()?;
    let baseline_path = workspace_root.join(BASELINE_RELATIVE);

    run_cargo_bench(suite)?;
    let current = harvest_criterion_results(&workspace_root)?;
    if current.benches.is_empty() {
        eprintln!(
            "xtask bench-runner: no criterion results found under \
             target/criterion — nothing to compare"
        );
        return Ok(());
    }

    if !baseline_path.exists() {
        write_baseline(&baseline_path, &current)?;
        println!(
            "xtask bench-runner: bootstrap — wrote {} baseline samples to {}",
            current.benches.len(),
            baseline_path.display()
        );
        return Ok(());
    }

    let baseline = read_baseline(&baseline_path)?;
    let regressions = diff_against_baseline(&baseline, &current);
    if regressions.is_empty() {
        println!(
            "xtask bench-runner: OK — {} benches within {:.1}% of baseline",
            current.benches.len(),
            REGRESSION_THRESHOLD_PCT,
        );
        Ok(())
    } else {
        for r in &regressions {
            eprintln!(
                "  REGRESSION  {name}: baseline={baseline:.1} ns  current={current:.1} ns  delta=+{delta:.1}%",
                name = r.name,
                baseline = r.baseline_ns,
                current = r.current_ns,
                delta = r.delta_pct,
            );
        }
        Err(eyre::eyre!(
            "xtask bench-runner: {} bench(es) regressed > {:.1}%",
            regressions.len(),
            REGRESSION_THRESHOLD_PCT
        ))
    }
}

fn run_cargo_bench(suite: &str) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("bench").arg("--workspace");
    if suite != "all" {
        cmd.arg("--").arg(suite);
    }
    let status = cmd.status().wrap_err("spawn cargo bench")?;
    if !status.success() {
        return Err(eyre::eyre!("cargo bench exited with {status}"));
    }
    Ok(())
}

fn workspace_root() -> Result<PathBuf> {
    // `xtask` is always invoked from the workspace root, so CARGO_MANIFEST_DIR
    // for the xtask crate is `<root>/xtask`.
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .wrap_err("CARGO_MANIFEST_DIR not set — run via `cargo xtask`")?;
    let root = Path::new(&manifest)
        .parent()
        .ok_or_else(|| eyre::eyre!("xtask manifest has no parent"))?
        .to_path_buf();
    Ok(root)
}

fn harvest_criterion_results(workspace_root: &Path) -> Result<Baseline> {
    let criterion_dir = workspace_root.join("target").join("criterion");
    let mut benches = BTreeMap::new();
    if !criterion_dir.exists() {
        return Ok(Baseline { benches });
    }
    for entry in walkdir::WalkDir::new(&criterion_dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let path = entry.path();
        if !path.is_file() || path.file_name() != Some(std::ffi::OsStr::new("estimates.json")) {
            continue;
        }
        // Only the `new/` snapshot reflects the current run.
        if path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            != Some("new")
        {
            continue;
        }
        let id_path = path
            .parent()
            .and_then(|p| p.parent())
            .ok_or_else(|| eyre::eyre!("estimates.json has no grandparent"))?;
        let bench_id = id_path.strip_prefix(&criterion_dir).map_or_else(
            |_| id_path.display().to_string(),
            |p| p.display().to_string().replace('\\', "/"),
        );

        let raw = fs::read_to_string(path)
            .wrap_err_with(|| format!("read {}", path.display()))?;
        // xtask is a cold-path CLI tool — the disallowed-methods lint
        // specifically permits serde_json in this category (clippy.toml).
        #[allow(clippy::disallowed_methods)]
        let parsed: CriterionEstimates = serde_json::from_str(&raw)
            .wrap_err_with(|| format!("parse {}", path.display()))?;
        benches.insert(bench_id, parsed.mean.point_estimate);
    }
    Ok(Baseline { benches })
}

fn write_baseline(path: &Path, baseline: &Baseline) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(baseline).wrap_err("serialize baseline")?;
    fs::write(path, json).wrap_err_with(|| format!("write {}", path.display()))?;
    Ok(())
}

fn read_baseline(path: &Path) -> Result<Baseline> {
    let raw = fs::read_to_string(path).wrap_err_with(|| format!("read {}", path.display()))?;
    // xtask cold-path: serde_json is permitted by clippy.toml note.
    #[allow(clippy::disallowed_methods)]
    serde_json::from_str(&raw).wrap_err_with(|| format!("parse {}", path.display()))
}

#[derive(Debug)]
struct Regression {
    name: String,
    baseline_ns: f64,
    current_ns: f64,
    delta_pct: f64,
}

fn diff_against_baseline(baseline: &Baseline, current: &Baseline) -> Vec<Regression> {
    let mut out = Vec::new();
    for (name, &current_ns) in &current.benches {
        let Some(&baseline_ns) = baseline.benches.get(name) else {
            continue; // new bench — not a regression
        };
        if baseline_ns <= 0.0 {
            continue;
        }
        let delta_pct = (current_ns - baseline_ns) / baseline_ns * 100.0;
        if delta_pct > REGRESSION_THRESHOLD_PCT {
            out.push(Regression {
                name: name.clone(),
                baseline_ns,
                current_ns,
                delta_pct,
            });
        }
    }
    out
}

#[derive(Debug, Deserialize)]
struct CriterionEstimates {
    mean: Estimate,
}

#[derive(Debug, Deserialize)]
struct Estimate {
    point_estimate: f64,
}
