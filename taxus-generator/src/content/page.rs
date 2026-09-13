//! Page type for individual content files (parse phase).
//!
//! A [`Page`] is the parsed form of one content file: frontmatter, body,
//! and the content-relative `source` path that joins it to its tree node.
//! See the book's [Identity](https://crustyrustacean.github.io/taxus/theory/identity.html) chapter.

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

    /// Raw Markdown content (without frontmatter)
    pub raw_content: String,
}

impl Page {
    /// Parse a page from a Markdown file.
    ///
    /// The file should contain TOML frontmatter between `+++` markers.
    /// `source` records the path exactly as given, so pass a path relative
    /// to the content directory when you have one.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use taxus_lib::content::Page;
    ///
    /// let page = Page::from_file("content/about.md")?;
    /// println!("Title: {}", page.frontmatter.title);
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

    /// Parse a page from a string with an explicit source path.
    ///
    /// `source` is used only for the `YYYY-MM-DD-` default date and in
    /// error messages; the slug and URL live in the Site Tree, not here.
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

        Ok(Self {
            frontmatter,
            raw_content,
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
    /// The plain text of a Markdown document (#10).
    ///
    /// Every construct the old hand-rolled stripper missed — tables,
    /// footnotes, task lists, strikethrough, nested emphasis, HTML — is
    /// handled structurally by walking the parsed document's `Text`
    /// events (the same walk the search index uses). Link destinations,
    /// image URLs, markup, and code fences contribute nothing; the words
    /// a reader reads are everything that remains.
    fn strip_markdown(text: &str) -> String {
        crate::build::pipeline::markdown::markdown_text(text)
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
    // ------------------------------------------------------------------
    // #10: constructs the old hand-rolled stripper missed
    // ------------------------------------------------------------------

    #[test]
    fn test_summary_handles_issue10_constructs() {
        let page = Page::from_str(
            "+++
title = \"T\"
+++
> A quoted lead with ~~strikethrough~~ text.
",
            "test.md",
        )
        .unwrap();
        let summary = page.summary();
        // Event-walk joining may leave doubled spaces where delimiters
        // stood; the words must all survive, the markers must not.
        let collapsed: String = summary.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            collapsed, "A quoted lead with strikethrough text.",
            "blockquote/strikethrough text should survive cleanly, got: {summary}"
        );
        assert!(!summary.contains('>'), "marker characters must not survive");
        assert!(
            !summary.contains("~~"),
            "strikethrough markers must not survive"
        );
    }

    #[test]
    fn test_word_count_counts_list_items_not_markers() {
        let page = Page::from_str(
            "+++
title = \"T\"
+++
- alpha
- beta
1. gamma
2. delta
",
            "test.md",
        )
        .unwrap();
        assert_eq!(page.word_count(), 4, "list words only, no markers/bullets");
    }

    #[test]
    fn test_word_count_ignores_table_markup() {
        let page = Page::from_str(
            "+++
title = \"T\"
+++
| h1 | h2 |
| --- | --- |
| a | b |
",
            "test.md",
        )
        .unwrap();
        // Cells only: h1 h2 a b — pipes and the delimiter row count for nothing.
        assert_eq!(page.word_count(), 4, "got {}", page.word_count());
    }

    #[test]
    fn test_word_count_ignores_horizontal_rules() {
        let page = Page::from_str(
            "+++
title = \"T\"
+++
before

---

after
",
            "test.md",
        )
        .unwrap();
        assert_eq!(page.word_count(), 2);
    }

    #[test]
    fn test_word_count_ignores_html_tags_and_task_markers() {
        // Inline HTML is opaque to the parser (it emits one Html event
        // whose interior text is invisible by CommonMark semantics), and
        // the checkbox marker in a task list contributes no Text event.
        let page = Page::from_str(
            "+++
title = \"T\"
+++
<div class=\"x\">real words</div>

- [x] done thing
",
            "test.md",
        )
        .unwrap();
        assert_eq!(page.word_count(), 2, "got {}", page.word_count());

        // Plain inline tags around real markdown text:
        let page = Page::from_str(
            "+++
title = \"T\"
+++
Visit <br> the <b>docs</b> page.
",
            "test.md",
        )
        .unwrap();
        assert_eq!(page.word_count(), 4, "got {}", page.word_count());
    }

    #[test]
    fn test_summary_footnote_reference_not_leaked() {
        let page = Page::from_str(
            "+++
title = \"T\"
+++
A claim[^1] with a note.

[^1]: the source.
",
            "test.md",
        )
        .unwrap();
        let summary = page.summary();
        assert!(
            !summary.contains("[^"),
            "footnote markers must not leak into summaries, got: {summary}"
        );
    }
}
