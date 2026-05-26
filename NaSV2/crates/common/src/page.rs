//! Pagination wrapper · ENTITY §10 (port consistency).
//!
//! Offset/limit only. Cursor pagination is a separate concern — see
//! Ideas-for-next-month in `docs/session-plans/AUDIT-2026-05-25.md`.

use serde::{Deserialize, Serialize};

/// A page of items together with pagination metadata.
///
/// Generic over the item type. Designed for HTTP JSON bodies — the
/// `#[serde]` derives produce `{"items": [...], "total": N, ...}`
/// directly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    /// Total items across all pages.
    pub total: u64,
    /// 1-based page number.
    pub page: u32,
    /// Items per page, as requested.
    pub per_page: u32,
}

impl<T> Page<T> {
    pub const fn new(items: Vec<T>, total: u64, page: u32, per_page: u32) -> Self {
        Self {
            items,
            total,
            page,
            per_page,
        }
    }

    pub const fn empty() -> Self {
        Self {
            items: Vec::new(),
            total: 0,
            page: 1,
            per_page: 0,
        }
    }

    /// Total number of pages, given the current `per_page` setting.
    #[must_use]
    pub fn total_pages(&self) -> u64 {
        if self.per_page == 0 {
            return 0;
        }
        let pp = u64::from(self.per_page);
        self.total.div_ceil(pp)
    }

    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> Page<U> {
        Page {
            items: self.items.into_iter().map(&mut f).collect(),
            total: self.total,
            page: self.page,
            per_page: self.per_page,
        }
    }
}

impl<T> Default for Page<T> {
    fn default() -> Self {
        Self::empty()
    }
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
    fn new_preserves_fields() {
        let p = Page::new(vec![1, 2, 3], 100, 2, 10);
        assert_eq!(p.items, vec![1, 2, 3]);
        assert_eq!(p.total, 100);
        assert_eq!(p.page, 2);
        assert_eq!(p.per_page, 10);
    }

    #[test]
    fn empty_has_zero_total_and_per_page() {
        let p = Page::<i32>::empty();
        assert!(p.items.is_empty());
        assert_eq!(p.total, 0);
        assert_eq!(p.page, 1);
        assert_eq!(p.per_page, 0);
    }

    #[test]
    fn map_preserves_meta_and_transforms_items() {
        let p = Page::new(vec![1, 2], 5, 1, 2);
        let mapped = p.map(|n| n.to_string());
        assert_eq!(mapped.items, vec!["1".to_string(), "2".to_string()]);
        assert_eq!(mapped.total, 5);
        assert_eq!(mapped.page, 1);
        assert_eq!(mapped.per_page, 2);
    }

    #[test]
    fn total_pages_ceiling_math() {
        let p: Page<i32> = Page::new(Vec::new(), 23, 1, 10);
        assert_eq!(p.total_pages(), 3);
        let exact: Page<i32> = Page::new(Vec::new(), 20, 1, 10);
        assert_eq!(exact.total_pages(), 2);
        let one: Page<i32> = Page::new(Vec::new(), 1, 1, 10);
        assert_eq!(one.total_pages(), 1);
    }

    #[test]
    fn total_pages_returns_zero_when_per_page_zero() {
        let p: Page<i32> = Page::new(Vec::new(), 100, 1, 0);
        assert_eq!(p.total_pages(), 0);
    }

    #[test]
    fn serde_roundtrip() {
        let p = Page::new(vec!["a".to_string()], 1, 1, 1);
        let json = serde_json::to_string(&p).expect("serialize");
        let back: Page<String> = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(p, back);
    }

    #[test]
    fn default_is_empty() {
        let p: Page<i32> = Page::default();
        assert!(p.items.is_empty());
        assert_eq!(p.total, 0);
    }
}
