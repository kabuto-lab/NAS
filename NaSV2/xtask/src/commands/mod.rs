//! `xtask` command implementations · ENTITY §18.
//!
//! Each command lives in its own module so `main.rs` remains a thin
//! dispatch surface (spine — ENTITY §12).

pub mod architecture_check;
pub mod bench_runner;
pub mod capability_coverage;
pub mod check_planning_refs;
pub mod magic_check;
pub mod pool_mode_check;
