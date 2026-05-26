// This file IS the magic-check rule definition. Test data inside the
// `tests` module contains literal strings like `"lazy_static! { ... }"`
// and `"tokio::spawn(async {});"` as deliberate fixtures. Opting out
// at file scope prevents magic-check from self-tripping on its own
// test corpus.
#![allow(clippy::disallowed_methods)]

//! `xtask magic-check` · forbid runtime footguns by grep · ENTITY §18.2
//!
//! Rules enforced (each via a `Rule` struct so the set is extensible):
//!   R1  raw `tokio::spawn(` outside `crates/runtime/src/supervisor.rs`
//!       and dev-test files that opt in via file-scope
//!       `#![allow(clippy::disallowed_methods)]`
//!   R2  `lazy_static!` outside `crates/common/src/registry/` (the
//!       registry path is reserved; rule armed for future drift)
//!   R3  `macro_rules!` definitions whose body exceeds 50 lines —
//!       per ENTITY §9.4, magic-heavy macros are forbidden
//!
//! This gate complements clippy's `disallowed-methods`: clippy catches
//! these at compile time, but a developer who runs `cargo check` only,
//! or who bypasses clippy locally, would not be flagged. `magic-check`
//! scans .rs files independently of cargo's target set.
//!
//! ## TLA layers
//!
//! - L1 Correctness: regex tested against canonical examples
//! - L2 Performance: walks once; no async; OK to run at every commit
//! - L3 Scalability: O(file_count × file_size); acceptable for ~50 files
//! - L4 Operability: violations printed with file:line + ENTITY reference

use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use eyre::{eyre, Result, WrapErr};
use regex::Regex;
use walkdir::WalkDir;

// ── Configuration ────────────────────────────────────────────────────────

/// Path fragments pruned from the walk. `walkdir` does not respect
/// `.gitignore`, so we hand-prune build artefacts and editor noise.
const SKIP_FRAGMENTS: &[&str] = &["target", ".sqlx", ".git", ".vscode", ".idea"];

/// Paths (relative to workspace root, forward-slash-normalized) where
/// raw `tokio::spawn` is permitted. ENTITY §4.8 sanctions the runtime
/// supervisor as the sole spawn surface.
const SPAWN_WHITELIST: &[&str] = &["crates/runtime/src/supervisor.rs"];

const R1_NAME: &str = "tokio_spawn";
const R2_NAME: &str = "lazy_static";
const R3_NAME: &str = "macro_rules_50_lines";

const MACRO_BODY_MAX_LINES: usize = 50;

// ── Static regex initializers ────────────────────────────────────────────
//
// `expect_used` is allowed because the regex literals are compile-time
// constants. Panic on init = correct failure mode (the alternative is
// runtime error propagation for a class of error that cannot occur).

#[allow(clippy::expect_used)]
static SPAWN_RE: LazyLock<Regex> = LazyLock::new(|| {
    // Word-boundary anchored; the `(` is required so that mentions like
    // `tokio::spawn_blocking(` are not falsely matched.
    Regex::new(r"\btokio::spawn\s*\(").expect("compile-time-valid regex")
});

#[allow(clippy::expect_used)]
static LAZY_STATIC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\blazy_static!\s*\{").expect("compile-time-valid regex"));

#[allow(clippy::expect_used)]
static MACRO_OPEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*macro_rules!\s+\w+\s*\{").expect("compile-time-valid regex"));

// ── Entry point ──────────────────────────────────────────────────────────

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let scan_dirs = ["crates", "apps", "xtask"];
    let mut violations: Vec<Violation> = Vec::new();

    let rules = [
        Rule::new(R1_NAME, &SPAWN_RE)
            .with_path_whitelist(SPAWN_WHITELIST)
            .with_file_scope_allow("clippy::disallowed_methods"),
        Rule::new(R2_NAME, &LAZY_STATIC_RE),
    ];

    let mut scanned: usize = 0;
    for dir in scan_dirs {
        let abs = root.join(dir);
        if !abs.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&abs).into_iter().filter_map(Result::ok) {
            if !is_rust_source(entry.path()) {
                continue;
            }
            if path_should_skip(entry.path()) {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(&root)
                .unwrap_or_else(|_| entry.path())
                .to_path_buf();
            let body = std::fs::read_to_string(entry.path())
                .wrap_err_with(|| format!("read {}", rel.display()))?;
            for rule in &rules {
                rule.scan(&rel, &body, &mut violations);
            }
            scan_macro_size(&rel, &body, &mut violations);
            scanned += 1;
        }
    }

    if violations.is_empty() {
        println!(
            "xtask magic-check: ok ({scanned} file(s) scanned across {} dir(s))",
            scan_dirs.len()
        );
        return Ok(());
    }
    for v in &violations {
        eprintln!("ERROR: {v}");
    }
    Err(eyre!(
        "xtask magic-check: {} violation(s) — see stderr",
        violations.len()
    ))
}

// ── Rule engine ──────────────────────────────────────────────────────────

struct Rule {
    name: &'static str,
    pattern: &'static LazyLock<Regex>,
    path_whitelist: &'static [&'static str],
    file_scope_allow: Option<&'static str>,
}

