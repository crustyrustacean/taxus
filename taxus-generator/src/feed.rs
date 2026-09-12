// taxus-generator/src/feed.rs

//! Feed generation: RSS and Atom documents (emit phase).
//!
//! This module provides types for generating RSS and Atom feeds for blog content.
//! Feeds allow users to subscribe to site updates using feed readers. Which
//! pages a feed carries is decided upstream, by
//! `build::pipeline::feeds::feed_pages` over the Site Tree; this module
//! turns those pages into XML. See the book's
//! [Derivations](https://crustyrustacean.github.io/taxus/theory/derivations.html) chapter.
//!
//! # Overview
//!
//! - [`FeedGenerator`] - Main type for generating feeds from pre-built entries
//! - [`FeedEntry`] - A single entry in a feed (corresponds to a page)
//! - [`FeedConfig`] - Configuration for feed generation
//!
//! # Example
//!
//! ```no_run
//! use taxus_lib::feed::{FeedEntry, FeedGenerator, FeedConfig};
//! use taxus_lib::content::Frontmatter;
//!
//! let config = FeedConfig {
//!     title: "My Blog".to_string(),
//!     description: "My blog about things".to_string(),
//!     base_url: "https://example.com".to_string(),
//!     author: Some("Author Name".to_string()),
//!     ..Default::default()
//! };
//!
//! let entries = vec![FeedEntry::from_parts(
//!     &Frontmatter { title: "A post".into(), ..Default::default() },
//!     "A summary of the body.".into(),
//!     "https://example.com/a-post/".into(),
//!     None,
//! )];
//! let generator = FeedGenerator::new(config);
//!
//! // Generate RSS feed
//! let rss = generator.generate_rss_from_entries(entries.clone())?;
//!
//! // Generate Atom feed
//! let atom = generator.generate_atom_from_entries(entries)?;
//! # Ok::<(), taxus_lib::error::GeneratorError>(())
//! ```

mod atom;
mod rss;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::content::Frontmatter;
use crate::error::Result;

pub use atom::generate_atom_feed;
pub use rss::generate_rss_feed;

/// Configuration for feed generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedConfig {
    /// Feed title
    pub title: String,

    /// Feed description
    pub description: String,

    /// Base URL for the site (e.g., `<https://example.com>`)
    pub base_url: String,

    /// Feed author name
    pub author: Option<String>,

    /// Feed author email
    pub author_email: Option<String>,

    /// Language code (e.g., "en")
    #[serde(default = "default_language")]
    pub language: String,

    /// Maximum number of entries in the feed. `None` means no limit.
    #[serde(default = "default_limit")]
    pub limit: Option<usize>,

    /// Include full content in feed (vs just summary)
    #[serde(default)]
    pub full_content: bool,

    /// Feed output filename (without extension)
    #[serde(default = "default_filename")]
    pub filename: String,
}

fn default_language() -> String {
    "en".to_string()
}

fn default_limit() -> Option<usize> {
    None
}

fn default_filename() -> String {
    "feed".to_string()
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self {
            title: String::new(),
            description: String::new(),
            base_url: String::new(),
            author: None,
            author_email: None,
            language: default_language(),
            limit: default_limit(),
            full_content: false,
            filename: default_filename(),
        }
    }
}

/// A single entry in a feed.
#[derive(Debug, Clone)]
pub struct FeedEntry {
    /// Entry title
    pub title: String,

    /// Entry URL (absolute)
    pub url: String,

    /// Entry summary/description
    pub summary: String,

    /// Entry content (HTML)
    pub content: Option<String>,

    /// Publication date
    pub date: DateTime<Utc>,

    /// Last updated date
    pub updated: Option<DateTime<Utc>>,

    /// Author name
    pub author: Option<String>,

    /// Author email
    pub author_email: Option<String>,

    /// Tags/categories
    pub tags: Vec<String>,
}

impl FeedEntry {
    /// Create a feed entry from a document's schema, its body-derived
    /// summary, its served URL, and (optionally) its rendered content.
    ///
    /// `url` is the page's *effective* URL path (the served address, where
    /// a frontmatter `slug` override has already moved the page) joined to
    /// `base_url`. The caller owns the URL because the Site Tree is its
    /// single derivation point (`UrlPath::from_node_path`); deriving it
    /// here would be wrong for custom slugs (#19).
    ///
    /// `body_summary` is the caller's summary of the raw body (`<!-- more -->`
    /// split, first paragraph, markdown stripped). It is the fallback in
    /// the priority chain: `frontmatter.summary`, then
    /// `frontmatter.description`, then `body_summary` (#28).
    pub fn from_parts(
        meta: &Frontmatter,
        body_summary: String,
        url: String,
        content: Option<String>,
    ) -> Self {
        let summary = match (meta.summary.as_ref(), meta.description.as_ref()) {
            (Some(explicit), _) => explicit.clone(),
            (None, Some(description)) => description.clone(),
            (None, None) => body_summary,
        };

        // Convert date to DateTime<Utc>
        let date = meta
            .date
            .map(|d| {
                d.and_hms_opt(0, 0, 0)
                    .unwrap_or_else(|| chrono::NaiveDateTime::new(d, chrono::NaiveTime::MIN))
            })
            .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
            .unwrap_or_else(Utc::now);

        let updated = meta
            .updated
            .map(|d| {
                d.and_hms_opt(0, 0, 0)
                    .unwrap_or_else(|| chrono::NaiveDateTime::new(d, chrono::NaiveTime::MIN))
            })
            .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc));

        Self {
            title: meta.title.clone(),
            url,
            summary,
            content,
            date,
            updated,
            author: None, // Could be extended to use frontmatter.author
            author_email: None,
            tags: meta.tags.clone(),
        }
    }
}

