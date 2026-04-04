// taxus-generator/src/templates/context.rs

//! Template context types for rendering.
//!
//! This module provides the context types that hold variables available
//! to templates during rendering. These types are serialized to JSON
//! and passed to the template engine.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Context for template rendering containing all available variables.
///
/// This is the main context type passed to templates. It contains
/// site-wide configuration, the current page or section being rendered,
/// and any extra variables from frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateContext {
    /// Current page or section being rendered
    pub page: Option<PageContext>,

    /// Current section (if rendering a section)
    pub section: Option<SectionContext>,

    /// Site-wide configuration
    pub site: SiteContext,

    /// Current date/time information
    pub now: NowContext,

    /// Custom variables from frontmatter extra field
    pub extra: HashMap<String, JsonValue>,
}

/// Current date/time context for templates.
///
/// Provides the current year and other date information for use in templates,
/// such as copyright notices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NowContext {
    /// Current year (e.g., 2024)
    pub year: i32,
}

/// Page-specific context for templates.
///
/// Contains all the variables available for a single page,
/// including its frontmatter metadata and rendered content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContext {
    /// Page title from frontmatter
    pub title: String,

    /// Page description
    pub description: Option<String>,

    /// Page URL path (e.g., "/about/")
    pub path: String,

    /// Pre-computed absolute URL combining base_url and path
    /// (e.g., "https://example.com/about/")
    pub permalink: String,

    /// Rendered HTML content
    pub content: String,

    /// Raw markdown content
    pub raw_content: String,

    /// Publication date (ISO 8601 format)
    pub date: Option<String>,

    /// Whether this is a draft
    pub draft: bool,

    /// Summary/excerpt for the page
    pub summary: String,

    /// Word count for the page
    pub word_count: usize,

    /// Estimated reading time in minutes
    pub reading_time: usize,

    /// Tags for the page
    #[serde(default)]
    pub tags: Vec<String>,

    /// Categories for the page
    #[serde(default)]
    pub categories: Vec<String>,

    /// Series name for the page
    #[serde(default)]
    pub series: Option<String>,
}

/// Compute a permalink by joining base_url and path with proper slash handling.
///
/// Ensures there are no double slashes or missing slashes between the base URL
/// and the path. The result is always a properly formed absolute URL.
///
/// # Examples
///
/// ```
/// use taxus_lib::templates::compute_permalink;
///
/// // base_url without trailing slash, path with leading slash
/// assert_eq!(compute_permalink("https://example.com", "/about/"), "https://example.com/about/");
///
/// // base_url with trailing slash, path with leading slash
/// assert_eq!(compute_permalink("https://example.com/", "/about/"), "https://example.com/about/");
///
/// // Root path
/// assert_eq!(compute_permalink("https://example.com", "/"), "https://example.com/");
/// ```
pub fn compute_permalink(base_url: &str, path: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    if path.is_empty() {
        format!("{}/", base)
    } else {
        format!("{}/{}", base, path)
    }
}

/// Section-specific context for templates.
///
/// A section represents a collection of pages, such as a blog.
/// The section context includes metadata about the section and
/// a list of all pages within it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionContext {
    /// Section title
    pub title: String,

    /// Section description (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Section URL path (e.g., "/blog/")
    pub path: String,

    /// Section HTML content (rendered from markdown)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Pages in this section
    pub pages: Vec<PageContext>,

    /// Pagination information (if this is a paginated section)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationContext>,
}

/// Pagination context for templates.
///
/// Contains all pagination information needed to render navigation
/// and display the current page's position in the paginated sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationContext {
    /// Current page number (1-indexed)
    pub current: usize,

    /// Total number of pages
    pub total: usize,

    /// Number of items per page
    pub per_page: usize,

    /// Total number of items across all pages
    pub total_items: usize,

    /// URL path to previous page (None if on first page)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,

    /// URL path to next page (None if on last page)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,

    /// URL path to first page
    pub first: String,

    /// URL path to last page
    pub last: String,
}

impl PaginationContext {
    /// Check if this is the first page.
    pub fn is_first(&self) -> bool {
        self.current == 1
    }

