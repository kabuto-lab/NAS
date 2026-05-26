//! Content blocks — typed enum for editor output · ENTITY §11.
//!
//! **Placeholder until W2 D4 (G2.P6).** A single variant exists so that
//! `Post::blocks: Vec<Block>` compiles today. Real variants
//! (Heading / Paragraph / Image / CodeBlock) replace it on 2026-06-04.

use serde::{Deserialize, Serialize};

/// Placeholder discriminant. Will be removed in W2 D4 and replaced with
/// the real Block variants. Do NOT take dependencies on this variant
/// beyond `Post::blocks` scaffolding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Placeholder,
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_methods
)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_roundtrip() {
        let b = Block::Placeholder;
        let json = serde_json::to_string(&b).unwrap();
        assert_eq!(json, r#"{"type":"placeholder"}"#);
        let back: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(b, back);
    }
}
