//! Content blocks — typed editor output · ENTITY §11.
//!
//! The block enum is the persistence and serialization shape of editor
//! content. Two design choices worth flagging:
//!
//!   1. **Tagged serde repr** (`#[serde(tag = "type")]`) — wire form is
//!      `{"type":"heading","level":"h2","text":"..."}`. Direct,
//!      schema-stable, grep-able in logs.
//!
//!   2. **`Paragraph { html: String }` stores PRE-SANITIZED HTML.**
//!      Sanitization is a write-path responsibility (§3.6); domain types
//!      assume the invariant has already been enforced. Storing raw
//!      HTML directly invites XSS the moment a downstream consumer
//!      prints it without re-escaping.
//!
//! Adding a new Block variant is intentionally low-friction — one match
//! arm + serde tag + one test. The barrier-to-entry for WHAT shapes
//! belong is editorial, not technical: every new variant needs an RFC
//! describing the editor UX it supports.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Heading {
        level: HeadingLevel,
        text: String,
    },
    Paragraph {
        html: String,
    },
    Image {
        // Short-term `String` shape. Becomes `MediaId` when the media
        // pipeline lands (M3 G2) — a content-version field will gate
        // the migration in a future ADR.
        src: String,
        alt: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        caption: Option<String>,
    },
    CodeBlock {
        #[serde(skip_serializing_if = "Option::is_none", default)]
        lang: Option<String>,
        code: String,
    },
}

impl Block {
    /// Stable static-string tag — useful for tracing fields and metric
    /// labels without serializing the full block.
    #[must_use]
    pub const fn block_type(&self) -> &'static str {
        match self {
            Self::Heading { .. } => "heading",
            Self::Paragraph { .. } => "paragraph",
            Self::Image { .. } => "image",
            Self::CodeBlock { .. } => "code_block",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadingLevel {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

impl HeadingLevel {
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::H1 => 1,
            Self::H2 => 2,
            Self::H3 => 3,
            Self::H4 => 4,
            Self::H5 => 5,
            Self::H6 => 6,
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_methods,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn heading_serde_roundtrip() {
        let b = Block::Heading {
            level: HeadingLevel::H2,
            text: "Hi".into(),
        };
        let json = serde_json::to_string(&b).unwrap();
        assert_eq!(json, r#"{"type":"heading","level":"h2","text":"Hi"}"#);
        let back: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(b, back);
    }

    #[test]
    fn paragraph_serde() {
        let b = Block::Paragraph {
            html: "<p>x</p>".into(),
        };
        let json = serde_json::to_string(&b).unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "paragraph");
        assert_eq!(v["html"], "<p>x</p>");
    }

    #[test]
    fn image_serde_with_caption() {
        let b = Block::Image {
            src: "/m/a.png".into(),
            alt: "alt".into(),
            caption: Some("c".into()),
        };
        let json = serde_json::to_string(&b).unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["caption"], "c");
        let back: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(b, back);
    }

    #[test]
    fn image_serde_without_caption_field_omitted() {
        let b = Block::Image {
            src: "/m/a.png".into(),
            alt: "alt".into(),
            caption: None,
        };
        let json = serde_json::to_string(&b).unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert!(v.get("caption").is_none(), "None caption must be omitted");
        let back: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(b, back);
    }

    #[test]
    fn code_block_serde_with_lang() {
        let b = Block::CodeBlock {
            lang: Some("rust".into()),
            code: "fn main(){}".into(),
        };
        let json = serde_json::to_string(&b).unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["lang"], "rust");
        assert_eq!(v["code"], "fn main(){}");
    }

    #[test]
    fn code_block_serde_no_lang_omits_field() {
        let b = Block::CodeBlock {
            lang: None,
            code: "x = 1".into(),
        };
        let json = serde_json::to_string(&b).unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert!(v.get("lang").is_none());
    }

    #[test]
    fn block_type_tags_match_serde_tag() {
        let cases: [Block; 4] = [
            Block::Heading {
                level: HeadingLevel::H1,
                text: String::new(),
            },
            Block::Paragraph {
                html: String::new(),
            },
            Block::Image {
                src: String::new(),
                alt: String::new(),
                caption: None,
            },
            Block::CodeBlock {
                lang: None,
                code: String::new(),
            },
        ];
        for b in &cases {
            let json = serde_json::to_string(b).unwrap();
            let v: Value = serde_json::from_str(&json).unwrap();
            assert_eq!(
                v["type"], b.block_type(),
                "block_type() must equal serde tag for {b:?}",
            );
        }
    }

    #[test]
    fn unknown_block_type_rejected() {
        let r: Result<Block, _> = serde_json::from_str(r#"{"type":"footnote"}"#);
        assert!(r.is_err());
    }

    #[test]
    fn heading_level_as_u8() {
        assert_eq!(HeadingLevel::H1.as_u8(), 1);
        assert_eq!(HeadingLevel::H2.as_u8(), 2);
        assert_eq!(HeadingLevel::H3.as_u8(), 3);
        assert_eq!(HeadingLevel::H4.as_u8(), 4);
        assert_eq!(HeadingLevel::H5.as_u8(), 5);
        assert_eq!(HeadingLevel::H6.as_u8(), 6);
    }

    #[test]
    fn heading_level_serde_lowercase() {
        let json = serde_json::to_string(&HeadingLevel::H3).unwrap();
        assert_eq!(json, r#""h3""#);
    }
}