    /// Check if this is the last page.
    pub fn is_last(&self) -> bool {
        self.current >= self.total
    }

    /// Get page numbers for pagination navigation.
    ///
    /// Returns a list of page numbers to display in pagination UI,
    /// with gaps represented by None values.
    pub fn page_range(&self) -> Vec<Option<usize>> {
        if self.total <= 7 {
            // Show all pages if 7 or fewer
            (1..=self.total).map(Some).collect()
        } else {
            let mut pages = Vec::new();

            // Always show first page
            pages.push(Some(1));

            if self.current > 3 {
                pages.push(None); // Gap indicator
            }

            // Show pages around current
            let start = std::cmp::max(2, self.current.saturating_sub(1));
            let end = std::cmp::min(self.total - 1, self.current + 1);

            for i in start..=end {
                pages.push(Some(i));
            }

            if self.current < self.total - 2 {
                pages.push(None); // Gap indicator
            }

            // Always show last page
            if self.total > 1 {
                pages.push(Some(self.total));
            }

            pages
        }
    }
}

/// Site-wide context for templates.
///
/// Contains global site configuration that is available to all templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteContext {
    /// Site name
    pub name: String,

    /// Base URL (e.g., "https://example.com")
    pub base_url: String,

    /// Site description
    pub description: Option<String>,

    /// Site author
    pub author: Option<String>,
}

/// Context for a single taxonomy term (e.g., a tag or category).
///
/// Contains information about a specific taxonomy term and its associated pages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyTermContext {
    /// The taxonomy kind (tag, category, series)
    pub kind: String,

    /// The display name of the term
    pub name: String,

    /// URL-safe slug for the term
    pub slug: String,

    /// URL path for the term page (e.g., "/tags/rust/")
    pub path: String,

    /// Number of pages with this term
    pub page_count: usize,

    /// Pages associated with this term
    pub pages: Vec<PageContext>,
}

/// Context for a taxonomy listing page (e.g., "/tags/").
///
/// Contains all terms for a specific taxonomy kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyListContext {
    /// The taxonomy kind (tag, category, series)
    pub kind: String,

    /// URL path for the taxonomy list (e.g., "/tags/")
    pub path: String,

    /// All terms for this taxonomy
    pub terms: Vec<TaxonomyTermContext>,
}

impl TemplateContext {
    /// Create a new template context with site defaults.
    ///
    /// # Example
    ///
    /// ```
    /// use taxus_lib::templates::{TemplateContext, SiteContext};
    ///
    /// let site = SiteContext {
    ///     name: "My Site".to_string(),
    ///     base_url: "https://example.com".to_string(),
    ///     description: None,
    ///     author: None,
    /// };
    ///
    /// let ctx = TemplateContext::new(site);
    /// assert_eq!(ctx.site.name, "My Site");
    /// ```
    pub fn new(site: SiteContext) -> Self {
        use chrono::{Datelike, Utc};
        Self {
            page: None,
            section: None,
            site,
            now: NowContext {
                year: Utc::now().year(),
            },
            extra: HashMap::new(),
        }
    }

    /// Add page context.
    ///
    /// # Example
    ///
    /// ```
    /// use taxus_lib::templates::{TemplateContext, SiteContext, PageContext};
    ///
    /// let site = SiteContext {
    ///     name: "Test".to_string(),
    ///     base_url: "https://example.com".to_string(),
    ///     description: None,
    ///     author: None,
    /// };
    /// let page = PageContext {
    ///     title: "Test Page".to_string(),
    ///     description: None,
    ///     path: "/test/".to_string(),
    ///     permalink: "https://example.com/test/".to_string(),
    ///     content: String::new(),
    ///     raw_content: String::new(),
    ///     date: None,
    ///     draft: false,
    ///     summary: String::new(),
    ///     word_count: 0,
    ///     reading_time: 0,
    ///     tags: vec![],
    ///     categories: vec![],
    ///     series: None,
    /// };
    ///
    /// let ctx = TemplateContext::new(site).with_page(page);
    /// assert!(ctx.page.is_some());
    /// ```
    pub fn with_page(mut self, page: PageContext) -> Self {
        self.page = Some(page);
        self
    }

