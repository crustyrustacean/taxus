//! Pagination types for section content.
//!
//! This module provides types for splitting large collections of pages
//! across multiple pages (pagination).

use serde::{Deserialize, Serialize};

use super::Page;

/// Sort order for pages within a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortBy {
    /// Sort by date (newest first)
    #[default]
    Date,
    /// Sort by title (alphabetically)
    Title,
    /// Sort by weight (lowest first)
    Weight,
    /// No sorting (preserve filesystem order)
    None,
}

/// Pagination information for a single page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    /// Current page number (1-indexed)
    pub current: usize,
    /// Total number of pages
    pub total: usize,
    /// Number of items per page
    pub per_page: usize,
    /// URL path to previous page (None if on first page)
    pub prev: Option<String>,
    /// URL path to next page (None if on last page)
    pub next: Option<String>,
    /// URL path to first page
    pub first: String,
    /// URL path to last page
    pub last: String,
    /// Total number of items across all pages
    pub total_items: usize,
}

impl PaginationInfo {
    /// Create pagination info for a given page.
    pub fn new(
        current: usize,
        total_pages: usize,
        per_page: usize,
        total_items: usize,
        base_path: &str,
    ) -> Self {
        let first = if base_path.ends_with('/') {
            base_path.to_string()
        } else {
            format!("{}/", base_path)
        };

        let page_path = |n: usize| -> String {
            if n == 1 {
                first.clone()
            } else {
                format!("{}/page/{}/", base_path.trim_end_matches('/'), n)
            }
        };

        Self {
            current,
            total: total_pages,
            per_page,
            prev: if current > 1 {
                Some(page_path(current - 1))
            } else {
                None
            },
            next: if current < total_pages {
                Some(page_path(current + 1))
            } else {
                None
            },
            first: first.clone(),
            last: if total_pages > 0 {
                page_path(total_pages)
            } else {
                first
            },
            total_items,
        }
    }

    /// Check if this is the first page.
    pub fn is_first(&self) -> bool {
        self.current == 1
    }

    /// Check if this is the last page.
    pub fn is_last(&self) -> bool {
        self.current >= self.total
    }

    /// Get the offset for database-style queries (0-indexed).
    pub fn offset(&self) -> usize {
        (self.current - 1) * self.per_page
    }
}

/// Paginator for splitting pages into chunks.
#[derive(Debug, Clone)]
pub struct Paginator {
    /// All pages to paginate
    pages: Vec<Page>,
    /// Number of items per page
    per_page: usize,
    /// Base URL path for pagination
    base_path: String,
}

impl Paginator {
    /// Create a new paginator.
    pub fn new(pages: Vec<Page>, per_page: usize, base_path: &str) -> Self {
        let base_path = if base_path.ends_with('/') {
            base_path.to_string()
        } else {
            format!("{}/", base_path)
        };

        Self {
            pages,
            per_page,
            base_path,
        }
    }

    /// Get the total number of pages.
    pub fn total_pages(&self) -> usize {
        if self.per_page == 0 {
            return 1;
        }
        if self.pages.is_empty() {
            return 1;
        }
        (self.pages.len() + self.per_page - 1) / self.per_page
    }

    /// Get the total number of items.
    pub fn total_items(&self) -> usize {
        self.pages.len()
    }

    /// Get pages for a specific page number (1-indexed).
    pub fn get_page(&self, page: usize) -> Option<PaginatedSlice> {
        if page == 0 || page > self.total_pages() {
            return None;
        }

        let start = (page - 1) * self.per_page;
        let end = if self.per_page == 0 {
            self.pages.len()
        } else {
            std::cmp::min(start + self.per_page, self.pages.len())
        };

        let pages = self.pages.get(start..end)?.to_vec();

        Some(PaginatedSlice {
            pages,
            pagination: PaginationInfo::new(
                page,
                self.total_pages(),
                self.per_page,
                self.pages.len(),
                &self.base_path,
            ),
        })
    }

    /// Iterate over all paginated slices.
    pub fn iter(&self) -> impl Iterator<Item = PaginatedSlice> + '_ {
        (1..=self.total_pages()).filter_map(|page| self.get_page(page))
    }

    /// Get all pagination URLs.
    pub fn page_urls(&self) -> Vec<String> {
        let total = self.total_pages();
        (1..=total)
            .map(|n| {
                if n == 1 {
                    self.base_path.clone()
                } else {
                    format!("{}/page/{}/", self.base_path.trim_end_matches('/'), n)
                }
            })
            .collect()
    }
}

/// A slice of paginated pages with pagination info.
#[derive(Debug, Clone)]
pub struct PaginatedSlice {
    /// Pages in this slice
    pub pages: Vec<Page>,
    /// Pagination information
    pub pagination: PaginationInfo,
}

/// Configuration for section pagination.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PaginationConfig {
    /// Number of items per page (0 = no pagination)
    #[serde(default)]
    pub paginate_by: usize,
    /// Sort order for pages
    #[serde(default)]
    pub sort_by: SortBy,
    /// Template for paginated pages (defaults to section template)
    pub paginate_template: Option<String>,
}

impl PaginationConfig {
    /// Create a new pagination config.
    pub fn new(per_page: usize, sort_by: SortBy) -> Self {
        Self {
            paginate_by: per_page,
            sort_by,
            paginate_template: None,
        }
    }

    /// Check if pagination is enabled.
    pub fn is_enabled(&self) -> bool {
        self.paginate_by > 0
    }

