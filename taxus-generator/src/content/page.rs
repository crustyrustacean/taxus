//! Page type for individual content files.

use crate::error::{ContentError, Result};
use chrono::NaiveDate;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use super::Frontmatter;

/// Split a leading `YYYY-MM-DD-` date prefix from a filename stem.
///
/// This interprets a storage convention into model data (#67): the date
/// prefix is stripped from the stem so it never leaks into slugs or URLs,
/// and the parsed date is returned so it can be used as default metadata.
///
/// The prefix is only stripped when it forms a valid calendar date *and*
/// something remains after it. A stem that is exactly a date (e.g.
/// `2026-04-06`), or that merely looks like one (e.g. `2026-13-45-post`),
/// is returned unchanged with `None`.
///
/// # Examples
///
/// ```
/// # use chrono::NaiveDate;
/// # use taxus_lib::content::split_date_prefix;
/// let (stem, date) = split_date_prefix("2026-04-06-my-post");
/// assert_eq!(stem, "my-post");
/// assert_eq!(date, NaiveDate::from_ymd_opt(2026, 4, 6));
///
/// // No (valid) prefix: unchanged.
/// assert_eq!(split_date_prefix("my-post").0, "my-post");
/// assert_eq!(split_date_prefix("2026-04-06").0, "2026-04-06");
/// ```
pub fn split_date_prefix(stem: &str) -> (&str, Option<NaiveDate>) {
    // Shape check: exactly `XXXX-XX-XX-` (11 bytes, digits in position).
    let b = stem.as_bytes();
    if b.len() <= 11 || b[4] != b'-' || b[7] != b'-' || b[10] != b'-' {
        return (stem, None);
    }
    let all_digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    if !all_digits(0..4) || !all_digits(5..7) || !all_digits(8..10) {
        return (stem, None);
    }

    // Parsing cannot fail: the digit shapes were just verified.
    let year: i32 = stem[0..4].parse().expect("verified digits");
    let month: u32 = stem[5..7].parse().expect("verified digits");
    let day: u32 = stem[8..10].parse().expect("verified digits");

    match NaiveDate::from_ymd_opt(year, month, day) {
        // `b.len() > 11` guarantees a non-empty remainder and that index 11
        // is a char boundary (bytes 0..=10 are ASCII).
        Some(date) if b.len() > 11 => (&stem[11..], Some(date)),
        _ => (stem, None),
    }
}

/// A single page with frontmatter and Markdown content.
#[derive(Debug, Clone)]
pub struct Page {
    /// Page metadata from frontmatter
    pub frontmatter: Frontmatter,

    /// URL path (e.g., "/about/")
    pub path: String,

    /// Source file path. [`from_file_in`](Self::from_file_in) stores it
    /// relative to the content directory (e.g. `blog/post-1.md`);
    /// [`from_file`](Self::from_file) stores the path it was given.
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
    /// `source` records the path exactly as given, so pass a path relative
    /// to the content directory when you have one — or use
    /// [`from_file_in`](Self::from_file_in), which does that for you.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use taxus_lib::content::Page;
    ///
    /// let page = Page::from_file("content/about.md")?;
    /// println!("Title: {}", page.frontmatter.title);
    /// println!("Path: {}", page.path);
    /// # Ok::<(), taxus_lib::error::GeneratorError>(())
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| ContentError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        let source = path
            .to_str()
            .ok_or_else(|| ContentError::InvalidPath(path.display().to_string()))?;