impl Rule {
    const fn new(name: &'static str, pattern: &'static LazyLock<Regex>) -> Self {
        Self {
            name,
            pattern,
            path_whitelist: &[],
            file_scope_allow: None,
        }
    }

    const fn with_path_whitelist(mut self, w: &'static [&'static str]) -> Self {
        self.path_whitelist = w;
        self
    }

    const fn with_file_scope_allow(mut self, lint: &'static str) -> Self {
        self.file_scope_allow = Some(lint);
        self
    }

    fn scan(&self, rel: &Path, body: &str, out: &mut Vec<Violation>) {
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if self.path_whitelist.iter().any(|w| rel_str == *w) {
            return;
        }
        if let Some(lint) = self.file_scope_allow {
            if file_opts_out(body, lint) {
                return;
            }
        }
        for (lineno, line) in body.lines().enumerate() {
            // Line-prefix comment check. Block comments `/* */` are
            // documented as a known limitation; macro_rules size rule
            // catches the loudest form of magic regardless.
            if is_doc_or_line_comment(line) {
                continue;
            }
            if self.pattern.is_match(line) {
                out.push(Violation {
                    rule: self.name,
                    file: rel.to_path_buf(),
                    line: lineno + 1,
                    excerpt: line.trim().to_owned(),
                });
            }
        }
    }
}

// ── File-scope-allow detection (multi-line aware) ────────────────────────

fn file_opts_out(body: &str, lint: &str) -> bool {
    // Match an outer attribute `#![allow(...)]` whose parenthesized list
    // contains the named lint, even when split across multiple lines.
    // `(?s)` flag makes `.` match newlines; `[^)]*` allows any list shape.
    let pattern = format!(r"(?s)#!\[allow\([^)]*{}[^)]*\)\]", regex::escape(lint));
    Regex::new(&pattern)
        .ok()
        .is_some_and(|re| re.is_match(body))
}

// ── R3: macro_rules! body length ─────────────────────────────────────────

fn scan_macro_size(rel: &Path, body: &str, out: &mut Vec<Violation>) {
    // Heuristic — count lines between `macro_rules! NAME {` and the
    // matching closing brace. Brace counting ignores braces inside
    // string literals; documented limitation.
    let mut depth: i32 = 0;
    let mut start_line: Option<usize> = None;
    for (lineno, line) in body.lines().enumerate() {
        if start_line.is_none() && MACRO_OPEN_RE.is_match(line) {
            start_line = Some(lineno);
            depth = 1;
            continue;
        }
        if let Some(start) = start_line {
            // i32 cast guarded — file with > 2^31 braces is unreal.
            let opens = i32::try_from(line.matches('{').count()).unwrap_or(0);
            let closes = i32::try_from(line.matches('}').count()).unwrap_or(0);
            depth += opens;
            depth -= closes;
            if depth <= 0 {
                let body_lines = lineno - start;
                if body_lines > MACRO_BODY_MAX_LINES {
                    out.push(Violation {
                        rule: R3_NAME,
                        file: rel.to_path_buf(),
                        line: start + 1,
                        excerpt: format!(
                            "macro_rules! body = {body_lines} lines (limit {MACRO_BODY_MAX_LINES})"
                        ),
                    });
                }
                start_line = None;
                depth = 0;
            }
        }
    }
}

// ── Utility ──────────────────────────────────────────────────────────────

fn is_doc_or_line_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//")
}

fn is_rust_source(p: &Path) -> bool {
    p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("rs")
}

