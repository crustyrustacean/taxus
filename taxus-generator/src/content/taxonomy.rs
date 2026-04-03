//! Taxonomy support for content organization.
//!
//! Taxonomies allow grouping content by categories, tags, and series.

use std::collections::HashMap;

use super::Page;

/// Type of taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaxonomyKind {
    /// Tags for content (can have multiple per page)
    Tag,
    /// Categories for content (can have multiple per page)
    Category,
    /// Series for multi-part content (single per page)
    Series,
}

impl TaxonomyKind {
    /// Get the URL path prefix for this taxonomy kind.
    pub fn path_prefix(&self) -> &'static str {
        match self {
            TaxonomyKind::Tag => "tags",
            TaxonomyKind::Category => "categories",
            TaxonomyKind::Series => "series",
        }
    }

    /// Get the plural name for this taxonomy kind.
    pub fn plural_name(&self) -> &'static str {
        match self {
            TaxonomyKind::Tag => "Tags",
            TaxonomyKind::Category => "Categories",
            TaxonomyKind::Series => "Series",
        }
    }
}

/// A single taxonomy term with associated pages.
#[derive(Debug, Clone)]
pub struct TaxonomyTerm {
    /// The taxonomy kind (tag, category, series)
    pub kind: TaxonomyKind,

    /// The term name (e.g., "rust", "tutorial")
    pub name: String,

    /// Slug for URL generation
    pub slug: String,

    /// Number of pages with this term
    pub page_count: usize,

    /// Paths to pages with this term
    pub page_paths: Vec<String>,
}

impl TaxonomyTerm {
    /// Create a new taxonomy term.
    pub fn new(kind: TaxonomyKind, name: &str) -> Self {
        let slug = slugify(name);
        Self {
            kind,
            name: name.to_string(),
            slug,
            page_count: 0,
            page_paths: Vec::new(),
        }
    }

    /// Add a page to this term.
    pub fn add_page(&mut self, page_path: &str) {
        self.page_paths.push(page_path.to_string());
        self.page_count = self.page_paths.len();
    }

    /// Get the URL path for this term's listing page.
    pub fn url_path(&self) -> String {
        format!("/{}/{}/", self.kind.path_prefix(), self.slug)
    }
}

/// Collection of all taxonomy terms organized by kind.
#[derive(Debug, Clone, Default)]
pub struct TaxonomyMap {
    /// Tags mapped by slug
    tags: HashMap<String, TaxonomyTerm>,

    /// Categories mapped by slug
    categories: HashMap<String, TaxonomyTerm>,

    /// Series mapped by slug
    series: HashMap<String, TaxonomyTerm>,
}

impl TaxonomyMap {
    /// Create a new empty taxonomy map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a taxonomy map from a collection of pages.
    pub fn from_pages(pages: &[Page]) -> Self {
        let mut map = Self::new();

        for page in pages {
            // Skip drafts
            if page.frontmatter.draft {
                continue;
            }

            // Add tags
            for tag in page.tags() {
                map.add_term(TaxonomyKind::Tag, tag, &page.path);
            }

            // Add categories
            for category in page.categories() {
                map.add_term(TaxonomyKind::Category, category, &page.path);
            }

            // Add series
            if let Some(series) = page.series() {
                map.add_term(TaxonomyKind::Series, series, &page.path);
            }
        }

        map
    }

    /// Add a term to the appropriate map.
    pub fn add_term(&mut self, kind: TaxonomyKind, name: &str, page_path: &str) {
        let map = match kind {
            TaxonomyKind::Tag => &mut self.tags,
            TaxonomyKind::Category => &mut self.categories,
            TaxonomyKind::Series => &mut self.series,
        };

        let slug = slugify(name);
        let term = map
            .entry(slug.clone())
            .or_insert_with(|| TaxonomyTerm::new(kind, name));
        term.add_page(page_path);
    }

    /// Get all tags.
    pub fn tags(&self) -> Vec<&TaxonomyTerm> {
        let mut tags: Vec<_> = self.tags.values().collect();
        tags.sort_by(|a, b| a.name.cmp(&b.name));
        tags
    }

    /// Get all categories.
    pub fn categories(&self) -> Vec<&TaxonomyTerm> {
        let mut categories: Vec<_> = self.categories.values().collect();
        categories.sort_by(|a, b| a.name.cmp(&b.name));
        categories
    }

    /// Get all series.
    pub fn series(&self) -> Vec<&TaxonomyTerm> {
        let mut series: Vec<_> = self.series.values().collect();
        series.sort_by(|a, b| a.name.cmp(&b.name));
        series
    }

    /// Get a specific tag by slug.
    pub fn get_tag(&self, slug: &str) -> Option<&TaxonomyTerm> {
        self.tags.get(slug)
    }

    /// Get a specific category by slug.
    pub fn get_category(&self, slug: &str) -> Option<&TaxonomyTerm> {
        self.categories.get(slug)
    }

    /// Get a specific series by slug.
    pub fn get_series(&self, slug: &str) -> Option<&TaxonomyTerm> {
        self.series.get(slug)
    }

    /// Check if there are any taxonomies.
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty() && self.categories.is_empty() && self.series.is_empty()
    }

    /// Get total number of taxonomy terms.
    pub fn total_terms(&self) -> usize {
        self.tags.len() + self.categories.len() + self.series.len()
    }
}

