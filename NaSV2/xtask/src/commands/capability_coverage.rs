//! `xtask capability-coverage` · ENTITY §14, §18.3.
//!
//! Greps `crates/presentation/src/api/**/*.rs` for handler-shaped
//! functions and requires each to either:
//!   (a) carry a `caps.require(<cap-name>)` marker in a doc comment
//!       OR in the immediately-preceding line range (max 30 lines
//!       above the signature), OR
//!   (b) carry a `no_capability_required: <reason>` opt-out (non-empty
//!       reason mandatory — discourages drive-by exemption).
//!
//! Heuristic detection — NOT a full AST pass. Trade-offs:
//!   + Zero rustc dep; runs without nightly / proc-macro tooling.
//!   + Fast (single regex pass per file).
//!   - Doesn't follow nested-module const-fn definitions; we have
//!     none today.
//!   - Doesn't understand `#[handler]`-style attributed fns (axum
//!     has no such attribute); we infer via signature shape.
//!
//! "Handler-shaped" =
//!   `pub async fn <name>(...) -> ...` whose file path lives under
//!   `crates/presentation/src/api/**/*.rs`.
//!
//! ## TLA layers
//!
//! - **L1 Correctness** — regex is constrained; opt-out requires non-
//!   empty reason; failure exit code is 1 with file:line addressing.
//! - **L2 Performance** — one regex compile (LazyLock) shared across
//!   all files; walkdir filter skips non-rust entries before file I/O.
//! - **L3 Scalability** — bounded by handler file count (single-digit
//!   today; growth is linear and well under any CI budget).
//! - **L4 Operability** — violations printed with relative path and
//!   line number for easy editor jump; exits non-zero so CI can gate.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use eyre::{eyre, Result, WrapErr};
use regex::Regex;
use walkdir::WalkDir;

const PRESENTATION_API_PREFIX: &str = "crates/presentation/src/api";
const MARKER_LOOKBACK_LINES: usize = 30;

// ── Static regex initializers ───────────────────────────────────────────
//
// `expect_used` allowed: regex literals are compile-time constants;
// panic-on-init is the correct failure mode (mirrors check_planning_refs).

#[allow(clippy::expect_used)]
static HANDLER_SIG: LazyLock<Regex> = LazyLock::new(|| {
    // Generics-aware: optional `<T, U>` between fn name and `(`.
    Regex::new(r"^\s*pub\s+async\s+fn\s+(\w+)(?:<[^>]*>)?\s*\(")
        .expect("compile-time-valid handler signature regex")
});

#[allow(clippy::expect_used)]
static MARKER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"caps\.require\s*\(\s*([a-z0-9._]+)\s*\)").expect("compile-time-valid marker regex")
});

#[allow(clippy::expect_used)]
static OPT_OUT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"no_capability_required\s*[:=]\s*\S+").expect("compile-time-valid opt-out regex")
});

// ── Entry point ──────────────────────────────────────────────────────────

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let api_dir = root.join(PRESENTATION_API_PREFIX);
    if !api_dir.is_dir() {
        println!(
            "xtask capability-coverage: no handler directory at {} — \
             skipping (presentation crate has no api/ yet)",
            api_dir.display()
        );
        return Ok(());
    }

    let mut violations: Vec<String> = Vec::new();
    let mut handler_count = 0_usize;

    for entry in WalkDir::new(&api_dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }

        let body =
            std::fs::read_to_string(path).wrap_err_with(|| format!("read {}", path.display()))?;
        let lines: Vec<&str> = body.lines().collect();

        for (idx, line) in lines.iter().enumerate() {
            let Some(captures) = HANDLER_SIG.captures(line) else {
                continue;
            };
            handler_count += 1;
            let handler_name = captures.get(1).map_or("?", |m| m.as_str());

            let lo = idx.saturating_sub(MARKER_LOOKBACK_LINES);
            // Bounds are mathematically safe: `lo` is `saturating_sub`,
            // `idx` is bounded by `enumerate()` over `lines`.
            #[allow(clippy::indexing_slicing)]
            let window: String = lines[lo..=idx].join("\n");

            if OPT_OUT_RE.is_match(&window) || MARKER_RE.is_match(&window) {
                continue;
            }

            let display_path = path.strip_prefix(&root).unwrap_or(path);
            violations.push(format!(
                "{file}:{line}: handler `{handler_name}` lacks \
                 `caps.require(<cap>)` marker (or `no_capability_required: <reason>`)",
                file = display_path.display(),
                line = idx + 1,
            ));
        }
    }

    if violations.is_empty() {
        println!(
            "xtask capability-coverage: ok ({handler_count} handler(s) scanned in {PRESENTATION_API_PREFIX})"
        );
        Ok(())
    } else {
        for v in &violations {
            eprintln!("ERROR: {v}  [ENTITY §14]");
        }
        Err(eyre!(
            "xtask capability-coverage: {} handler(s) missing capability gate",
            violations.len()
        ))
    }
}

