//! Atom feed generation.
//!
//! This module provides functions for generating Atom feeds.

use chrono::{DateTime, Utc};

use super::{FeedConfig, FeedEntry};
use crate::error::Result;

/// Generate an Atom feed from entries.
pub fn generate_atom_feed(entries: &[FeedEntry], config: &FeedConfig) -> Result<String> {
    let now: DateTime<Utc> = Utc::now();
    let now_rfc3339 = now.to_rfc3339();

    let mut entries_xml = String::new();
    for entry in entries {
        entries_xml.push_str(&generate_entry_xml(entry, config)?);
    }

    let author_xml = match (&config.author, &config.author_email) {
        (Some(name), Some(email)) => format!(
            "  <author>\n    <name>{}</name>\n    <email>{}</email>\n  </author>\n",
            escape_xml(name),
            escape_xml(email)
        ),
        (Some(name), None) => format!(
            "  <author>\n    <name>{}</name>\n  </author>\n",
            escape_xml(name)
        ),
        (None, Some(email)) => format!(
            "  <author>\n    <email>{}</email>\n  </author>\n",
            escape_xml(email)
        ),
        (None, None) => String::new(),
    };

    let atom = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>{}</title>
  <link href="{}"/>
  <link href="{}/{}.atom" rel="self"/>
  <updated>{}</updated>
  <id>{}</id>
{}
  <subtitle>{}</subtitle>
{}
</feed>"#,
        escape_xml(&config.title),
        config.base_url,
        config.base_url.trim_end_matches('/'),
        config.filename,
        now_rfc3339,
        config.base_url,
        author_xml,
        escape_xml(&config.description),
        entries_xml
    );

    Ok(atom)
}

/// Generate XML for a single Atom entry.
fn generate_entry_xml(entry: &FeedEntry, config: &FeedConfig) -> Result<String> {
    let published = entry.date.to_rfc3339();
    let updated = entry
        .updated
        .map(|d| d.to_rfc3339())
        .unwrap_or_else(|| published.clone());

    let author_xml = match (&entry.author, &entry.author_email) {
        (Some(name), Some(email)) => format!(
            "    <author>\n      <name>{}</name>\n      <email>{}</email>\n    </author>\n",
            escape_xml(name),
            escape_xml(email)
        ),
        (Some(name), None) => format!(
            "    <author>\n      <name>{}</name>\n    </author>\n",
            escape_xml(name)
        ),
        (None, Some(email)) => format!(
            "    <author>\n      <email>{}</email>\n    </author>\n",
            escape_xml(email)
        ),
        (None, None) => {
            // Use feed author if available
            match (&config.author, &config.author_email) {
                (Some(name), Some(email)) => format!(
                    "    <author>\n      <name>{}</name>\n      <email>{}</email>\n    </author>\n",
                    escape_xml(name),
                    escape_xml(email)
                ),
                (Some(name), None) => format!(
                    "    <author>\n      <name>{}</name>\n    </author>\n",
                    escape_xml(name)
                ),
                (None, Some(email)) => format!(
                    "    <author>\n      <email>{}</email>\n    </author>\n",
                    escape_xml(email)
                ),
                (None, None) => String::new(),
            }
        }
    };

    let category_xml = entry
        .tags
        .iter()
        .map(|tag| format!("    <category term=\"{}\"/>\n", escape_xml(tag)))
        .collect::<String>();

    let content_xml = if let Some(content) = &entry.content {
        if config.full_content {
            format!(
                "    <content type=\"html\"><![CDATA[{}]]></content>\n",
                content
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    Ok(format!(
        r#"  <entry>
    <title>{}</title>
    <link href="{}"/>
    <id>{}</id>
    <published>{}</published>
    <updated>{}</updated>
    <summary>{}</summary>
{}{}{}  </entry>
"#,
        escape_xml(&entry.title),
        entry.url,
        entry.url,
        published,
        updated,
        escape_xml(&entry.summary),
        author_xml,
        category_xml,
        content_xml
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
    fn test_generate_atom_feed() {
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

        let atom = generate_atom_feed(&entries, &config).unwrap();

        assert!(atom.contains("<?xml"));
        assert!(atom.contains("<feed"));
        assert!(atom.contains("Test Blog"));
        assert!(atom.contains("First Post"));
        assert!(atom.contains("Second Post"));
        assert!(atom.contains("https://example.com"));
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