/// Feed generator for RSS and Atom formats.
#[derive(Debug, Clone)]
pub struct FeedGenerator {
    config: FeedConfig,
}

impl FeedGenerator {
    /// Create a new feed generator with the given configuration.
    pub fn new(config: FeedConfig) -> Self {
        Self { config }
    }

    /// Get the feed configuration.
    pub fn config(&self) -> &FeedConfig {
        &self.config
    }

    /// Generate an RSS 2.0 feed from pre-built entries.
    ///
    /// This is the form the build pipeline uses: it owns each page's
    /// effective (served) URL path, so no URL is re-derived here.
    pub fn generate_rss_from_entries(&self, entries: Vec<FeedEntry>) -> Result<String> {
        let entries = self.apply_limit(entries);
        generate_rss_feed(&entries, &self.config)
    }

    /// Generate an Atom feed from pre-built entries; see
    /// [`FeedGenerator::generate_rss_from_entries`].
    pub fn generate_atom_from_entries(&self, entries: Vec<FeedEntry>) -> Result<String> {
        let entries = self.apply_limit(entries);
        generate_atom_feed(&entries, &self.config)
    }

    /// Sort newest-first and apply the configured entry limit.
    fn apply_limit(&self, mut entries: Vec<FeedEntry>) -> Vec<FeedEntry> {
        // Sort by date, newest first
        entries.sort_by_key(|b| std::cmp::Reverse(b.date));

        // Limit the number of entries
        if let Some(limit) = self.config.limit {
            entries.truncate(limit);
        }

        entries
    }

    /// Get the RSS feed filename.
    pub fn rss_filename(&self) -> String {
        format!("{}.xml", self.config.filename)
    }

    /// Get the Atom feed filename.
    pub fn atom_filename(&self) -> String {
        format!("{}.atom", self.config.filename)
    }
}

