// taxus-generator/src/feed.rs

//! Feed generation module.
//!
//! This module provides types for generating RSS and Atom feeds for blog content.
//! Feeds allow users to subscribe to site updates using feed readers.
//!
//! # Overview
//!
//! - [`FeedGenerator`] - Main type for generating feeds from pages
//! - [`FeedEntry`] - A single entry in a feed (corresponds to a page)
//! - [`FeedConfig`] - Configuration for feed generation
//!
//! # Example
//!
//! ```no_run
//! use taxus_lib::feed::{FeedGenerator, FeedConfig};
//! use taxus_lib::content::Page;
//!
//! let config = FeedConfig {
//!     title: "My Blog".to_string(),
//!     description: "My blog about things".to_string(),
//!     base_url: "https://example.com".to_string(),
//!     author: Some("Author Name".to_string()),
//!     ..Default::default()
//! };
//!
//! let pages: Vec<Page> = vec![]; // Your pages here
//! let generator = FeedGenerator::new(config);
//!
//! // Generate RSS feed
//! let rss = generator.generate_rss(&pages)?;
//!
//! // Generate Atom feed
//! let atom = generator.generate_atom(&pages)?;
//! # Ok::<(), taxus_lib::error::GeneratorError>(())
//! ```

mod atom;
mod rss;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::content::Page;
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

    /// Maximum number of entries in the feed
    #[serde(default = "default_limit")]
    pub limit: usize,

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

fn default_limit() -> usize {
    20
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
    /// Create a feed entry from a page.
    pub fn from_page(page: &Page, base_url: &str) -> Self {
        use crate::templates::compute_permalink;
        let url = compute_permalink(base_url, &page.path);

        // Use summary if available, otherwise generate from content
        let summary = page.frontmatter.summary.clone().unwrap_or_else(|| {
            // Generate a summary from the first 200 characters of content
            let text = page.raw_content.chars().take(200).collect::<String>();
            if text.len() < page.raw_content.len() {
                format!("{}...", text)
            } else {
                text
            }
        });

        // Convert date to DateTime<Utc>
        let date = page
            .frontmatter
            .date
            .map(|d| {
                d.and_hms_opt(0, 0, 0)
                    .unwrap_or_else(|| chrono::NaiveDateTime::new(d, chrono::NaiveTime::MIN))
            })
            .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
            .unwrap_or_else(Utc::now);

        let updated = page
            .frontmatter
            .updated
            .map(|d| {
                d.and_hms_opt(0, 0, 0)
                    .unwrap_or_else(|| chrono::NaiveDateTime::new(d, chrono::NaiveTime::MIN))
            })
            .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc));

        Self {
            title: page.frontmatter.title.clone(),
            url,
            summary,
            content: page.content.clone(),
            date,
            updated,
            author: None, // Could be extended to use page.frontmatter.author
            author_email: None,
            tags: page.frontmatter.tags.clone(),
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

    /// Generate an RSS 2.0 feed from pages.
    pub fn generate_rss(&self, pages: &[Page]) -> Result<String> {
        let entries = self.pages_to_entries(pages);
        generate_rss_feed(&entries, &self.config)
    }

    /// Generate an Atom feed from pages.
    pub fn generate_atom(&self, pages: &[Page]) -> Result<String> {
        let entries = self.pages_to_entries(pages);
        generate_atom_feed(&entries, &self.config)
    }

    /// Convert pages to feed entries.
    fn pages_to_entries(&self, pages: &[Page]) -> Vec<FeedEntry> {
        let mut entries: Vec<FeedEntry> = pages
            .iter()
            .filter(|p| !p.frontmatter.draft) // Exclude drafts
            .map(|p| FeedEntry::from_page(p, &self.config.base_url))
            .collect();

        // Sort by date, newest first
        entries.sort_by_key(|b| std::cmp::Reverse(b.date));

        // Limit the number of entries
        entries.truncate(self.config.limit);

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
    use crate::content::Frontmatter;
    use std::path::PathBuf;

    fn create_test_page(title: &str, date_str: &str) -> Page {
        let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok();
        Page {
            frontmatter: Frontmatter {
                title: title.to_string(),
                date,
                ..Default::default()
            },
            path: format!("/{}/", title.to_lowercase().replace(' ', "-")),
            source: PathBuf::from(format!("{}.md", title)),
            raw_content: format!("Content for {}", title),
            content: Some(format!("<p>Content for {}</p>", title)),
        }
    }

    #[test]
    fn test_feed_entry_from_page() {
        let page = create_test_page("Test Post", "2024-01-15");
        let entry = FeedEntry::from_page(&page, "https://example.com");

        assert_eq!(entry.title, "Test Post");
        assert_eq!(entry.url, "https://example.com/test-post/");
        assert!(!entry.summary.is_empty());
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
        let pages = vec![
            create_test_page("First Post", "2024-01-01"),
            create_test_page("Second Post", "2024-01-15"),
        ];

        let rss = generator.generate_rss(&pages).unwrap();
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
        let pages = vec![
            create_test_page("First Post", "2024-01-01"),
            create_test_page("Second Post", "2024-01-15"),
        ];

        let atom = generator.generate_atom(&pages).unwrap();
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
            limit: 2,
            ..Default::default()
        };

        let generator = FeedGenerator::new(config);
        let pages = vec![
            create_test_page("First Post", "2024-01-01"),
            create_test_page("Second Post", "2024-01-15"),
            create_test_page("Third Post", "2024-02-01"),
        ];

        let entries = generator.pages_to_entries(&pages);
        assert_eq!(entries.len(), 2);
        // Should be sorted by date, newest first
        assert_eq!(entries[0].title, "Third Post");
        assert_eq!(entries[1].title, "Second Post");
    }
}
