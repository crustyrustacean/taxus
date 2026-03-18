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

    /// Rendered HTML content
    pub content: String,

    /// Raw markdown content
    pub raw_content: String,

    /// Publication date (ISO 8601 format)
    pub date: Option<String>,

    /// Whether this is a draft
    pub draft: bool,
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

    /// Section URL path (e.g., "/blog/")
    pub path: String,

    /// Pages in this section
    pub pages: Vec<PageContext>,
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

impl TemplateContext {
    /// Create a new template context with site defaults.
    ///
    /// # Example
    ///
    /// ```
    /// use generator::templates::{TemplateContext, SiteContext};
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
    /// use generator::templates::{TemplateContext, SiteContext, PageContext};
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
    ///     content: String::new(),
    ///     raw_content: String::new(),
    ///     date: None,
    ///     draft: false,
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
    /// use generator::templates::{TemplateContext, SiteContext, SectionContext};
    ///
    /// let site = SiteContext {
    ///     name: "Test".to_string(),
    ///     base_url: "https://example.com".to_string(),
    ///     description: None,
    ///     author: None,
    /// };
    /// let section = SectionContext {
    ///     title: "Blog".to_string(),
    ///     path: "/blog/".to_string(),
    ///     pages: vec![],
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
    /// use generator::templates::{TemplateContext, SiteContext};
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
            content: "<p>Content</p>".to_string(),
            raw_content: "Content".to_string(),
            date: Some("2024-01-15".to_string()),
            draft: false,
        }
    }

    fn create_test_section() -> SectionContext {
        SectionContext {
            title: "Blog".to_string(),
            path: "/blog/".to_string(),
            pages: vec![create_test_page()],
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
            "content": "<p>Hello</p>",
            "raw_content": "Hello",
            "date": "2024-01-15",
            "draft": false
        }"#;

        let page: PageContext = serde_json::from_str(json).unwrap();
        assert_eq!(page.title, "Test");
        assert_eq!(page.path, "/test/");
        assert!(!page.draft);
    }
}
