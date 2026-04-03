//! Frontmatter parsing for content files.
//!
//! Frontmatter is TOML metadata at the beginning of a Markdown file,
//! delimited by `+++` markers.

use std::str::FromStr;

use chrono::NaiveDate;
use serde::Deserialize;

/// Page frontmatter metadata parsed from TOML between `+++` markers.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Frontmatter {
    /// Page title (required for pages, optional for sections)
    #[serde(default)]
    pub title: String,

    /// Optional page description
    pub description: Option<String>,

    /// Optional publication date (TOML datetime)
    #[serde(default, with = "optional_date")]
    pub date: Option<NaiveDate>,

    /// Optional template override
    pub template: Option<String>,

    /// Draft status (drafts are not built in release mode)
    #[serde(default)]
    pub draft: bool,

    /// Manual summary/excerpt override
    pub summary: Option<String>,

    /// Custom slug for URL path (overrides filename-based path)
    pub slug: Option<String>,

    /// Alternative URLs that should redirect to this page
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Tags for the page (taxonomy)
    #[serde(default)]
    pub tags: Vec<String>,

    /// Categories for the page (taxonomy)
    #[serde(default)]
    pub categories: Vec<String>,

    /// Series name for multi-part content
    pub series: Option<String>,

    /// Custom extra metadata
    pub extra: Option<toml::Value>,

    // ========================================
    // Phase 3: Pagination Fields
    // ========================================
    /// Sort order for section pages
    #[serde(default)]
    pub sort_by: SortBy,

    /// Number of items per page (0 = no pagination)
    #[serde(default)]
    pub paginate_by: usize,

    /// Template for paginated pages
    pub paginate_template: Option<String>,

    /// Weight for manual ordering (lower = first)
    #[serde(default)]
    pub weight: i32,

    /// Last updated date
    #[serde(default, with = "optional_date")]
    pub updated: Option<NaiveDate>,
}

/// Sort order for pages within a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
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

/// Custom serialization module for optional NaiveDate with TOML datetime support.
mod optional_date {
    use chrono::NaiveDate;
    use serde::{Deserialize, Deserializer, Serializer};

    #[allow(dead_code)]
    pub fn serialize<S>(date: &Option<NaiveDate>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(d) => {
                let s = d.format("%Y-%m-%d").to_string();
                serializer.serialize_str(&s)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<toml::value::Datetime>::deserialize(deserializer)?;
        Ok(opt.and_then(|dt| {
            dt.date.and_then(|d| {
                NaiveDate::from_ymd_opt(i32::from(d.year), u32::from(d.month), u32::from(d.day))
            })
        }))
    }
}

impl FromStr for Frontmatter {
    type Err = toml::de::Error;

    /// Parse frontmatter from a TOML string.
    ///
    /// # Example
    ///
    /// ```
    /// use std::str::FromStr;
    /// use taxus_lib::content::Frontmatter;
    ///
    /// let fm = Frontmatter::from_str(r#"
    /// title = "My Page"
    /// description = "A description"
    /// "#)?;
    ///
    /// assert_eq!(fm.title, "My Page");
    /// # Ok::<(), toml::de::Error>(())
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s)
    }
}

