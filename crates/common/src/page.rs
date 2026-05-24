//! `Page<T>` — keyset cursor pagination wrapper.
//!
//! `ENTITY.md §11.6` запрещает `OFFSET` (O(N) на больших таблицах, tenant noisy-neighbor
//! risk). Все paginated APIs возвращают `Page<T>` с opaque `next_cursor` строкой;
//! клиент передаёт её обратно как `?cursor=<...>` для следующей страницы.

use serde::{Deserialize, Serialize};

/// Pagination wrapper для list endpoints.
///
/// `next_cursor` — opaque строка (обычно base64-encoded JSON или `(created_at, id)`-tuple).
/// `None` означает "это была последняя страница".
///
/// `total` НЕ возвращается — count'ы на больших таблицах дорого; если нужно — отдельный
/// endpoint `?include=total` (Phase B).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub limit: u32,
}

impl<T> Page<T> {
    #[must_use]
    pub const fn new(items: Vec<T>, next_cursor: Option<String>, limit: u32) -> Self {
        Self {
            items,
            next_cursor,
            limit,
        }
    }

    #[must_use]
    pub fn empty(limit: u32) -> Self {
        Self {
            items: Vec::new(),
            next_cursor: None,
            limit,
        }
    }

    #[must_use]
    pub fn map<U, F: FnMut(T) -> U>(self, f: F) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            next_cursor: self.next_cursor,
            limit: self.limit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_empty_has_no_items() {
        let p: Page<i32> = Page::empty(50);
        assert_eq!(p.items.len(), 0);
        assert_eq!(p.limit, 50);
        assert!(p.next_cursor.is_none());
    }

    #[test]
    fn page_map_transforms_items() {
        let p = Page::new(vec![1, 2, 3], None, 10);
        let mapped = p.map(|x| x * 2);
        assert_eq!(mapped.items, vec![2, 4, 6]);
        assert_eq!(mapped.limit, 10);
    }

    #[test]
    fn page_serializes_camel_case() {
        let p = Page::new(vec!["a", "b"], Some("cursor-x".to_string()), 50);
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["items"], serde_json::json!(["a", "b"]));
        assert_eq!(json["nextCursor"], "cursor-x");
        assert_eq!(json["limit"], 50);
    }
}