    /// Add section context.
    ///
    /// # Example
    ///
    /// ```
    /// use taxus_lib::templates::{TemplateContext, SiteContext, SectionContext};
    ///
    /// let site = SiteContext {
    ///     name: "Test".to_string(),
    ///     base_url: "https://example.com".to_string(),
    ///     description: None,
    ///     author: None,
    /// };
    /// let section = SectionContext {
    ///     title: "Blog".to_string(),
    ///     description: None,
    ///     path: "/blog/".to_string(),
    ///     content: None,
    ///     pages: vec![],
    ///     pagination: None,
    /// };
    ///
    /// let ctx = TemplateContext::new(site).with_section(section);
    /// assert!(ctx.section.is_some());
    /// ```
    pub fn with_section(mut self, section: SectionContext) -> Self {
        self.section = Some(section);
        self
    }

    /// Add extra variables.
    ///
    /// # Example
    ///
    /// ```
    /// use taxus_lib::templates::{TemplateContext, SiteContext};
    /// use std::collections::HashMap;
    /// use serde_json::json;
    ///
    /// let site = SiteContext {
    ///     name: "Test".to_string(),
    ///     base_url: "https://example.com".to_string(),
    ///     description: None,
    ///     author: None,
    /// };
    ///
    /// let mut extra = HashMap::new();
    /// extra.insert("custom".to_string(), json!("value"));
    ///
    /// let ctx = TemplateContext::new(site).with_extra(extra);
    /// assert_eq!(ctx.extra.len(), 1);
    /// ```
    pub fn with_extra(mut self, extra: HashMap<String, JsonValue>) -> Self {
        self.extra = extra;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_site() -> SiteContext {
        SiteContext {
            name: "Test Site".to_string(),
            base_url: "https://example.com".to_string(),
            description: Some("A test site".to_string()),
            author: Some("Test Author".to_string()),
        }
    }

    fn create_test_page() -> PageContext {
        PageContext {
            title: "Test Page".to_string(),
            description: Some("A test page".to_string()),
            path: "/test/".to_string(),
            permalink: "https://example.com/test/".to_string(),
            content: "<p>Content</p>".to_string(),
            raw_content: "Content".to_string(),
            date: Some("2024-01-15".to_string()),
            draft: false,
            summary: "A test page summary".to_string(),
            word_count: 1,
            reading_time: 1,
            tags: vec!["rust".to_string(), "tutorial".to_string()],
            categories: vec!["programming".to_string()],
            series: Some("Learning Rust".to_string()),
        }
    }

    fn create_test_section() -> SectionContext {
        SectionContext {
            title: "Blog".to_string(),
            description: Some("Blog section description".to_string()),
            path: "/blog/".to_string(),
            content: Some("<p>Welcome to the blog.</p>".to_string()),
            pages: vec![create_test_page()],
            pagination: None,
        }
    }

    #[test]
    fn test_template_context_new() {
        let site = create_test_site();
        let ctx = TemplateContext::new(site);

        assert_eq!(ctx.site.name, "Test Site");
        assert!(ctx.page.is_none());
        assert!(ctx.section.is_none());
        assert!(ctx.extra.is_empty());
    }

    #[test]
    fn test_template_context_with_page() {
        let site = create_test_site();
        let page = create_test_page();
        let ctx = TemplateContext::new(site).with_page(page);

        assert!(ctx.page.is_some());
        assert_eq!(ctx.page.unwrap().title, "Test Page");
    }

    #[test]
    fn test_template_context_with_section() {
        let site = create_test_site();
        let section = create_test_section();
        let ctx = TemplateContext::new(site).with_section(section);

        assert!(ctx.section.is_some());
        assert_eq!(ctx.section.unwrap().title, "Blog");
    }

    #[test]
    fn test_template_context_with_extra() {
        let site = create_test_site();
        let mut extra = HashMap::new();
        extra.insert(
            "custom_var".to_string(),
            JsonValue::String("custom value".to_string()),
        );

        let ctx = TemplateContext::new(site).with_extra(extra);
        assert_eq!(ctx.extra.len(), 1);
        assert!(ctx.extra.contains_key("custom_var"));
    }

    #[test]
    fn test_template_context_builder_chain() {
        let site = create_test_site();
        let page = create_test_page();
        let section = create_test_section();
        let mut extra = HashMap::new();
        extra.insert("key".to_string(), JsonValue::Bool(true));

        let ctx = TemplateContext::new(site)
            .with_page(page)
            .with_section(section)
            .with_extra(extra);

        assert!(ctx.page.is_some());
        assert!(ctx.section.is_some());
        assert_eq!(ctx.extra.len(), 1);
    }

    #[test]
    fn test_page_context_serialization() {
        let page = create_test_page();
        let json = serde_json::to_string(&page).unwrap();

        assert!(json.contains("Test Page"));
        assert!(json.contains("/test/"));
        assert!(json.contains("2024-01-15"));
    }

    #[test]
    fn test_section_context_serialization() {
        let section = create_test_section();
        let json = serde_json::to_string(&section).unwrap();

        assert!(json.contains("Blog"));
        assert!(json.contains("/blog/"));
        assert!(json.contains("Test Page"));
    }

    #[test]
    fn test_site_context_serialization() {
        let site = create_test_site();
        let json = serde_json::to_string(&site).unwrap();

        assert!(json.contains("Test Site"));
        assert!(json.contains("https://example.com"));
        assert!(json.contains("Test Author"));
    }

    #[test]
    fn test_template_context_serialization() {
        let site = create_test_site();
        let page = create_test_page();
        let ctx = TemplateContext::new(site).with_page(page);

        let json = serde_json::to_string(&ctx).unwrap();

        assert!(json.contains("site"));
        assert!(json.contains("page"));
        assert!(json.contains("Test Site"));
        assert!(json.contains("Test Page"));
    }

    #[test]
    fn test_page_context_deserialization() {
        let json = r#"{
            "title": "Test",
            "description": "A test",
            "path": "/test/",
            "permalink": "https://example.com/test/",
            "content": "<p>Hello</p>",
            "raw_content": "Hello",
            "date": "2024-01-15",
            "draft": false,
            "summary": "Test summary",
            "word_count": 5,
            "reading_time": 1
        }"#;

        let page: PageContext = serde_json::from_str(json).unwrap();
        assert_eq!(page.title, "Test");
        assert_eq!(page.path, "/test/");
        assert_eq!(page.permalink, "https://example.com/test/");
        assert!(!page.draft);
        assert_eq!(page.summary, "Test summary");
        assert_eq!(page.word_count, 5);
        assert_eq!(page.reading_time, 1);
    }

