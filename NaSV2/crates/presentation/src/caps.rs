//! Per-request capability extraction.
//!
//! Today: stub returns `{CmsPageRead}` unconditionally. The marker
//! `caps.require(cms.page.read)` in handler doc-comments is the
//! contract that `xtask capability-coverage` (W4 D2) greps for.
//!
//! M2 W3: replace with JWT-derived extraction
//! (`fn extract_caps(req: &Request) -> Result<CapabilitySet, AppError>`).
//!
//! ## Test injection
//!
//! `with_caps_for_test(set) -> CapsGuard` installs a thread-local
//! override for subsequent `extract_caps_for_today` calls on the same
//! thread; the guard restores the previous value on drop (RAII).
//!
//! The thread-local check is always compiled (not gated `#[cfg(test)]`)
//! because integration-test binaries see the lib without that cfg.
//! Production overhead: one `RefCell::borrow()` returning `None` per
//! request — ~10 ns, negligible vs ENTITY §7's 10–20 ms p95 target.

use std::cell::RefCell;

use nas2_domain::{Capability, CapabilitySet};

thread_local! {
    static CAPS_OVERRIDE: RefCell<Option<CapabilitySet>> = const { RefCell::new(None) };
}

/// Default extractor — returns the stub set today; in M2 W3 this
/// becomes JWT-derived.
///
/// caps.require(cms.page.read)
///
/// (Marker above — `xtask capability-coverage` greps for it. Removing
/// without migrating to JWT-derived extraction breaks CI W4 D2 onward.)
#[must_use]
pub fn extract_caps_for_today() -> CapabilitySet {
    if let Some(caps) = CAPS_OVERRIDE.with(|c| c.borrow().clone()) {
        return caps;
    }
    CapabilitySet::from_iter([Capability::CmsPageRead])
}

/// Install a thread-local capability override for subsequent
/// `extract_caps_for_today` calls on this thread. Returns an RAII
/// guard that restores the previous value on drop.
///
/// **Test-only by convention.** Calling this in production is a code
/// smell — there is no operational reason to override caps.
///
/// `#[tokio::test]` defaults to `current_thread` runtime, so the
/// override is visible to the handler. If a future test uses
/// `flavor = "multi_thread"`, switch to `tokio::task_local!`.
#[must_use]
pub fn with_caps_for_test(caps: CapabilitySet) -> CapsGuard {
    let prev = CAPS_OVERRIDE.with(|c| c.replace(Some(caps)));
    CapsGuard { prev }
}

pub struct CapsGuard {
    prev: Option<CapabilitySet>,
}

impl Drop for CapsGuard {
    fn drop(&mut self) {
        CAPS_OVERRIDE.with(|c| *c.borrow_mut() = self.prev.take());
    }
}
