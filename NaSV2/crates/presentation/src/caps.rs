//! Per-request capability extraction.
//!
//! Today: stub returns `{CmsPageRead}` unconditionally. The marker
//! `caps.require(cms.page.read)` in handler doc-comments is the
//! contract that `xtask capability-coverage` (W4 D2) greps for.
//!
//! M2 W3: replace with JWT-derived extraction
//! (`fn extract_caps(req: &Request) -> Result<CapabilitySet, AppError>`).

use nas2_domain::{Capability, CapabilitySet};

/// Stub capability extractor.
///
/// caps.require(cms.page.read)
///
/// (Marker line above — `xtask capability-coverage` greps for
/// `caps.require(<cap>)` near handlers. Removing this without
/// migrating to JWT-derived extraction breaks CI on W4 D2 onward.)
#[must_use]
pub fn extract_caps_for_today() -> CapabilitySet {
    CapabilitySet::from_iter([Capability::CmsPageRead])
}

/// Test-only override. Lets integration tests inject specific
/// capability sets to hit the 403 path before JWT lands.
#[cfg(test)]
#[must_use]
pub fn extract_caps_for_test(caps: CapabilitySet) -> CapabilitySet {
    caps
}