impl Frontmatter {
    /// Get the template name, defaulting to "page.html".
    pub fn template(&self) -> &str {
        self.template.as_deref().unwrap_or("page.html")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_frontmatter() {
        let fm = Frontmatter::from_str(r#"title = "Test""#).unwrap();
        assert_eq!(fm.title, "Test");
        assert!(fm.description.is_none());
        assert!(fm.date.is_none());
        assert!(!fm.draft);
    }

    #[test]
    fn test_parse_full_frontmatter() {
        let fm = Frontmatter::from_str(
            r#"
title = "Test Page"
description = "A test page"
date = 2024-01-15
template = "custom.html"
draft = true
"#,
        )
        .unwrap();

        assert_eq!(fm.title, "Test Page");
        assert_eq!(fm.description, Some("A test page".to_string()));
        assert_eq!(fm.date, Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()));
        assert_eq!(fm.template, Some("custom.html".to_string()));
        assert!(fm.draft);
    }

    #[test]
    fn test_parse_frontmatter_with_extra() {
        let fm = Frontmatter::from_str(
            r#"
title = "Test"
[extra]
author = "John Doe"
tags = ["rust", "web"]
"#,
        )
        .unwrap();

        assert_eq!(fm.title, "Test");
        assert!(fm.extra.is_some());
    }

    #[test]
    fn test_default_template() {
        let fm = Frontmatter::default();
        assert_eq!(fm.template(), "page.html");
    }

    #[test]
    fn test_custom_template() {
        let fm = Frontmatter::from_str(r#"title = "Test""#).unwrap();
        assert_eq!(fm.template(), "page.html");

        let fm = Frontmatter::from_str(
            r#"
title = "Test"
template = "custom.html"
"#,
        )
        .unwrap();
        assert_eq!(fm.template(), "custom.html");
    }

    #[test]
    fn test_draft_defaults_to_false() {
        let fm = Frontmatter::from_str(r#"title = "Test""#).unwrap();
        assert!(!fm.draft);
    }

    #[test]
    fn test_parse_invalid_toml() {
        let result = Frontmatter::from_str("invalid[");
        assert!(result.is_err());
    }

    #[test]
    fn test_default_frontmatter() {
        let fm = Frontmatter::default();
        assert!(fm.title.is_empty());
        assert!(fm.description.is_none());
        assert!(fm.date.is_none());
        assert!(fm.template.is_none());
        assert!(!fm.draft);
        assert!(fm.extra.is_none());
    }

    // ============================================
    // Phase 1.1: Summary/Excerpt Support Tests
    // ============================================

    #[test]
    fn test_parse_frontmatter_with_summary() {
        let fm = Frontmatter::from_str(
            r#"
            title = "Test Page"
            summary = "This is a custom summary for the page."
            "#,
        )
        .unwrap();

        assert_eq!(fm.title, "Test Page");
        assert_eq!(
            fm.summary,
            Some("This is a custom summary for the page.".to_string())
        );
    }

    #[test]
    fn test_summary_defaults_to_none() {
        let fm = Frontmatter::from_str(r#"title = "Test""#).unwrap();
        assert!(fm.summary.is_none());
    }

    #[test]
    fn test_summary_with_multiline_text() {
        let fm = Frontmatter::from_str(
            r#"
            title = "Test"
            summary = "A longer summary that spans multiple words and provides context."
            "#,
        )
        .unwrap();

        assert!(fm.summary.is_some());
        assert!(fm.summary.unwrap().contains("longer summary"));
    }

    // ============================================
    // Phase 2.1: Taxonomies Tests
    // ============================================

    #[test]
    fn test_parse_frontmatter_with_tags() {
        let fm = Frontmatter::from_str(
            r#"
            title = "My Blog Post"
            tags = ["rust", "web", "tutorial"]
            "#,
        )
        .unwrap();

        assert_eq!(fm.title, "My Blog Post");
        assert_eq!(fm.tags, vec!["rust", "web", "tutorial"]);
    }

    #[test]
    fn test_tags_default_to_empty() {
        let fm = Frontmatter::from_str(r#"title = "Test""#).unwrap();
        assert!(fm.tags.is_empty());
    }

    #[test]
    fn test_parse_frontmatter_with_categories() {
        let fm = Frontmatter::from_str(
            r#"
            title = "My Blog Post"
            categories = ["Programming", "Web Development"]
            "#,
        )
        .unwrap();

        assert_eq!(fm.title, "My Blog Post");
        assert_eq!(fm.categories, vec!["Programming", "Web Development"]);
    }

    #[test]
    fn test_categories_default_to_empty() {
        let fm = Frontmatter::from_str(r#"title = "Test""#).unwrap();
        assert!(fm.categories.is_empty());
    }

    #[test]
    fn test_parse_frontmatter_with_series() {
        let fm = Frontmatter::from_str(
            r#"
            title = "Part 1: Getting Started"
            series = "Rust Web Development"
            "#,
        )
        .unwrap();

        assert_eq!(fm.title, "Part 1: Getting Started");
        assert_eq!(fm.series, Some("Rust Web Development".to_string()));
    }

    #[test]
    fn test_series_defaults_to_none() {
        let fm = Frontmatter::from_str(r#"title = "Test""#).unwrap();
        assert!(fm.series.is_none());
    }

    #[test]
    fn test_parse_frontmatter_with_all_taxonomies() {
        let fm = Frontmatter::from_str(
            r#"
            title = "Complete Post"
            tags = ["rust", "yew"]
            categories = ["Tutorial"]
            series = "Yew SSG Guide"
            "#,
        )
        .unwrap();

        assert_eq!(fm.tags, vec!["rust", "yew"]);
        assert_eq!(fm.categories, vec!["Tutorial"]);
        assert_eq!(fm.series, Some("Yew SSG Guide".to_string()));
    }

    // ============================================
    // Phase 1.3: Slug Customization Tests
    // ============================================

    #[test]
    fn test_parse_frontmatter_with_slug() {
        let fm = Frontmatter::from_str(
            r#"
            title = "My Blog Post"
            slug = "custom-url-slug"
            "#,
        )
        .unwrap();

        assert_eq!(fm.title, "My Blog Post");
        assert_eq!(fm.slug, Some("custom-url-slug".to_string()));
    }

    #[test]
    fn test_slug_defaults_to_none() {
        let fm = Frontmatter::from_str(r#"title = "Test""#).unwrap();
        assert!(fm.slug.is_none());
    }

    #[test]
    fn test_parse_frontmatter_with_aliases() {
        let fm = Frontmatter::from_str(
            r#"
            title = "My Blog Post"
            aliases = ["/old-url/", "/another-old-path/"]
            "#,
        )
        .unwrap();

        assert_eq!(fm.title, "My Blog Post");
        assert_eq!(fm.aliases, vec!["/old-url/", "/another-old-path/"]);
    }

    #[test]
    fn test_aliases_default_to_empty() {
        let fm = Frontmatter::from_str(r#"title = "Test""#).unwrap();
        assert!(fm.aliases.is_empty());
    }

    #[test]
    fn test_parse_frontmatter_with_slug_and_aliases() {
        let fm = Frontmatter::from_str(
            r#"
            title = "My Blog Post"
            slug = "my-custom-slug"
            aliases = ["/old-path/", "/legacy-url/"]
            "#,
        )
        .unwrap();

        assert_eq!(fm.slug, Some("my-custom-slug".to_string()));
        assert_eq!(fm.aliases, vec!["/old-path/", "/legacy-url/"]);
    }

    // ============================================
    // Phase 3: Pagination Tests
    // ============================================

    #[test]
    fn test_parse_frontmatter_with_sort_by() {
        let fm = Frontmatter::from_str(
            r#"
            title = "Blog Section"
            sort_by = "weight"
            "#,
        )
        .unwrap();

        assert_eq!(fm.title, "Blog Section");
        assert_eq!(fm.sort_by, SortBy::Weight);
    }

    #[test]
    fn test_sort_by_defaults_to_date() {
        let fm = Frontmatter::from_str(r#"title = "Test""#).unwrap();
        assert_eq!(fm.sort_by, SortBy::Date);
    }

    #[test]
    fn test_parse_frontmatter_with_paginate_by() {
        let fm = Frontmatter::from_str(
            r#"
            title = "Blog Section"
            paginate_by = 10
            "#,
        )
        .unwrap();

        assert_eq!(fm.title, "Blog Section");
        assert_eq!(fm.paginate_by, 10);
    }

    #[test]
    fn test_paginate_by_defaults_to_zero() {
        let fm = Frontmatter::from_str(r#"title = "Test""#).unwrap();
        assert_eq!(fm.paginate_by, 0);
    }

    #[test]
    fn test_parse_frontmatter_with_paginate_template() {
        let fm = Frontmatter::from_str(
            r#"
            title = "Blog Section"
            paginate_by = 10
            paginate_template = "blog-page.html"
            "#,
        )
        .unwrap();

        assert_eq!(fm.paginate_by, 10);
        assert_eq!(fm.paginate_template, Some("blog-page.html".to_string()));
    }

    #[test]
    fn test_parse_frontmatter_with_weight() {
        let fm = Frontmatter::from_str(
            r#"
            title = "My Page"
            weight = 5
            "#,
        )
        .unwrap();

        assert_eq!(fm.title, "My Page");
        assert_eq!(fm.weight, 5);
    }

    #[test]
    fn test_weight_defaults_to_zero() {
        let fm = Frontmatter::from_str(r#"title = "Test""#).unwrap();
        assert_eq!(fm.weight, 0);
    }

    #[test]
    fn test_parse_frontmatter_with_updated_date() {
        let fm = Frontmatter::from_str(
            r#"
            title = "My Page"
            date = 2024-01-15
            updated = 2024-02-20
            "#,
        )
        .unwrap();

        assert_eq!(fm.date, Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()));
        assert_eq!(
            fm.updated,
            Some(NaiveDate::from_ymd_opt(2024, 2, 20).unwrap())
        );
    }

    #[test]
    fn test_updated_defaults_to_none() {
        let fm = Frontmatter::from_str(r#"title = "Test""#).unwrap();
        assert!(fm.updated.is_none());
    }

    #[test]
    fn test_parse_frontmatter_with_all_pagination_fields() {
        let fm = Frontmatter::from_str(
            r#"
            title = "Blog"
            sort_by = "title"
            paginate_by = 5
            paginate_template = "blog-paginated.html"
            weight = 10
            "#,
        )
        .unwrap();

        assert_eq!(fm.sort_by, SortBy::Title);
        assert_eq!(fm.paginate_by, 5);
        assert_eq!(
            fm.paginate_template,
            Some("blog-paginated.html".to_string())
        );
        assert_eq!(fm.weight, 10);
    }

    #[test]
    fn test_sort_by_all_variants() {
        let fm = Frontmatter::from_str(
            r#"
            title = "Test"
            sort_by = "date"
            "#,
        )
        .unwrap();
        assert_eq!(fm.sort_by, SortBy::Date);

        let fm = Frontmatter::from_str(
            r#"
            title = "Test"
            sort_by = "title"
            "#,
        )
        .unwrap();
        assert_eq!(fm.sort_by, SortBy::Title);

        let fm = Frontmatter::from_str(
            r#"
            title = "Test"
            sort_by = "weight"
            "#,
        )
        .unwrap();
        assert_eq!(fm.sort_by, SortBy::Weight);

        let fm = Frontmatter::from_str(
            r#"
            title = "Test"
            sort_by = "none"
            "#,
        )
        .unwrap();
        assert_eq!(fm.sort_by, SortBy::None);
    }
}
