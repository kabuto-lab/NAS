//! HTML sanitization · ENTITY §3.6 (sanitize-on-write).
//!
//! **This module is a placeholder.** The real `clean_html` wiring
//! against `ammonia` lands W4 D4 (G5.P3 of `ROADMAP-2026-05`).
//!
//! Reserving the module name now means:
//!   1. Downstream code can write `use nas2_common::sanitize::*;`
//!      without churn when the real impl arrives.
//!   2. The named placeholder forces callers to acknowledge they are
//!      using a no-op (function is deliberately not called `clean_html`).
//!
//! DO NOT use `clean_html_placeholder` for untrusted input.

/// Identity function. Returns input unchanged.
///
/// Replace with `clean_html` (ammonia-backed, allow-listed tags) before
/// any user-content write path lands.
#[must_use]
pub fn clean_html_placeholder(input: &str) -> String {
    input.to_owned()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_is_identity() {
        // Intentional: until W4 D4, this is a no-op. Test asserts the
        // contract loudly so anyone wiring this into a write path
        // notices the input passes through unchanged.
        let input = "<script>alert(1)</script>";
        assert_eq!(clean_html_placeholder(input), input);
    }
}