    /// Set the pagination template.
    pub fn with_template(mut self, template: impl Into<String>) -> Self {
        self.paginate_template = Some(template.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Frontmatter;

    fn create_test_page(title: &str) -> Page {
        let frontmatter = Frontmatter {
            title: title.to_string(),
            ..Default::default()
        };
        Page {
            frontmatter,
            path: format!("/{}/", title.to_lowercase().replace(' ', "-")),
            raw_content: String::new(),
            source: std::path::PathBuf::from(format!("{}.md", title)),
            content: None,
        }
    }

    #[test]
    fn test_pagination_info_first_page() {
        let info = PaginationInfo::new(1, 5, 10, 50, "/blog");

        assert_eq!(info.current, 1);
        assert_eq!(info.total, 5);
        assert!(info.prev.is_none());
        assert_eq!(info.next, Some("/blog/page/2/".to_string()));
        assert_eq!(info.first, "/blog/");
        assert_eq!(info.last, "/blog/page/5/");
        assert!(info.is_first());
        assert!(!info.is_last());
    }

    #[test]
    fn test_pagination_info_middle_page() {
        let info = PaginationInfo::new(3, 5, 10, 50, "/blog");

        assert_eq!(info.current, 3);
        assert_eq!(info.prev, Some("/blog/page/2/".to_string()));
        assert_eq!(info.next, Some("/blog/page/4/".to_string()));
        assert!(!info.is_first());
        assert!(!info.is_last());
    }

    #[test]
    fn test_pagination_info_last_page() {
        let info = PaginationInfo::new(5, 5, 10, 50, "/blog");

        assert_eq!(info.current, 5);
        assert_eq!(info.prev, Some("/blog/page/4/".to_string()));
        assert!(info.next.is_none());
        assert!(!info.is_first());
        assert!(info.is_last());
    }

    #[test]
    fn test_pagination_info_single_page() {
        let info = PaginationInfo::new(1, 1, 10, 5, "/blog");

        assert_eq!(info.current, 1);
        assert_eq!(info.total, 1);
        assert!(info.prev.is_none());
        assert!(info.next.is_none());
        assert_eq!(info.first, "/blog/");
        assert_eq!(info.last, "/blog/");
    }

    #[test]
    fn test_paginator_total_pages() {
        let pages: Vec<Page> = (0..25).map(|i| create_test_page(&format!("Page {}", i))).collect();

        let paginator = Paginator::new(pages, 10, "/blog");
        assert_eq!(paginator.total_pages(), 3);

        let paginator = Paginator::new(vec![], 10, "/blog");
        assert_eq!(paginator.total_pages(), 1);

        let pages2: Vec<Page> = (0..25).map(|i| create_test_page(&format!("Page {}", i))).collect();
        let paginator = Paginator::new(pages2, 0, "/blog");
        assert_eq!(paginator.total_pages(), 1);
    }

    #[test]
    fn test_paginator_get_page() {
        let pages: Vec<Page> = (0..25)
            .map(|i| create_test_page(&format!("Page {}", i)))
            .collect();

        let paginator = Paginator::new(pages, 10, "/blog");

        // First page
        let slice = paginator.get_page(1).unwrap();
        assert_eq!(slice.pages.len(), 10);
        assert_eq!(slice.pagination.current, 1);

        // Second page
        let slice = paginator.get_page(2).unwrap();
        assert_eq!(slice.pages.len(), 10);
        assert_eq!(slice.pagination.current, 2);

        // Third page (partial)
        let slice = paginator.get_page(3).unwrap();
        assert_eq!(slice.pages.len(), 5);
        assert_eq!(slice.pagination.current, 3);

        // Out of bounds
        assert!(paginator.get_page(0).is_none());
        assert!(paginator.get_page(4).is_none());
    }

    #[test]
    fn test_paginator_page_urls() {
        let pages: Vec<Page> = (0..25).map(|i| create_test_page(&format!("Page {}", i))).collect();

        let paginator = Paginator::new(pages, 10, "/blog");
        let urls = paginator.page_urls();

        assert_eq!(urls.len(), 3);
        assert_eq!(urls[0], "/blog/");
        assert_eq!(urls[1], "/blog/page/2/");
        assert_eq!(urls[2], "/blog/page/3/");
    }

    #[test]
    fn test_paginator_iter() {
        let pages: Vec<Page> = (0..25)
            .map(|i| create_test_page(&format!("Page {}", i)))
            .collect();

        let paginator = Paginator::new(pages, 10, "/blog");
        let all_slices: Vec<_> = paginator.iter().collect();

        assert_eq!(all_slices.len(), 3);
        assert_eq!(all_slices[0].pagination.current, 1);
        assert_eq!(all_slices[1].pagination.current, 2);
        assert_eq!(all_slices[2].pagination.current, 3);
    }

    #[test]
    fn test_pagination_config() {
        let config = PaginationConfig::new(10, SortBy::Date);
        assert!(config.is_enabled());
        assert_eq!(config.paginate_by, 10);
        assert_eq!(config.sort_by, SortBy::Date);

        let config = PaginationConfig::default();
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_sort_by_serde() {
        #[derive(Deserialize)]
        struct SortByWrapper {
            sort_by: SortBy,
        }

        let wrapper: SortByWrapper = toml::from_str("sort_by = \"date\"").unwrap();
        assert_eq!(wrapper.sort_by, SortBy::Date);

        let wrapper: SortByWrapper = toml::from_str("sort_by = \"title\"").unwrap();
        assert_eq!(wrapper.sort_by, SortBy::Title);

        let wrapper: SortByWrapper = toml::from_str("sort_by = \"weight\"").unwrap();
        assert_eq!(wrapper.sort_by, SortBy::Weight);

        let wrapper: SortByWrapper = toml::from_str("sort_by = \"none\"").unwrap();
        assert_eq!(wrapper.sort_by, SortBy::None);
    }
}
