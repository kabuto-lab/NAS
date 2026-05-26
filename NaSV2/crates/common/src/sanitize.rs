//! HTML sanitization · ENTITY §3.6 — sanitize on WRITE only.
//!
//! Backed by [`ammonia`] with a curated allow-list. The output is
//! safe to store and to render directly into HTML contexts; the
//! read path MUST NOT re-sanitize.
//!
//! ## Allow-list
//!
//! - **Tags:** p, br, strong, em, u, h1..h6, ul, ol, li, blockquote,
//!   code, pre, a, img
//! - **Attrs:** `a[href]`, `img[src,alt,title]`, `code[class]`
//! - **Schemes:** `http`, `https`, `mailto`
//!
//! `code[class]` exists to preserve `code class="language-rust"` etc.
//! emitted by the M3+ block editor; the value is not whitelisted —
//! ammonia treats it as opaque text content.
//!
//! Additions need an [RFC](../../docs/rfc/) referencing the XSS class
//! being unblocked.
//!
//! ## TLA layers
//!
//! - **L1 Correctness** — ammonia's curated default + explicit
//!   tag/attr/scheme additions. Idempotent (sanitize-twice ≡
//!   sanitize-once for any input).
//! - **L2 Performance** — single `OnceLock<Builder>` per process;
//!   sanitize cost is dominated by ammonia internals (HTML5 parser).
//!   Cost lives on the write path — never on the cached read path
//!   (ENTITY §3.6 + §7).
//! - **L3 Scalability** — no shared state beyond the static builder;
//!   safe for unbounded concurrent calls.
//! - **L4 Operability** — failures cannot occur: ammonia produces a
//!   string for every input. Caller does not need a fallible signature.

use std::collections::HashSet;
use std::sync::OnceLock;

use ammonia::Builder;

/// Strip everything outside the AX•CMS allow-list. Idempotent —
/// calling twice yields the same result as once.
#[must_use]
pub fn clean_html(input: &str) -> String {
    builder().clean(input).to_string()
}

/// Placeholder retained for migration. The previous identity-function
/// shape was the W1 D5 reservation; this delegate keeps the public
/// surface stable while callers migrate to [`clean_html`].
///
/// Removal is scheduled for the end of M2 once all in-tree call sites
/// (currently none) migrate.
#[deprecated(
    note = "use `clean_html` instead; placeholder will be removed at end of M2"
)]
#[must_use]
pub fn clean_html_placeholder(input: &str) -> String {
    clean_html(input)
}

fn builder() -> &'static Builder<'static> {
    static B: OnceLock<Builder<'static>> = OnceLock::new();
    B.get_or_init(|| {
        let mut b = Builder::default();
        b.add_tag_attributes("a", ["href"]);
        b.add_tag_attributes("img", ["src", "alt", "title"]);
        b.add_tag_attributes("code", ["class"]);
        let schemes: HashSet<&'static str> =
            ["http", "https", "mailto"].into_iter().collect();
        b.url_schemes(schemes);
        b
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::clean_html;

    #[test]
    fn strips_script_tag() {
        // ammonia drops the disallowed element entirely (tag + body).
        let out = clean_html("<p>ok</p><script>alert(1)</script>");
        assert!(out.contains("<p>ok</p>"));
        assert!(!out.contains("<script"));
        assert!(!out.contains("alert(1)"));
    }

    #[test]
    fn keeps_allowed_tag_paragraph_strong() {
        let input = "<p><strong>hi</strong></p>";
        let out = clean_html(input);
        // ammonia is canonicalization-aware; exact-string equality is
        // brittle. Assert that the tags survived.
        assert!(out.contains("<p>"));
        assert!(out.contains("<strong>"));
        assert!(out.contains("hi"));
    }

    #[test]
    fn filters_javascript_url_scheme() {
        let out = clean_html(r#"<a href="javascript:alert(1)">x</a>"#);
        // Anchor element stays; the href attribute is dropped because
        // the scheme is not in the allow-list.
        assert!(!out.contains("javascript:"));
    }

    #[test]
    fn keeps_http_and_mailto_schemes() {
        assert!(clean_html(r#"<a href="https://x.com">x</a>"#).contains("https://x.com"));
        assert!(clean_html(r#"<a href="http://x.com">x</a>"#).contains("http://x.com"));
        assert!(clean_html(r#"<a href="mailto:a@b.c">x</a>"#).contains("mailto:a@b.c"));
    }

    #[test]
    fn idempotent() {
        let input = "<p>hello <strong>world</strong></p><script>bad</script>";
        let once = clean_html(input);
        let twice = clean_html(&once);
        assert_eq!(once, twice);
    }
}
