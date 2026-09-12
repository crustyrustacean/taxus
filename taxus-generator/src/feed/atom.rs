// taxus-generator/src/feed/atom.rs

//! Atom feed generation.
//!
//! This module provides functions for generating Atom feeds.

use super::{FeedConfig, FeedEntry, escape_xml};
use crate::error::Result;
use chrono::{DateTime, Utc};

/// Generate an Atom feed from entries.
pub fn generate_atom_feed(entries: &[FeedEntry], config: &FeedConfig) -> Result<String> {
    // #38: the newest entry's updated date, not `Utc::now()` — a build
    // that changed no content must produce a byte-identical feed.
    let now: DateTime<Utc> = entries
        .iter()
        .map(|e| e.updated.unwrap_or(e.date))
        .max()
        .unwrap_or_else(Utc::now);
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
  <link href="{}" rel="self"/>
  <updated>{}</updated>
  <id>{}</id>
{}
  <subtitle>{}</subtitle>
{}
</feed>"#,
        escape_xml(&config.title),
        escape_xml(&config.base_url),
        escape_xml(&format!(
            "{}/{}.atom",
            config.base_url.trim_end_matches('/'),
            config.filename
        )),
        now_rfc3339,
        escape_xml(&config.base_url),
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
                escape_cdata(content)
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
        escape_xml(&entry.url),
        escape_xml(&entry.url),
        published,
        updated,
        escape_xml(&entry.summary),
        author_xml,
        category_xml,
        content_xml
    ))
}

/// Make text safe to embed inside a `<![CDATA[ … ]]>` section: the
/// sequence `]]>` terminates the section early (#38), so each occurrence
/// is split into `]]]]><![CDATA[>` — close, escaped tail, reopen.
fn escape_cdata(s: &str) -> String {
    s.replace("]]>", "]]]]><![CDATA[>")
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

    /// #38: well-formed XML even when URLs and content carry
    /// metacharacters; `updated` reflects the newest entry, not the
    /// build time.
    #[test]
    fn atom_with_metacharacters_is_well_formed_xml() {
        use chrono::TimeZone;

        let config = FeedConfig {
            title: "A & B".to_string(),
            base_url: "https://example.com".to_string(),
            full_content: true,
            ..Default::default()
        };

        let entries = vec![FeedEntry {
            title: "Q & A".to_string(),
            url: "https://example.com/q-a/?x=1&y=2".to_string(),
            summary: "s".to_string(),
            content: Some("<pre>idx]]>size</pre>".to_string()),
            date: Utc.with_ymd_and_hms(2026, 2, 2, 0, 0, 0).unwrap(),
            updated: Some(Utc.with_ymd_and_hms(2026, 3, 3, 0, 0, 0).unwrap()),
            author: None,
            author_email: None,
            tags: vec![],
        }];

        let atom = generate_atom_feed(&entries, &config).unwrap();

        let mut reader = quick_xml::Reader::from_str(&atom);
        reader.config_mut().check_end_names = true;
        let mut buf = Vec::new();
        let mut cdata_text = String::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(quick_xml::events::Event::CData(d)) => {
                    cdata_text.push_str(d.as_ref());
                }
                Ok(_) => {}
                Err(e) => panic!("feed is not well-formed XML: {e}\n{atom}"),
            }
            buf.clear();
        }
        assert!(
            cdata_text.contains("idx]]>size"),
            "CDATA content must round-trip; got {cdata_text:?}"
        );
        assert!(atom.contains("2026-03-03"));
    }
}
