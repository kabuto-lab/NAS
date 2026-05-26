//! `xtask check-planning-refs` · ENTITY §13 planning-trail gate.
//!
//! Walks `git log <range>` and verifies each non-trivial commit message
//! contains at least one `(RFC|ADR|PLAN|VAL)-\w{1,16}` reference. Trivial
//! commits (typo / dep bump / fmt / comment-only) are exempt per
//! ENTITY §13.
//!
//! ## Severity
//!
//! Exit 1 on any violation. Designed to gate PRs in
//! `.github/workflows/nasv2-ci.yml` (job `xtask-gates`).
//!
//! ## TLA layers
//!
//! - L1 Correctness: regex tested against canonical examples in-module
//! - L2 Performance: single `git log` invocation; in-process scan
//! - L3 Scalability: bounded by commit count in range; CI typically
//!   passes `--since main~10` or compares against the merge-base
//! - L4 Operability: violations printed with short-SHA + first line + reason

use std::{process::Command, sync::LazyLock};

use eyre::{eyre, Result, WrapErr};
use regex::Regex;

// ── Static regex initializers ────────────────────────────────────────────
//
// `expect_used` allowed: regex literals are compile-time constants;
// panic-on-init is the correct failure mode (alternative would be
// runtime propagation of an error class that cannot occur).

#[allow(clippy::expect_used)]
static REF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:RFC|ADR|PLAN|VAL)-\w{1,16}\b").expect("compile-time-valid regex")
});

#[allow(clippy::expect_used)]
static TRIVIAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"^(?:",
        // typo fixes
        r"typo[:(]",
        r"|fix:? typo",
        // chore/style: fmt / formatting
        r"|chore\(fmt\)",
        r"|chore\([^)]*\):\s*fmt\b",
        r"|chore:\s*fmt\b",
        r"|style:?\s*format",
        // dep bumps  — `chore(deps): …` or `chore: deps …`
        r"|chore\(deps?\)",
        r"|chore\([^)]*\):\s*deps?\b",
        r"|chore:\s*deps?\b",
        r"|build:?\s*bump\b",
        // comment-only
        r"|docs?[:(]?\s*comment",
        r"|comment:",
        r")",
    ))
    .expect("compile-time-valid regex")
});

// ── Entry point ──────────────────────────────────────────────────────────

pub fn run(commit: &str) -> Result<()> {
    let messages = git_log_messages(commit)?;
    let total = messages.len();
    let mut violations: Vec<Violation> = Vec::new();

    for entry in &messages {
        let first_line = entry.body.lines().next().unwrap_or("").to_owned();
        if TRIVIAL_RE.is_match(&first_line) {
            println!("OK (trivial): {}  {}", short(&entry.sha), first_line);
            continue;
        }
        if REF_RE.is_match(&entry.body) {
            println!("OK         : {}  {}", short(&entry.sha), first_line);
            continue;
        }
        violations.push(Violation {
            sha: entry.sha.clone(),
            first_line,
            reason: "no RFC/ADR/PLAN/VAL ref and not trivial",
        });
    }

    if violations.is_empty() {
        println!("xtask check-planning-refs: ok ({total} commit(s) checked)");
        return Ok(());
    }
    for v in &violations {
        eprintln!(
            "ERROR: {sha}  {first}  ({reason})  [ENTITY §13]",
            sha = short(&v.sha),
            first = v.first_line,
            reason = v.reason
        );
    }
    Err(eyre!(
        "xtask check-planning-refs: {}/{} commit(s) failed",
        violations.len(),
        total
    ))
}

// ── git log harvest ──────────────────────────────────────────────────────

struct CommitEntry {
    sha: String,
    body: String,
}

fn git_log_messages(range_spec: &str) -> Result<Vec<CommitEntry>> {
    // \x00 between SHA and body, \x01 between commits — bytes that git
    // refuses inside commit messages, so collision is impossible.
    let out = Command::new("git")
        .args(["log", range_spec, "--format=%H%x00%B%x01"])
        .output()
        .wrap_err("invoking `git log` — is git in PATH?")?;
    if !out.status.success() {
        return Err(eyre!(
            "git log {range_spec} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let raw = String::from_utf8(out.stdout).wrap_err("git log output is not UTF-8")?;
    let mut entries = Vec::new();
    for chunk in raw.split('\x01').filter(|s| !s.trim().is_empty()) {
        let mut parts = chunk.splitn(2, '\x00');
        let sha = parts.next().unwrap_or("").trim().to_owned();
        let body = parts.next().unwrap_or("").to_owned();
        if !sha.is_empty() {
            entries.push(CommitEntry { sha, body });
        }
    }
    Ok(entries)
}

fn short(sha: &str) -> &str {
    let end = sha.len().min(7);
    &sha[..end]
}

// ── Violation type ───────────────────────────────────────────────────────

#[derive(Debug)]
struct Violation {
    sha: String,
    first_line: String,
    reason: &'static str,
}

// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn detects_rfc_ref() {
        assert!(REF_RE.is_match("feat: edge architecture · see RFC-002"));
    }

    #[test]
    fn detects_adr_ref() {
        assert!(REF_RE.is_match("feat(infra,ADR-003): three pools wired"));
    }

    #[test]
    fn detects_plan_ref() {
        // PLAN-G1 (alphanumeric tail) is the M1 convention.
        assert!(REF_RE.is_match("feat(ax/g1-d1,PLAN-G1): xtask magic-check"));
    }

    #[test]
    fn detects_val_ref() {
        assert!(REF_RE.is_match("test: VAL-005 RLS proptest"));
    }

    #[test]
    fn detects_multiple_refs_in_one_body() {
        let body = "feat: foo\n\nRFC-001 and PLAN-G2\n";
        assert!(REF_RE.is_match(body));
        // Count is implementation detail; only "any match" is contractual.
    }

    #[test]
    fn accepts_trivial_typo() {
        assert!(TRIVIAL_RE.is_match("typo: fix s/foo/bar"));
        assert!(TRIVIAL_RE.is_match("typo(docs): one letter"));
        assert!(TRIVIAL_RE.is_match("fix typo in README"));
    }

    #[test]
    fn accepts_trivial_dep_bump() {
        assert!(TRIVIAL_RE.is_match("chore(deps): bump tokio to 1.45"));
        assert!(TRIVIAL_RE.is_match("chore: deps update"));
        assert!(TRIVIAL_RE.is_match("build: bump rustc to 1.85"));
    }

    #[test]
    fn accepts_trivial_fmt() {
        assert!(TRIVIAL_RE.is_match("chore(xtask): fmt"));
        assert!(TRIVIAL_RE.is_match("chore: fmt"));
        assert!(TRIVIAL_RE.is_match("style: format imports"));
    }

    #[test]
    fn accepts_trivial_comment_only() {
        assert!(TRIVIAL_RE.is_match("docs: comment grammar"));
        assert!(TRIVIAL_RE.is_match("comment: clarify boundary"));
    }

    #[test]
    fn rejects_random_feat_message() {
        let first = "feat: add cool thing";
        let body = "feat: add cool thing\n\nMore details here.\n";
        assert!(!TRIVIAL_RE.is_match(first), "must NOT be trivial");
        assert!(!REF_RE.is_match(body), "must NOT contain planning ref");
    }

    #[test]
    fn short_sha_truncates() {
        assert_eq!(short("abcdef1234567890"), "abcdef1");
        assert_eq!(short("abc"), "abc");
        assert_eq!(short(""), "");
    }
}
