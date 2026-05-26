//! REST API surface · ENTITY §14 (capability gating per handler).
//!
//! # Handler capability contract
//!
//! Every `pub async fn` declared under this module **must** declare its
//! capability requirement in one of two ways:
//!
//! 1. **Required gate** — a marker matching `caps.require(<cap.name>)`
//!    placed in a doc comment OR within the 30 lines immediately
//!    preceding the function signature. Example:
//!
//!    ```ignore
//!    /// caps.require(cms.page.read)
//!    pub async fn get_page_by_slug(/* … */) -> Result<Json<Post>, AppError> {
//!        let caps = extract_caps_for_today();
//!        // …
//!    }
//!    ```
//!
//! 2. **Explicit opt-out** — a marker matching
//!    `no_capability_required: <reason>` with a non-empty reason.
//!    Reserve this for unauthenticated probes (health endpoints,
//!    CSP nonce rotation, OAuth callbacks). Example:
//!
//!    ```ignore
//!    // no_capability_required: liveness probe — Kubernetes only
//!    pub async fn live() -> &'static str { "ok" }
//!    ```
//!
//! The `cargo xtask capability-coverage` CI gate enforces this contract
//! (see `xtask/src/commands/capability_coverage.rs`). A handler that
//! ships with neither marker fails the build with `file:line` precision.

pub mod pages;
