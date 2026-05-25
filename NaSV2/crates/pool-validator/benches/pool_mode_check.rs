//! Criterion bench — `PoolMode::from_str` parser hot-path.
//!
//! Purpose: provide the first nas2 microbench so that
//! `cargo bench --workspace --no-run` exercises the criterion compile path,
//! and so that future regressions in the parser are detected early.
//!
//! Online detection of pgbouncer is intentionally NOT benched here; that
//! requires a live container and would invalidate criterion's noise-floor
//! assumptions. A separate `detect_pool_mode_online` bench can be added
//! behind a `DATABASE_URL_DIRECT` env-var gate once the bench-runner
//! workflow is wired.
//!
//! ENTITY §6.3 — nightly benchmark pipeline · §8.1 benchmark-before-optimize.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nas2_pool_validator::PoolMode;

/// All four parser variants — covers the typed branches plus the
/// allocate-on-Unknown path so the bench captures both common arms.
const INPUTS: &[&str] = &["transaction", "session", "statement", "weird-future-mode"];

fn bench_pool_mode_from_str(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_mode_from_str");
    for input in INPUTS {
        group.bench_with_input(*input, input, |b, &raw| {
            b.iter(|| {
                let m = PoolMode::from_str(black_box(raw));
                black_box(m);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_pool_mode_from_str);
criterion_main!(benches);
