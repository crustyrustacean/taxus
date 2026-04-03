//! RSS 2.0 feed generation.
//!
//! This module provides functions for generating RSS 2.0 feeds.

use chrono::{DateTime, Utc};

use super::{FeedConfig, FeedEntry};
use crate::error::Result;

/// Generate an RSS 2.0 feed from entries.
pub fn generate_rss_feed(entries: &[FeedEntry], config: &FeedConfig) -> Result<String> {
    let now: DateTime<Utc> = Utc::now();
    let now_rfc2822 = now.format("%a, %d %b %Y %H:%M:%S %z").to_string();

    let mut items_xml = String::new();
    for entry in entries {
        items_xml.push_str(&generate_item_xml(entry, config)?);
    }

    let rss = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
<channel>
    <title>{}</title>
    <link>{}</link>
    <description>{}</description>
    <language>{}</language>
    <lastBuildDate>{}</lastBuildDate>
    <atom:link href="{}/{}.xml" rel="self" type="application/rss+xml"/>
{}
</channel>
</rss>"#,
        escape_xml(&config.title),
        config.base_url,
        escape_xml(&config.description),
        config.language,
        now_rfc2822,
        config.base_url.trim_end_matches('/'),
        config.filename,
        items_xml
    );

    Ok(rss)
}

/// Generate XML for a single RSS item.
fn generate_item_xml(entry: &FeedEntry, config: &FeedConfig) -> Result<String> {
    let pub_date = entry.date.format("%a, %d %b %Y %H:%M:%S %z").to_string();

    let author_xml = match (&entry.author, &entry.author_email) {
        (Some(name), Some(email)) => format!(
            "    <author>{}</author>\n",
            escape_xml(&format!("{} ({})", email, name))
        ),
        (Some(name), None) => format!("    <author>{}</author>\n", escape_xml(name)),
        (None, Some(email)) => format!("    <author>{}</author>\n", escape_xml(email)),
        (None, None) => {
            // Use feed author if available
            match (&config.author, &config.author_email) {
                (Some(name), Some(email)) => format!(
                    "    <author>{}</author>\n",
                    escape_xml(&format!("{} ({})", email, name))
                ),
                (Some(name), None) => format!("    <author>{}</author>\n", escape_xml(name)),
                (None, Some(email)) => format!("    <author>{}</author>\n", escape_xml(email)),
                (None, None) => String::new(),
            }
        }
    };

    let category_xml = entry
        .tags
        .iter()
        .map(|tag| format!("    <category>{}</category>\n", escape_xml(tag)))
        .collect::<String>();

    let content_xml = if let Some(content) = &entry.content {
        if config.full_content {
            format!(
                "    <content:encoded><![CDATA[{}]]></content:encoded>\n",
                content
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let guid = format!("    <guid isPermaLink=\"true\">{}</guid>\n", entry.url);

    Ok(format!(
        r#"    <item>
        <title>{}</title>
        <link>{}</link>
        <description>{}</description>
        <pubDate>{}</pubDate>
{}{}{}{}    </item>
"#,
        escape_xml(&entry.title),
        entry.url,
        escape_xml(&entry.summary),
        pub_date,
        author_xml,
        category_xml,
        content_xml,
        guid
    ))
}

/// Escape special XML characters.
fn escape_xml(s: &str) -> String {
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

    fn create_test_entry(title: &str) -> FeedEntry {
        FeedEntry {
            title: title.to_string(),
            url: format!(
                "https://example.com/{}/",
                title.to_lowercase().replace(' ', "-")
            ),
            summary: format!("Summary for {}", title),
            content: Some(format!("<p>Content for {}</p>", title)),
            date: Utc::now(),
            updated: None,
            author: Some("Test Author".to_string()),
            author_email: Some("test@example.com".to_string()),
            tags: vec!["rust".to_string(), "programming".to_string()],
        }
    }

    #[test]
    fn test_generate_rss_feed() {
        let config = FeedConfig {
            title: "Test Blog".to_string(),
            description: "A test blog".to_string(),
            base_url: "https://example.com".to_string(),
            language: "en".to_string(),
            ..Default::default()
        };

        let entries = vec![
            create_test_entry("First Post"),
            create_test_entry("Second Post"),
        ];

        let rss = generate_rss_feed(&entries, &config).unwrap();

        assert!(rss.contains("<?xml"));
        assert!(rss.contains("<rss"));
        assert!(rss.contains("Test Blog"));
        assert!(rss.contains("First Post"));
        assert!(rss.contains("Second Post"));
        assert!(rss.contains("https://example.com"));
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("a & b"), "a \u{26}amp; b");
        assert_eq!(escape_xml("<tag>"), "\u{26}lt;tag\u{26}gt;");
        // Test quote escaping
        let escaped = escape_xml("\"quoted\"");
        assert!(escaped.contains("\u{26}quot;"));
    }
}
