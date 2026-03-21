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

    /// Custom extra metadata
    pub extra: Option<toml::Value>,
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
    /// use yew_ssg_lib::content::Frontmatter;
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
}
