//! Page type for individual content files.

use crate::error::{ContentError, Result};
use std::path::{Path, PathBuf};
use std::str::FromStr;

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

    /// Extract summary from content.
    ///
    /// Priority:
    /// 1. Use frontmatter.summary if set
    /// 2. Split at `<!-- more -->` marker
    /// 3. Use first paragraph as fallback
    pub fn summary(&self) -> String {
        // 1. Use frontmatter summary if set
        if let Some(ref summary) = self.frontmatter.summary {
            return summary.clone();
        }

        // 2. Check for <!-- more --> marker
        if let Some(pos) = self.raw_content.find("<!-- more -->") {
            let summary = self.raw_content[..pos].trim();
            return Self::strip_markdown(summary);
        }

        // 3. Use first paragraph as fallback
        let first_paragraph = self
            .raw_content
            .split("\n\n")
            .next()
            .unwrap_or("")
            .trim();

        Self::strip_markdown(first_paragraph)
    }

    /// Calculate word count from the raw content.
    ///
    /// Strips markdown formatting and counts words (whitespace-separated tokens).
    pub fn word_count(&self) -> usize {
        let stripped = Self::strip_markdown(&self.raw_content);
        stripped
            .split_whitespace()
            .count()
    }

    /// Calculate estimated reading time in minutes.
    ///
    /// Uses 200 words per minute as the average reading speed.
    /// Returns at least 1 minute for any content.
    pub fn reading_time(&self) -> usize {
        const WORDS_PER_MINUTE: usize = 200;
        let words = self.word_count();
        if words == 0 {
            return 0;
        }
        (words + WORDS_PER_MINUTE - 1) / WORDS_PER_MINUTE // Ceiling division
    }

    /// Get the effective slug for this page.
    ///
    /// Returns the custom slug from frontmatter if set, otherwise derives
    /// from the source filename.
    pub fn slug(&self) -> &str {
        if let Some(ref slug) = self.frontmatter.slug {
            slug
        } else {
            // Derive from source filename
            Path::new(&self.source)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("index")
        }
    }

    /// Get the URL path for this page, respecting custom slug.
    ///
    /// If a custom slug is set in frontmatter, uses that instead of the
    /// filename-based path.
    pub fn url_path(&self) -> String {
        let slug = self.slug();
        if slug == "_index" {
            "/".to_string()
        } else {
            format!("/{}/", slug)
        }
    }

    /// Get aliases (alternative URLs) for this page.
    pub fn aliases(&self) -> &[String] {
        &self.frontmatter.aliases
    }

    /// Get tags for this page.
    pub fn tags(&self) -> &[String] {
        &self.frontmatter.tags
    }

    /// Get categories for this page.
    pub fn categories(&self) -> &[String] {
        &self.frontmatter.categories
    }

    /// Get series for this page, if any.
    pub fn series(&self) -> Option<&str> {
        self.frontmatter.series.as_deref()
    }

    /// Strip markdown formatting from text using simple string manipulation.
    fn strip_markdown(text: &str) -> String {
        let mut result = text.to_string();
        
        // Remove headers (## Header -> Header)
        let mut new_result = String::new();
        for line in result.lines() {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("# ") {
                new_result.push_str(rest);
            } else if let Some(rest) = trimmed.strip_prefix("## ") {
                new_result.push_str(rest);
            } else if let Some(rest) = trimmed.strip_prefix("### ") {
                new_result.push_str(rest);
            } else if let Some(rest) = trimmed.strip_prefix("#### ") {
                new_result.push_str(rest);
            } else if let Some(rest) = trimmed.strip_prefix("##### ") {
                new_result.push_str(rest);
            } else if let Some(rest) = trimmed.strip_prefix("###### ") {
                new_result.push_str(rest);
            } else {
                new_result.push_str(line);
            }
            new_result.push('\n');
        }
        result = new_result.trim().to_string();

        // Remove bold (**text** or __text__)
        result = Self::remove_delimiters(&result, "**", "**");
        result = Self::remove_delimiters(&result, "__", "__");

        // Remove italic (*text* or _text_)
        result = Self::remove_delimiters(&result, "*", "*");
        result = Self::remove_delimiters(&result, "_", "_");

        // Remove inline code (`code`)
        result = Self::remove_delimiters(&result, "`", "`");

        // Remove links [text](url) -> text
        result = Self::remove_links(&result);

        // Remove images ![alt](url) -> empty
        result = Self::remove_images(&result);

        result.trim().to_string()
    }

    /// Remove paired delimiters from text (e.g., **bold**, *italic*).
    fn remove_delimiters(text: &str, start: &str, end: &str) -> String {
        let mut result = String::new();
        let mut chars = text.chars().peekable();
        let start_chars: Vec<char> = start.chars().collect();
        let end_chars: Vec<char> = end.chars().collect();
        
        while let Some(c) = chars.next() {
            // Check if we're at the start delimiter
            if c == start_chars[0] {
                let mut matched = true;
                let mut temp: Vec<char> = vec![c];
                
                for expected in &start_chars[1..] {
                    if let Some(&next) = chars.peek() {
                        if next == *expected {
                            temp.push(chars.next().unwrap());
                        } else {
                            matched = false;
                            break;
                        }
                    } else {
                        matched = false;
                        break;
                    }
                }
                
                if matched && start_chars.len() > 1 {
                    // Look for end delimiter
                    let mut content = String::new();
                    let mut found_end = false;
                    
                    while let Some(&next) = chars.peek() {
                        if next == end_chars[0] {
                            let mut end_match = true;
                            let mut end_temp: Vec<char> = vec![];
                            
                            for expected in &end_chars {
                                if let Some(&n) = chars.peek() {
                                    if n == *expected {
                                        end_temp.push(chars.next().unwrap());
                                    } else {
                                        end_match = false;
                                        break;
                                    }
                                } else {
                                    end_match = false;
                                    break;
                                }
                            }
                            
                            if end_match {
                                found_end = true;
                                break;
                            } else {
                                content.extend(end_temp);
                            }
                        } else {
                            content.push(chars.next().unwrap());
                        }
                    }
                    
                    if found_end {
                        result.push_str(&content);
                        continue;
                    } else {
                        result.extend(temp);
                        result.push_str(&content);
                        continue;
                    }
                } else if matched {
                    // Single char delimiter, look for closing
                    let mut content = String::new();
                    let mut found_end = false;
                    
                    while let Some(&next) = chars.peek() {
                        if next == end_chars[0] {
                            chars.next(); // consume end delimiter
                            found_end = true;
                            break;
                        } else {
                            content.push(chars.next().unwrap());
                        }
                    }
                    
                    if found_end {
                        result.push_str(&content);
                        continue;
                    } else {
                        result.push(c);
                        result.push_str(&content);
                        continue;
                    }
                } else {
                    result.extend(temp);
                    continue;
                }
            }
            result.push(c);
        }
        
        result
    }

    /// Remove markdown links [text](url) -> text.
    fn remove_links(text: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            if chars[i] == '[' {
                // Find the closing bracket
                let mut j = i + 1;
                while j < chars.len() && chars[j] != ']' {
                    j += 1;
                }
                
                if j < chars.len() && chars[j] == ']' {
                    // Check if followed by (url)
                    if j + 1 < chars.len() && chars[j + 1] == '(' {
                        // Find closing paren
                        let mut k = j + 2;
                        while k < chars.len() && chars[k] != ')' {
                            k += 1;
                        }
                        
                        if k < chars.len() {
                            // Extract the link text
                            let link_text: String = chars[i + 1..j].iter().collect();
                            result.push_str(&link_text);
                            i = k + 1;
                            continue;
                        }
                    }
                }
            }
            result.push(chars[i]);
            i += 1;
        }
        
        result
    }

    /// Remove markdown images ![alt](url) -> empty.
    fn remove_images(text: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            // Check for image syntax ![
            if chars[i] == '!' && i + 1 < chars.len() && chars[i + 1] == '[' {
                // Find the closing bracket
                let mut j = i + 2;
                while j < chars.len() && chars[j] != ']' {
                    j += 1;
                }
                
                if j < chars.len() && chars[j] == ']' {
                    // Check if followed by (url)
                    if j + 1 < chars.len() && chars[j + 1] == '(' {
                        // Find closing paren
                        let mut k = j + 2;
                        while k < chars.len() && chars[k] != ')' {
                            k += 1;
                        }
                        
                        if k < chars.len() {
                            // Skip the entire image syntax
                            i = k + 1;
                            continue;
                        }
                    }
                }
            }
            result.push(chars[i]);
            i += 1;
        }
        
        result
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

    // ============================================
    // Phase 1.1: Summary/Excerpt Support Tests
    // ============================================

    #[test]
    fn test_summary_from_frontmatter() {
        let content = r#"
+++
title = "Test"
summary = "Custom summary from frontmatter"
+++
# Content here
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert_eq!(page.summary(), "Custom summary from frontmatter");
    }

    #[test]
    fn test_summary_from_more_marker() {
        let content = r#"
+++
title = "Test"
+++
This is the intro paragraph.

<!-- more -->

This is the rest of the content.
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert_eq!(page.summary(), "This is the intro paragraph.");
    }

    #[test]
    fn test_summary_from_first_paragraph() {
        let content = r#"
+++
title = "Test"
+++
This is the first paragraph.

This is the second paragraph.
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert_eq!(page.summary(), "This is the first paragraph.");
    }

    #[test]
    fn test_summary_frontmatter_takes_precedence() {
        let content = r#"
+++
title = "Test"
summary = "Frontmatter summary"
+++
First paragraph.

<!-- more -->

Rest of content.
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        // Frontmatter summary takes precedence
        assert_eq!(page.summary(), "Frontmatter summary");
    }

    #[test]
    fn test_summary_with_no_content() {
        let content = r#"
+++
title = "Test"
+++
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert_eq!(page.summary(), "");
    }

    #[test]
    fn test_summary_strips_markdown_formatting() {
        let content = r#"
+++
title = "Test"
+++
This has **bold** and *italic* text.
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        // Summary should strip markdown formatting
        let summary = page.summary();
        assert!(!summary.contains("**"));
        assert!(!summary.contains("*"));
    }

    // ============================================
    // Phase 1.2: Reading Time and Word Count Tests
    // ============================================

    #[test]
    fn test_word_count_simple() {
        let content = r#"
+++
title = "Test"
+++
This is a simple test with eight words.
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert_eq!(page.word_count(), 8);
    }

    #[test]
    fn test_word_count_empty() {
        let content = r#"
+++
title = "Test"
+++
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert_eq!(page.word_count(), 0);
    }

    #[test]
    fn test_word_count_with_markdown() {
        let content = r#"
+++
title = "Test"
+++
This has **bold** and *italic* and `code` text.
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        // After stripping markdown: "This has bold and italic and code text."
        // Word count should be 8
        assert_eq!(page.word_count(), 8);
    }

    #[test]
    fn test_word_count_with_links() {
        let content = r#"
+++
title = "Test"
+++
Check out [this link](https://example.com) for more info.
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        // After stripping: "Check out this link for more info."
        // Word count should be 7
        assert_eq!(page.word_count(), 7);
    }

    #[test]
    fn test_reading_time_one_minute() {
        let content = r#"
+++
title = "Test"
+++
This is a short article with just a few words.
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert_eq!(page.reading_time(), 1);
    }

    #[test]
    fn test_reading_time_multiple_minutes() {
        // Create content with ~400 words (should be 2 minutes)
        let words: Vec<&str> = (0..400).map(|_| "word").collect();
        let content = format!(
            "+++\ntitle = \"Test\"\n+++\n{}",
            words.join(" ")
        );
        let page = Page::from_str(&content, "test.md").unwrap();
        assert_eq!(page.reading_time(), 2);
    }

    #[test]
    fn test_reading_time_empty() {
        let content = r#"
+++
title = "Test"
+++
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert_eq!(page.reading_time(), 0);
    }

    // ============================================
    // Phase 1.3: Slug Customization Tests
    // ============================================

    #[test]
    fn test_slug_from_filename() {
        let content = r#"
+++
title = "Test"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "my-blog-post.md").unwrap();
        assert_eq!(page.slug(), "my-blog-post");
    }

    #[test]
    fn test_slug_from_frontmatter() {
        let content = r#"
+++
title = "Test"
slug = "custom-slug"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "my-blog-post.md").unwrap();
        // Frontmatter slug takes precedence over filename
        assert_eq!(page.slug(), "custom-slug");
    }

    #[test]
    fn test_slug_index_file() {
        let content = r#"
+++
title = "Test"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "_index.md").unwrap();
        assert_eq!(page.slug(), "_index");
    }

    #[test]
    fn test_url_path_regular_page() {
        let content = r#"
+++
title = "Test"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "about.md").unwrap();
        assert_eq!(page.url_path(), "/about/");
    }

    #[test]
    fn test_url_path_with_custom_slug() {
        let content = r#"
+++
title = "Test"
slug = "my-custom-url"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "original-filename.md").unwrap();
        assert_eq!(page.url_path(), "/my-custom-url/");
    }

    #[test]
    fn test_url_path_index_page() {
        let content = r#"
+++
title = "Home"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "_index.md").unwrap();
        // _index pages should have root path
        assert_eq!(page.url_path(), "/");
    }

    #[test]
    fn test_aliases_empty() {
        let content = r#"
+++
title = "Test"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert!(page.aliases().is_empty());
    }

    #[test]
    fn test_aliases_from_frontmatter() {
        let content = r#"
+++
title = "Test"
aliases = ["/old-url/", "/another-old-path/"]
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert_eq!(page.aliases(), &["/old-url/", "/another-old-path/"]);
    }

    #[test]
    fn test_slug_with_path_in_source() {
        let content = r#"
+++
title = "Test"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "blog/my-post.md").unwrap();
        // Should extract just the filename stem, not the full path
        assert_eq!(page.slug(), "my-post");
    }

    // ============================================
    // Phase 2.1: Taxonomies Tests
    // ============================================

    #[test]
    fn test_tags_empty() {
        let content = r#"
+++
title = "Test"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert!(page.tags().is_empty());
    }

    #[test]
    fn test_tags_from_frontmatter() {
        let content = r#"
+++
title = "Test"
tags = ["rust", "web", "tutorial"]
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert_eq!(page.tags(), &["rust", "web", "tutorial"]);
    }

    #[test]
    fn test_categories_empty() {
        let content = r#"
+++
title = "Test"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert!(page.categories().is_empty());
    }

    #[test]
    fn test_categories_from_frontmatter() {
        let content = r#"
+++
title = "Test"
categories = ["Programming", "Web Development"]
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert_eq!(page.categories(), &["Programming", "Web Development"]);
    }

    #[test]
    fn test_series_none() {
        let content = r#"
+++
title = "Test"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert!(page.series().is_none());
    }

    #[test]
    fn test_series_from_frontmatter() {
        let content = r#"
+++
title = "Test"
series = "Rust Web Development"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert_eq!(page.series(), Some("Rust Web Development"));
    }

    #[test]
    fn test_all_taxonomies_together() {
        let content = r#"
+++
title = "Complete Post"
tags = ["rust", "yew"]
categories = ["Tutorial"]
series = "Yew SSG Guide"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "test.md").unwrap();
        assert_eq!(page.tags(), &["rust", "yew"]);
        assert_eq!(page.categories(), &["Tutorial"]);
        assert_eq!(page.series(), Some("Yew SSG Guide"));
    }
}