    #[test]
    fn test_compute_permalink_slash_handling() {
        // base_url without trailing slash, path with leading slash
        assert_eq!(
            compute_permalink("https://example.com", "/about/"),
            "https://example.com/about/"
        );

        // base_url with trailing slash, path with leading slash
        assert_eq!(
            compute_permalink("https://example.com/", "/about/"),
            "https://example.com/about/"
        );

        // base_url without trailing slash, path without leading slash
        assert_eq!(
            compute_permalink("https://example.com", "about/"),
            "https://example.com/about/"
        );

        // base_url with trailing slash, path without leading slash
        assert_eq!(
            compute_permalink("https://example.com/", "about/"),
            "https://example.com/about/"
        );

        // Root path
        assert_eq!(
            compute_permalink("https://example.com", "/"),
            "https://example.com/"
        );

        // Root path with trailing slash on base_url
        assert_eq!(
            compute_permalink("https://example.com/", "/"),
            "https://example.com/"
        );

        // Path without trailing slash (should still work)
        assert_eq!(
            compute_permalink("https://example.com", "/about"),
            "https://example.com/about"
        );

        // Complex path
        assert_eq!(
            compute_permalink("https://example.com", "/blog/2024/my-post/"),
            "https://example.com/blog/2024/my-post/"
        );

        // Double trailing slash on base_url (should be normalized)
        assert_eq!(
            compute_permalink("https://example.com//", "/about/"),
            "https://example.com/about/"
        );
    }
}
