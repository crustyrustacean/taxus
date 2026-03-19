//! Page type for individual content files.

use crate::error::{ContentError, Result};
use std::path::{Path, PathBuf};

use super::Frontmatter;

/// A single page with frontmatter and Markdown content.
#[derive(Debug, Clone)]
pub struct Page {
    /// Page metadata from frontmatter
    pub frontmatter: Frontmatter,

    /// URL path (e.g., "/about/")
    pub path: String,

    /// Source file path relative to content directory
    pub source: PathBuf,

    /// Raw Markdown content (without frontmatter)
    pub raw_content: String,

    /// Rendered HTML content (set after rendering)
    pub content: Option<String>,
}

impl Page {
    /// Parse a page from a Markdown file.
    ///
    /// The file should contain TOML frontmatter between `+++` markers.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use yew_ssg_lib::content::Page;
    ///
    /// let page = Page::from_file("content/about.md")?;
    /// println!("Title: {}", page.frontmatter.title);
    /// println!("Path: {}", page.path);
    /// # Ok::<(), yew_ssg_lib::error::GeneratorError>(())
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| ContentError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        let source = path
            .file_name()
            .ok_or_else(|| ContentError::InvalidPath(path.display().to_string()))?
            .to_string_lossy()
            .to_string();

        Self::from_str(&content, &source)
    }

    /// Parse a page from a string with explicit source name.
    ///
    /// # Example
    ///
    /// ```
    /// use yew_ssg_lib::content::Page;
    ///
    /// let content = r#"+++
    /// title = "Test Page"
    /// +++
    ///
    /// # Hello World
    /// "#;
    ///
    /// let page = Page::from_str(content, "test.md")?;
    /// assert_eq!(page.frontmatter.title, "Test Page");
    /// assert_eq!(page.path, "/test/");
    /// # Ok::<(), yew_ssg_lib::error::GeneratorError>(())
    /// ```
    pub fn from_str(content: &str, source: &str) -> Result<Self> {
        let (frontmatter, raw_content) = Self::parse_frontmatter(content, source)?;

        // Generate URL path from source filename
        let path = Self::source_to_path(source);

        Ok(Self {
            frontmatter,
            path,
            source: PathBuf::from(source),
            raw_content,
            content: None,
        })
    }

    /// Parse frontmatter from content string.
    fn parse_frontmatter(content: &str, source: &str) -> Result<(Frontmatter, String)> {
        // Normalize line endings to \n
        let content = content.replace("\r\n", "\n");

        // Check for frontmatter markers
        if !content.starts_with("+++\n") {
            return Ok((Frontmatter::default(), content.to_string()));
        }

        // Find closing marker - handle both "\n+++\n" and "+++\n" (empty frontmatter)
        let end = if content[4..].starts_with("+++\n") {
            // Empty frontmatter: +++\n+++\n
            4
        } else {
            // Normal case: +++\n...\n+++\n
            content[4..]
                .find("\n+++\n")
                .map(|i| i + 4)
                .ok_or_else(|| ContentError::UnclosedFrontmatter(PathBuf::from(source)))?
        };

        let fm_str = &content[4..end];
        let body_start = if end == 4 {
            // Empty frontmatter: skip "+++\n+++\n"
            8
        } else {
            // Normal case: skip content + "\n+++\n"
            end + 5
        };
        let body = content[body_start..].trim_start().to_string();

        let frontmatter =
            Frontmatter::from_str(fm_str).map_err(|e| ContentError::InvalidFrontmatter {
                path: PathBuf::from(source),
                source: e,
            })?;

        Ok((frontmatter, body))
    }

    /// Convert source filename to URL path.
    fn source_to_path(source: &str) -> String {
        let stem = Path::new(source)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("index");

        if stem == "_index" {
            "/".to_string()
        } else {
            format!("/{}/", stem)
        }
    }

    /// Get the template name for this page.
    pub fn template(&self) -> &str {
        self.frontmatter.template()
    }

    /// Check if this page is a draft.
    pub fn is_draft(&self) -> bool {
        self.frontmatter.draft
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_page_with_frontmatter() {
        let content = r#"
+++
title = "Test Page"
description = "A test"
+++

# Hello World

This is content.
"#;

        let page = Page::from_str(content.trim_start(), "test.md").unwrap();

        assert_eq!(page.frontmatter.title, "Test Page");
        assert_eq!(page.frontmatter.description, Some("A test".to_string()));
        assert!(page.raw_content.contains("Hello World"));
    }

    #[test]
    fn test_parse_page_without_frontmatter() {
        let content = "# Just content\n\nNo frontmatter here.";
        let page = Page::from_str(content, "test.md").unwrap();

        assert!(page.frontmatter.title.is_empty());
        assert_eq!(page.raw_content, content);
    }

    #[test]
    fn test_parse_page_empty_frontmatter() {
        let content = "+++\n+++\n\n# Content";
        let page = Page::from_str(content, "test.md").unwrap();

        assert!(page.frontmatter.title.is_empty());
        assert!(page.raw_content.contains("# Content"));
    }

    #[test]
    fn test_source_to_path_regular() {
        assert_eq!(Page::source_to_path("about.md"), "/about/");
        assert_eq!(Page::source_to_path("contact.md"), "/contact/");
    }

    #[test]
    fn test_source_to_path_index() {
        assert_eq!(Page::source_to_path("_index.md"), "/");
    }

    #[test]
    fn test_is_draft() {
        let content = "+++\ntitle = \"Test\"\ndraft = true\n+++\nContent";
        let page = Page::from_str(content, "test.md").unwrap();
        assert!(page.is_draft());

        let content = "+++\ntitle = \"Test\"\n+++\nContent";
        let page = Page::from_str(content, "test.md").unwrap();
        assert!(!page.is_draft());
    }

    #[test]
    fn test_template() {
        let content = "+++\ntitle = \"Test\"\n+++\nContent";
        let page = Page::from_str(content, "test.md").unwrap();
        assert_eq!(page.template(), "page.html");

        let content = "+++\ntitle = \"Test\"\ntemplate = \"custom.html\"\n+++\nContent";
        let page = Page::from_str(content, "test.md").unwrap();
        assert_eq!(page.template(), "custom.html");
    }

    #[test]
    fn test_error_malformed_frontmatter() {
        let content = "+++\ninvalid[\n+++\nContent";
        let result = Page::from_str(content, "test.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_unclosed_frontmatter() {
        let content = "+++\ntitle = \"Test\"\n\nNo closing marker";
        let result = Page::from_str(content, "test.md");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            crate::error::GeneratorError::Content(ContentError::UnclosedFrontmatter(_))
        ));
    }

    #[test]
    fn test_page_from_str_with_path() {
        let content = "+++\ntitle = \"About\"\n+++\nAbout page";
        let page = Page::from_str(content, "about.md").unwrap();

        assert_eq!(page.path, "/about/");
        assert_eq!(page.source, PathBuf::from("about.md"));
    }
}