fn path_should_skip(p: &Path) -> bool {
    let s = p.to_string_lossy();
    SKIP_FRAGMENTS.iter().any(|f| s.contains(f))
}

fn workspace_root() -> Result<PathBuf> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .wrap_err("CARGO_MANIFEST_DIR not set — run via `cargo xtask`")?;
    let root = Path::new(&manifest)
        .parent()
        .ok_or_else(|| eyre!("xtask manifest has no parent directory"))?
        .to_path_buf();
    Ok(root)
}

// ── Violation type ───────────────────────────────────────────────────────

#[derive(Debug)]
struct Violation {
    rule: &'static str,
    file: PathBuf,
    line: usize,
    excerpt: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{rule} at {file}:{line}: {excerpt}  [ENTITY §18.2]",
            rule = self.rule,
            file = self.file.display(),
            line = self.line,
            excerpt = self.excerpt
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::format_push_string
)]
mod tests {
    use super::*;

    fn rule_spawn() -> Rule {
        Rule::new(R1_NAME, &SPAWN_RE)
            .with_path_whitelist(SPAWN_WHITELIST)
            .with_file_scope_allow("clippy::disallowed_methods")
    }

    fn rule_lazy() -> Rule {
        Rule::new(R2_NAME, &LAZY_STATIC_RE)
    }

    #[test]
    fn detects_bare_tokio_spawn() {
        let body = "fn foo() {\n    tokio::spawn(async {});\n}\n";
        let mut out = Vec::new();
        rule_spawn().scan(Path::new("crates/foo/src/lib.rs"), body, &mut out);
        assert_eq!(out.len(), 1, "expected one violation, got {out:?}");
        assert_eq!(out[0].line, 2);
    }

    #[test]
    fn skips_runtime_supervisor_path() {
        let body = "tokio::spawn(async {});\n";
        let mut out = Vec::new();
        rule_spawn().scan(
            Path::new("crates/runtime/src/supervisor.rs"),
            body,
            &mut out,
        );
        assert!(out.is_empty(), "supervisor must be whitelisted");
    }

    #[test]
    fn skips_test_files_with_file_scope_allow() {
        let body = "#![allow(clippy::disallowed_methods)]\ntokio::spawn(async {});\n";
        let mut out = Vec::new();
        rule_spawn().scan(
            Path::new("crates/pool-validator/tests/foo.rs"),
            body,
            &mut out,
        );
        assert!(out.is_empty(), "file-scope allow must opt the file out");
    }

    #[test]
    fn doc_comment_does_not_trip_rule() {
        let body = "/// example: tokio::spawn(future)\n";
        let mut out = Vec::new();
        rule_spawn().scan(Path::new("crates/foo/src/lib.rs"), body, &mut out);
        assert!(out.is_empty(), "/// comment must be ignored");
    }

    #[test]
    fn lazy_static_macro_flagged_outside_registry() {
        // Constructed at runtime so this file's source does NOT contain
        // the literal `lazy_static!` token — otherwise `magic-check`
        // would self-trip when scanning its own test corpus.
        let needle = [stringify!(lazy_static), "!"].concat();
        let body = format!("{needle} {{ static ref X: u8 = 1; }}\n");
        let mut out = Vec::new();
        rule_lazy().scan(Path::new("crates/foo/src/lib.rs"), &body, &mut out);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn macro_rules_oversize_flagged() {
        // Body of 60 lines between `{` and final `}`.
        let mut body = String::from("macro_rules! big {\n");
        for i in 0..60 {
            body.push_str(&format!("    // line {i}\n"));
        }
        body.push_str("}\n");
        let mut out = Vec::new();
        scan_macro_size(Path::new("crates/foo/src/lib.rs"), &body, &mut out);
        assert_eq!(out.len(), 1, "60-line macro body must trip R3");
        assert_eq!(out[0].rule, R3_NAME);
    }

    #[test]
    fn macro_rules_under_limit_not_flagged() {
        let mut body = String::from("macro_rules! small {\n");
        for i in 0..10 {
            body.push_str(&format!("    // line {i}\n"));
        }
        body.push_str("}\n");
        let mut out = Vec::new();
        scan_macro_size(Path::new("crates/foo/src/lib.rs"), &body, &mut out);
        assert!(out.is_empty(), "10-line macro body must pass");
    }
}