        Self::from_str(&content, source)
    }

    /// Parse the page at `relative` inside `content_dir`.
    ///
    /// The file is read from `content_dir.join(relative)` and `source` is
    /// `relative` (e.g. `blog/post-1.md`), the same identity the Site Tree
    /// and the route registry use for the file — so error messages name
    /// the file the way the author knows it, and a page can be joined back
    /// to its tree node by `source`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use taxus_lib::content::Page;
    ///
    /// let page = Page::from_file_in("content", "blog/post-1.md")?;
    /// assert_eq!(page.source.to_str(), Some("blog/post-1.md"));
    /// # Ok::<(), taxus_lib::error::GeneratorError>(())
    /// ```
    pub fn from_file_in<C: AsRef<Path>, R: AsRef<Path>>(
        content_dir: C,
        relative: R,
    ) -> Result<Self> {
        let relative = relative.as_ref();
        let full_path = content_dir.as_ref().join(relative);
        let content = std::fs::read_to_string(&full_path).map_err(|e| ContentError::Io {
            path: full_path.clone(),
            source: e,
        })?;

        let source = relative
            .to_str()
            .ok_or_else(|| ContentError::InvalidPath(relative.display().to_string()))?;

        Self::from_str(&content, source)
    }

    /// Parse a page from a string with an explicit source path.
    ///
    /// `source` is stored verbatim; only its file stem is used to derive
    /// the URL path, the slug and the `YYYY-MM-DD-` default date.
    ///
    /// # Example
    ///
    /// ```
    /// use taxus_lib::content::Page;
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
    /// # Ok::<(), taxus_lib::error::GeneratorError>(())
    /// ```
    pub fn from_str(content: &str, source: &str) -> Result<Self> {
        let (mut frontmatter, raw_content) = Self::parse_frontmatter(content, source)?;

        // A `YYYY-MM-DD-` filename prefix supplies the default publication
        // date when frontmatter does not set one (#67). Frontmatter wins.
        if frontmatter.date.is_none() {
            let stem = Path::new(source)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if let Some(date) = split_date_prefix(stem).1 {
                frontmatter.date = Some(date);
            }
        }

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
        let stem = split_date_prefix(stem).0;

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
        let first_paragraph = self.raw_content.split("\n\n").next().unwrap_or("").trim();

        Self::strip_markdown(first_paragraph)
    }

    /// Calculate word count from the raw content.
    ///
    /// Strips markdown formatting and counts words (whitespace-separated tokens).
    pub fn word_count(&self) -> usize {
        let stripped = Self::strip_markdown(&self.raw_content);
        stripped.split_whitespace().count()
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
        words.div_ceil(WORDS_PER_MINUTE) // Ceiling division
    }

    /// Get the effective slug for this page.
    ///
    /// Returns the custom slug from frontmatter if set, otherwise derives
    /// from the source filename, stripping a `YYYY-MM-DD-` date prefix if
    /// present (#67) so that dates never leak into slugs or URLs.
    pub fn slug(&self) -> &str {
        if let Some(ref slug) = self.frontmatter.slug {
            slug
        } else {
            // Derive from source filename
            let stem = Path::new(&self.source)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("index");
            split_date_prefix(stem).0
        }
    }

    /// The page's slug as a root-level URL path (`/my-post/`; `/` for an
    /// `_index.md`).
    ///
    /// This is *not* the URL the page is served at: that is section path +
    /// slug, derived from the Site Tree and exposed as
    /// `ProcessedPage::effective_url_path()`. This helper only knows the
    /// page, not where it lives.
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
    use crate::error::GeneratorError;

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
            GeneratorError::Content(inner) if matches!(*inner, ContentError::UnclosedFrontmatter(_))
        ));
    }

    #[test]
    fn test_page_from_str_with_path() {
        let content = "+++\ntitle = \"About\"\n+++\nAbout page";
        let page = Page::from_str(content, "about.md").unwrap();

        assert_eq!(page.path, "/about/");
        assert_eq!(page.source, PathBuf::from("about.md"));
    }

    #[test]
    fn test_page_from_str_keeps_directory_in_source() {
        let content = "+++\ntitle = \"Post\"\n+++\nBody";
        let page = Page::from_str(content, "blog/2026-04-06-post-1.md").unwrap();

        // Storage keeps the directory (#23); identity is still the stem.
        assert_eq!(page.source, PathBuf::from("blog/2026-04-06-post-1.md"));
        assert_eq!(page.path, "/post-1/");
        assert_eq!(page.slug(), "post-1");
        assert_eq!(
            page.frontmatter.date,
            chrono::NaiveDate::from_ymd_opt(2026, 4, 6)
        );
    }

    #[test]
    fn test_from_file_in_records_relative_path() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("blog")).unwrap();
        std::fs::write(
            dir.path().join("blog/post.md"),
            "+++\ntitle = \"Post\"\n+++\nBody",
        )
        .unwrap();

        let page = Page::from_file_in(dir.path(), "blog/post.md").unwrap();
        assert_eq!(page.source, PathBuf::from("blog/post.md"));
        assert_eq!(page.path, "/post/");

        // from_file keeps whatever path it was given.
        let page = Page::from_file(dir.path().join("blog/post.md")).unwrap();
        assert_eq!(page.source, dir.path().join("blog/post.md"));
    }

    #[test]
    fn test_from_file_in_error_names_relative_path() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("blog")).unwrap();
        std::fs::write(dir.path().join("blog/bad.md"), "+++\ntitle = \n+++\n").unwrap();

        let err = Page::from_file_in(dir.path(), "blog/bad.md")
            .unwrap_err()
            .to_string();
        assert!(err.contains("Invalid frontmatter in blog/bad.md"), "{err}");
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
        let content = format!("+++\ntitle = \"Test\"\n+++\n{}", words.join(" "));
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

    // ============================================
    // Date-Prefix Stripping Tests (#67)
    // ============================================

    #[test]
    fn test_slug_strips_date_prefix() {
        let content = r#"
+++
title = "Test"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "2026-04-06-my-blog-post.md").unwrap();
        assert_eq!(page.slug(), "my-blog-post");
        assert_eq!(page.path, "/my-blog-post/");
        assert_eq!(page.url_path(), "/my-blog-post/");
    }

    #[test]
    fn test_slug_pure_date_filename_not_stripped() {
        // A stem that is only a date has nothing left after stripping;
        // the whole stem stays the slug.
        let content = r#"
+++
title = "Test"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "2026-04-06.md").unwrap();
        assert_eq!(page.slug(), "2026-04-06");
        assert_eq!(page.path, "/2026-04-06/");
    }

    #[test]
    fn test_slug_invalid_date_prefix_not_stripped() {
        let content = r#"
+++
title = "Test"
+++
Content
"#;
        // Month 13 is not a valid date: no strip, no default date.
        let page = Page::from_str(content.trim_start(), "2026-13-45-my-post.md").unwrap();
        assert_eq!(page.slug(), "2026-13-45-my-post");
        assert_eq!(page.frontmatter.date, None);

        // Wrong digit shapes are not dates either.
        let page = Page::from_str(content.trim_start(), "26-4-6-my-post.md").unwrap();
        assert_eq!(page.slug(), "26-4-6-my-post");
    }

    #[test]
    fn test_frontmatter_slug_wins_over_date_prefix() {
        let content = r#"
+++
title = "Test"
slug = "custom-slug"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "2026-04-06-my-post.md").unwrap();
        assert_eq!(page.slug(), "custom-slug");
    }

    #[test]
    fn test_date_defaults_from_filename_prefix() {
        let content = r#"
+++
title = "Test"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "2026-04-06-my-post.md").unwrap();
        assert_eq!(
            page.frontmatter.date,
            chrono::NaiveDate::from_ymd_opt(2026, 4, 6)
        );
    }

    #[test]
    fn test_frontmatter_date_beats_filename_prefix() {
        let content = r#"
+++
title = "Test"
date = 2020-01-01
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "2026-04-06-my-post.md").unwrap();
        assert_eq!(
            page.frontmatter.date,
            chrono::NaiveDate::from_ymd_opt(2020, 1, 1)
        );
    }

    #[test]
    fn test_no_date_without_filename_prefix() {
        let content = r#"
+++
title = "Test"
+++
Content
"#;
        let page = Page::from_str(content.trim_start(), "my-post.md").unwrap();
        assert_eq!(page.frontmatter.date, None);
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
