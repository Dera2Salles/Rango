//! Django-like Paginator
//!
//! Provides cursor-based and offset-based pagination utilities.
//!
//! # Example
//! ```rust,ignore
//! use rango::paginator::Paginator;
//!
//! // From a QuerySet Page (offset pagination)
//! let page = Post::objects().paginate(page_num, 10).await?;
//! let paginator = Paginator::from_page(&page);
//!
//! // Render page info
//! println!("Page {} of {}", paginator.current_page, paginator.num_pages);
//! ```

use serde::{Deserialize, Serialize};

/// Pagination metadata — passed to templates alongside items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginator {
    /// Current page number (1-indexed).
    pub current_page: u64,
    /// Items per page.
    pub per_page: u64,
    /// Total number of items.
    pub total: i64,
    /// Total number of pages.
    pub num_pages: u64,
    /// Whether there is a next page.
    pub has_next: bool,
    /// Whether there is a previous page.
    pub has_prev: bool,
    /// Next page number (None if on last page).
    pub next_page: Option<u64>,
    /// Previous page number (None if on first page).
    pub prev_page: Option<u64>,
    /// List of page numbers to show in the pagination widget.
    /// Includes ellipsis markers (0) for gaps.
    pub page_range: Vec<i64>,
}

impl Paginator {
    /// Create a paginator from raw values.
    pub fn new(current_page: u64, per_page: u64, total: i64) -> Self {
        let num_pages = if per_page == 0 {
            0
        } else {
            (total as u64 + per_page - 1) / per_page
        };
        let has_next = current_page < num_pages;
        let has_prev = current_page > 1;
        let next_page = if has_next {
            Some(current_page + 1)
        } else {
            None
        };
        let prev_page = if has_prev {
            Some(current_page - 1)
        } else {
            None
        };
        let page_range = build_page_range(current_page, num_pages, 5);

        Paginator {
            current_page,
            per_page,
            total,
            num_pages,
            has_next,
            has_prev,
            next_page,
            prev_page,
            page_range,
        }
    }

    /// Create from a `Page` returned by `QuerySet::paginate()`.
    #[cfg(feature = "db")]
    pub fn from_page<T>(page: &crate::db::Page<T>) -> Self {
        Self::new(page.page, page.per_page, page.total)
    }

    /// Calculate the item range displayed on the current page.
    /// Returns `(start, end)` where both are 1-indexed and inclusive.
    pub fn item_range(&self) -> (u64, u64) {
        if self.total == 0 {
            return (0, 0);
        }
        let start = (self.current_page - 1) * self.per_page + 1;
        let end = (start + self.per_page - 1).min(self.total as u64);
        (start, end)
    }

    /// Whether the paginator has multiple pages.
    pub fn is_paginated(&self) -> bool {
        self.num_pages > 1
    }

    /// Convert to a JSON value for use in templates.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// Build a page range with ellipsis markers (0 = ellipsis).
///
/// Example for page 5 of 20 with window 2:
/// `[1, 0, 3, 4, 5, 6, 7, 0, 20]`
fn build_page_range(current: u64, total: u64, window: u64) -> Vec<i64> {
    if total <= 1 {
        return (1..=total).map(|p| p as i64).collect();
    }

    let mut pages: Vec<i64> = Vec::new();

    pages.push(1);

    let start = current.saturating_sub(window).max(2);
    let end = (current + window).min(total.saturating_sub(1));

    if start > 2 {
        pages.push(0); 
    }

    for p in start..=end {
        pages.push(p as i64);
    }

    if end < total.saturating_sub(1) {
        pages.push(0); 
    }

    if total > 1 {
        pages.push(total as i64);
    }

    pages
}

/// Query parameter extraction helper for pagination.
///
/// Extracts `page` and `per_page` from query parameters with sensible defaults.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PaginationParams {
    /// Current page (default: 1, minimum: 1).
    #[serde(default = "default_page")]
    pub page: u64,
    /// Items per page (default: 20, maximum: 100).
    #[serde(default = "default_per_page")]
    pub per_page: u64,
}

fn default_page() -> u64 {
    1
}
fn default_per_page() -> u64 {
    20
}

impl PaginationParams {
    pub fn page(&self) -> u64 {
        self.page.max(1)
    }

    pub fn per_page(&self) -> u64 {
        self.per_page.clamp(1, 100)
    }
}

impl Default for PaginationParams {
    fn default() -> Self {
        PaginationParams {
            page: 1,
            per_page: 20,
        }
    }
}