/// Convert a term name to a URL-safe slug.
fn slugify(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .replace([' ', '_'], "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect();

    // Collapse consecutive dashes into one
    let mut result = String::new();
    let mut prev_dash = false;
    for c in slug.chars() {
        if c == '-' {
            if !prev_dash {
                result.push(c);
                prev_dash = true;
            }
        } else {
            result.push(c);
            prev_dash = false;
        }
    }

    // Trim leading and trailing dashes
    result.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taxonomy_kind_path_prefix() {
        assert_eq!(TaxonomyKind::Tag.path_prefix(), "tags");
        assert_eq!(TaxonomyKind::Category.path_prefix(), "categories");
        assert_eq!(TaxonomyKind::Series.path_prefix(), "series");
    }

    #[test]
    fn test_taxonomy_kind_plural_name() {
        assert_eq!(TaxonomyKind::Tag.plural_name(), "Tags");
        assert_eq!(TaxonomyKind::Category.plural_name(), "Categories");
        assert_eq!(TaxonomyKind::Series.plural_name(), "Series");
    }

    #[test]
    fn test_taxonomy_term_new() {
        let term = TaxonomyTerm::new(TaxonomyKind::Tag, "Rust Programming");
        assert_eq!(term.kind, TaxonomyKind::Tag);
        assert_eq!(term.name, "Rust Programming");
        assert_eq!(term.slug, "rust-programming");
        assert_eq!(term.page_count, 0);
        assert!(term.page_paths.is_empty());
    }

    #[test]
    fn test_taxonomy_term_add_page() {
        let mut term = TaxonomyTerm::new(TaxonomyKind::Tag, "rust");
        term.add_page("/blog/post-1/");
        term.add_page("/blog/post-2/");

        assert_eq!(term.page_count, 2);
        assert_eq!(term.page_paths, vec!["/blog/post-1/", "/blog/post-2/"]);
    }

    #[test]
    fn test_taxonomy_term_url_path() {
        let term = TaxonomyTerm::new(TaxonomyKind::Tag, "Rust");
        assert_eq!(term.url_path(), "/tags/rust/");

        let category = TaxonomyTerm::new(TaxonomyKind::Category, "Web Development");
        assert_eq!(category.url_path(), "/categories/web-development/");

        let series = TaxonomyTerm::new(TaxonomyKind::Series, "Tutorial Series");
        assert_eq!(series.url_path(), "/series/tutorial-series/");
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Rust"), "rust");
        assert_eq!(slugify("Web Development"), "web-development");
        assert_eq!(slugify("Hello_World"), "hello-world");
        assert_eq!(slugify("Test & Demo!"), "test-demo");
        assert_eq!(slugify("Multiple   Spaces"), "multiple-spaces");
    }

    #[test]
    fn test_taxonomy_map_new() {
        let map = TaxonomyMap::new();
        assert!(map.is_empty());
        assert_eq!(map.total_terms(), 0);
    }

    #[test]
    fn test_taxonomy_map_add_term() {
        let mut map = TaxonomyMap::new();

        map.add_term(TaxonomyKind::Tag, "rust", "/blog/post-1/");
        map.add_term(TaxonomyKind::Tag, "rust", "/blog/post-2/");
        map.add_term(TaxonomyKind::Tag, "web", "/blog/post-1/");

        assert!(!map.is_empty());
        assert_eq!(map.total_terms(), 2); // rust and web

        let rust_tag = map.get_tag("rust").unwrap();
        assert_eq!(rust_tag.page_count, 2);

        let web_tag = map.get_tag("web").unwrap();
        assert_eq!(web_tag.page_count, 1);
    }

    #[test]
    fn test_taxonomy_map_from_pages() {
        let content1 = r#"
+++
title = "Post 1"
tags = ["rust", "web"]
categories = ["Tutorial"]
+++
Content
"#;

        let content2 = r#"
+++
title = "Post 2"
tags = ["rust"]
series = "Learning Rust"
+++
Content
"#;

        let page1 = Page::from_str(content1.trim_start(), "post-1.md").unwrap();
        let page2 = Page::from_str(content2.trim_start(), "post-2.md").unwrap();

        let map = TaxonomyMap::from_pages(&[page1, page2]);

        assert_eq!(map.tags().len(), 2); // rust, web
        assert_eq!(map.categories().len(), 1); // Tutorial
        assert_eq!(map.series().len(), 1); // Learning Rust

        let rust_tag = map.get_tag("rust").unwrap();
        assert_eq!(rust_tag.page_count, 2);
    }

    #[test]
    fn test_taxonomy_map_sorted_output() {
        let mut map = TaxonomyMap::new();

        map.add_term(TaxonomyKind::Tag, "zebra", "/post-1/");
        map.add_term(TaxonomyKind::Tag, "apple", "/post-2/");
        map.add_term(TaxonomyKind::Tag, "mango", "/post-3/");

        let tags = map.tags();
        assert_eq!(tags[0].name, "apple");
        assert_eq!(tags[1].name, "mango");
        assert_eq!(tags[2].name, "zebra");
    }

    #[test]
    fn test_taxonomy_map_skips_drafts() {
        let content1 = r#"
+++
title = "Published"
tags = ["rust"]
+++
Content
"#;

        let content2 = r#"
+++
title = "Draft"
tags = ["rust", "draft"]
draft = true
+++
Content
"#;

        let page1 = Page::from_str(content1.trim_start(), "published.md").unwrap();
        let page2 = Page::from_str(content2.trim_start(), "draft.md").unwrap();

        let map = TaxonomyMap::from_pages(&[page1, page2]);

        // Draft page should not contribute to taxonomy
        assert_eq!(map.tags().len(), 1); // Only "rust" from published page
        let rust_tag = map.get_tag("rust").unwrap();
        assert_eq!(rust_tag.page_count, 1); // Only from published page
    }
}