fn workspace_root() -> Result<PathBuf> {
    // `cargo run -p xtask` sets CARGO_MANIFEST_DIR to xtask/; the
    // workspace root is its parent.
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .wrap_err("CARGO_MANIFEST_DIR not set — invoke via `cargo run -p xtask`")?;
    Path::new(&manifest)
        .parent()
        .ok_or_else(|| eyre!("xtask manifest has no parent"))
        .map(Path::to_path_buf)
}

// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::same_item_push
)]
mod tests {
    use super::{HANDLER_SIG, MARKER_LOOKBACK_LINES, MARKER_RE, OPT_OUT_RE};

    #[test]
    fn handler_signature_matches_pub_async_fn() {
        assert!(HANDLER_SIG.is_match("pub async fn get_page_by_slug("));
        assert!(HANDLER_SIG.is_match("    pub async fn list_pages("));
        // Generics-aware (daily prompt §pitfall 2):
        assert!(HANDLER_SIG.is_match("pub async fn create<T>(payload: T,"));
        assert!(HANDLER_SIG.is_match("pub async fn handler<T, U>(a: T, b: U)"));
        // Negatives:
        assert!(!HANDLER_SIG.is_match("async fn private_helper("));
        assert!(!HANDLER_SIG.is_match("pub fn sync_handler("));
    }

    #[test]
    fn marker_regex_captures_cap_name() {
        let caps = MARKER_RE
            .captures("/// caps.require(cms.page.read)")
            .unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "cms.page.read");
        // Allows whitespace + multi-segment cap names.
        let caps = MARKER_RE
            .captures("// caps.require( commerce.product.publish )")
            .unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "commerce.product.publish");
    }

    #[test]
    fn opt_out_regex_matches_no_capability_required() {
        assert!(OPT_OUT_RE.is_match("// no_capability_required: health probe"));
        assert!(OPT_OUT_RE.is_match("// no_capability_required = csp_nonce_endpoint"));
        // Non-empty reason mandatory:
        assert!(!OPT_OUT_RE.is_match("// no_capability_required:"));
        assert!(!OPT_OUT_RE.is_match("// nope"));
    }

    #[test]
    fn marker_within_lookback_window_is_satisfied() {
        // Marker at top, then MARKER_LOOKBACK_LINES filler lines, then
        // handler — handler sits exactly at the lookback boundary.
        let mut lines: Vec<&str> = vec!["// caps.require(cms.page.read)"];
        lines.extend(std::iter::repeat_n(
            "// filler line",
            MARKER_LOOKBACK_LINES - 1,
        ));
        lines.push("pub async fn get(state: AppState) -> Result<()> {");
        let window = lines.join("\n");
        assert!(MARKER_RE.is_match(&window));
    }

    #[test]
    fn marker_outside_lookback_window_is_missed() {
        // Marker exactly 1 line beyond MARKER_LOOKBACK_LINES. The
        // `run()` function slices lines[idx-30..=idx] so a marker at
        // idx-31 is dropped from the window.
        let mut lines: Vec<&str> = vec!["// caps.require(cms.page.read)"];
        lines.extend(std::iter::repeat_n("// filler line", MARKER_LOOKBACK_LINES));
        lines.push("pub async fn handler() {}");

        let handler_idx = lines.len() - 1;
        let lo = handler_idx.saturating_sub(MARKER_LOOKBACK_LINES);
        let window = lines[lo..=handler_idx].join("\n");
        assert!(!MARKER_RE.is_match(&window));
    }
}