/// Escape special XML characters.
pub fn escape_xml(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("\u{26}amp;"),
            '<' => result.push_str("\u{26}lt;"),
            '>' => result.push_str("\u{26}gt;"),
            '"' => result.push_str("\u{26}quot;"),
            '\'' => result.push_str("\u{26}apos;"),
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(title: &str, date_str: &str) -> FeedEntry {
        let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok();
        let meta = Frontmatter {
            title: title.to_string(),
            date,
            ..Default::default()
        };
        FeedEntry::from_parts(
            &meta,
            format!("Summary of {}", title),
            format!(
                "https://example.com/{}/",
                title.to_lowercase().replace(' ', "-")
            ),
            Some(format!("<p>Content for {}</p>", title)),
        )
    }

    fn meta_of(title: &str) -> Frontmatter {
        Frontmatter {
            title: title.to_string(),
            ..Default::default()
        }
    }

    fn body_summary(raw: &str) -> String {
        // The caller-side projection the pipeline computes from the raw
        // body: first paragraph, markdown stripped (Page::summary logic).
        crate::content::Page {
            frontmatter: Frontmatter::default(),
            raw_content: raw.to_string(),
        }
        .summary()
    }

    #[test]
    fn test_feed_entry_from_parts() {
        let entry = create_test_entry("Test Post", "2024-01-15");
        assert_eq!(entry.title, "Test Post");
        assert_eq!(entry.url, "https://example.com/test-post/");
        assert!(!entry.summary.is_empty());
    }

    #[test]
    fn test_feed_entry_from_parts_uses_the_url_it_is_given() {
        // #19: the served URL is the caller's, verbatim — a custom slug
        // moves the page and the entry must follow it.
        let meta = Frontmatter {
            title: "Renamed Post".into(),
            slug: Some("custom-slug".into()),
            ..Default::default()
        };
        let entry = FeedEntry::from_parts(
            &meta,
            "Summary.".into(),
            "https://example.com/custom-slug/".into(),
            None,
        );
        assert_eq!(entry.url, "https://example.com/custom-slug/");
    }

    #[test]
    fn test_feed_entry_summary_strips_markdown() {
        // #28: feed summaries must not leak raw markdown into <description>
        let summary = body_summary(
            "# A Heading

Some *emphasized* prose here.",
        );
        let entry = FeedEntry::from_parts(
            &meta_of("Md Post"),
            summary,
            "https://example.com/x/".into(),
            None,
        );

        assert!(
            !entry.summary.contains('#'),
            "summary must not contain raw heading syntax: {}",
            entry.summary
        );
        assert!(
            !entry.summary.contains('*'),
            "summary must not contain raw emphasis syntax: {}",
            entry.summary
        );
    }

    #[test]
    fn test_feed_entry_summary_respects_more_marker() {
        let summary = body_summary(
            "Intro text only.

<!-- more -->

Rest of the article.",
        );
        let entry = FeedEntry::from_parts(
            &meta_of("More Post"),
            summary,
            "https://example.com/x/".into(),
            None,
        );
        assert_eq!(entry.summary, "Intro text only.");
    }

    #[test]
    fn test_feed_entry_summary_prefers_description() {
        // description is user-authored prose for exactly this purpose;
        // it should outrank content-derived summaries
        let meta = Frontmatter {
            title: "Desc Post".into(),
            description: Some("An authored description.".into()),
            ..Default::default()
        };
        let entry = FeedEntry::from_parts(
            &meta,
            "The body summary.".into(),
            "https://example.com/x/".into(),
            None,
        );
        assert_eq!(entry.summary, "An authored description.");
    }

    #[test]
    fn test_feed_entry_summary_prefers_explicit_summary_over_description() {
        // frontmatter.summary is the most specific signal; description
        // is the fallback among authored fields
        let meta = Frontmatter {
            title: "Both Post".into(),
            description: Some("The description.".into()),
            summary: Some("The summary.".into()),
            ..Default::default()
        };
        let entry = FeedEntry::from_parts(
            &meta,
            "The body summary.".into(),
            "https://example.com/x/".into(),
            None,
        );
        assert_eq!(entry.summary, "The summary.");
    }

    #[test]
    fn test_feed_generator_rss() {
        let config = FeedConfig {
            title: "Test Blog".to_string(),
            description: "A test blog".to_string(),
            base_url: "https://example.com".to_string(),
            ..Default::default()
        };

        let generator = FeedGenerator::new(config);
        let entries = vec![
            create_test_entry("First Post", "2024-01-01"),
            create_test_entry("Second Post", "2024-01-15"),
        ];

        let rss = generator.generate_rss_from_entries(entries).unwrap();
        assert!(rss.contains("<?xml"));
        assert!(rss.contains("<rss"));
        assert!(rss.contains("Test Blog"));
    }

    #[test]
    fn test_feed_generator_atom() {
        let config = FeedConfig {
            title: "Test Blog".to_string(),
            description: "A test blog".to_string(),
            base_url: "https://example.com".to_string(),
            ..Default::default()
        };

        let generator = FeedGenerator::new(config);
        let entries = vec![
            create_test_entry("First Post", "2024-01-01"),
            create_test_entry("Second Post", "2024-01-15"),
        ];

        let atom = generator.generate_atom_from_entries(entries).unwrap();
        assert!(atom.contains("<?xml"));
        assert!(atom.contains("<feed"));
        assert!(atom.contains("Test Blog"));
    }

    #[test]
    fn test_feed_limit() {
        let config = FeedConfig {
            title: "Test Blog".to_string(),
            description: "A test blog".to_string(),
            base_url: "https://example.com".to_string(),
            limit: Some(2),
            ..Default::default()
        };

        let generator = FeedGenerator::new(config);
        let entries = vec![
            create_test_entry("First Post", "2024-01-01"),
            create_test_entry("Second Post", "2024-01-15"),
            create_test_entry("Third Post", "2024-02-01"),
        ];

        let rss = generator.generate_rss_from_entries(entries).unwrap();
        assert_eq!(rss.matches("<item>").count(), 2);
        // Should be sorted by date, newest first
        assert!(rss.contains("Third Post"));
        assert!(rss.contains("Second Post"));
    }

    #[test]
    fn test_no_limit_keeps_all_entries() {
        // #58: unset limit means no limit, not a silent 20-entry cap
        let config = FeedConfig {
            base_url: "https://example.com".to_string(),
            ..Default::default()
        };

        let generator = FeedGenerator::new(config);
        let entries: Vec<FeedEntry> = (0..30)
            .map(|i| {
                create_test_entry(
                    &format!("Post {i}"),
                    &format!("2024-01-{:02}", (i % 28) + 1),
                )
            })
            .collect();

        let rss = generator.generate_rss_from_entries(entries).unwrap();
        assert_eq!(rss.matches("<item>").count(), 30);
    }
}
