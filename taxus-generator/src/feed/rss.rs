// taxus-generator/src/feed/rss.rs

//! RSS 2.0 feed generation.
//!
//! This module provides functions for generating RSS 2.0 feeds.

use super::{FeedConfig, FeedEntry, escape_xml};
use crate::error::Result;
use chrono::{DateTime, Utc};

/// Generate an RSS 2.0 feed from entries.
pub fn generate_rss_feed(entries: &[FeedEntry], config: &FeedConfig) -> Result<String> {
    // #38: the newest entry's date, not `Utc::now()` — a build that changed
    // no content must produce a byte-identical feed.
    let now: DateTime<Utc> = entries
        .iter()
        .map(|e| e.date)
        .max()
        .unwrap_or_else(Utc::now);
    let now_rfc2822 = now.format("%a, %d %b %Y %H:%M:%S %z").to_string();

    let mut items_xml = String::new();
    for entry in entries {
        items_xml.push_str(&generate_item_xml(entry, config)?);
    }

    let rss = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom" xmlns:content="http://purl.org/rss/1.0/modules/content/">
<channel>
    <title>{}</title>
    <link>{}</link>
    <description>{}</description>
    <language>{}</language>
    <lastBuildDate>{}</lastBuildDate>
    <atom:link href="{}" rel="self" type="application/rss+xml"/>
{}
</channel>
</rss>"#,
        escape_xml(&config.title),
        escape_xml(&config.base_url),
        escape_xml(&config.description),
        escape_xml(&config.language),
        now_rfc2822,
        escape_xml(&format!(
            "{}/{}.xml",
            config.base_url.trim_end_matches('/'),
            config.filename
        )),
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
                escape_cdata(content)
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let guid = format!(
        "    <guid isPermaLink=\"true\">{}</guid>\n",
        escape_xml(&entry.url)
    );

    Ok(format!(
        r#"    <item>
        <title>{}</title>
        <link>{}</link>
        <description>{}</description>
        <pubDate>{}</pubDate>
{}{}{}{}    </item>
"#,
        escape_xml(&entry.title),
        escape_xml(&entry.url),
        escape_xml(&entry.summary),
        pub_date,
        author_xml,
        category_xml,
        content_xml,
        guid
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
    use chrono::TimeZone;

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

    /// #38: the feed must be well-formed XML even when titles, URLs and
    /// content carry XML metacharacters, and the `content:` namespace
    /// must be declared.
    #[test]
    fn rss_with_metacharacters_is_well_formed_xml() {
        let config = FeedConfig {
            title: "A & B <blog>".to_string(),
            description: "Stuff & nonsense".to_string(),
            base_url: "https://example.com".to_string(),
            full_content: true,
            ..Default::default()
        };

        let entries = vec![FeedEntry {
            title: "Tom & Jerry <the cat>".to_string(),
            url: "https://example.com/tom-jerry/?a=1&b=2".to_string(),
            summary: "An & summary".to_string(),
            content: Some("<p>If x ]]&gt; 3 then &amp; so</p>".to_string()),
            date: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            updated: None,
            author: None,
            author_email: None,
            tags: vec!["a & b".to_string()],
        }];

        let rss = generate_rss_feed(&entries, &config).unwrap();

        assert!(
            rss.contains(r#"xmlns:content="http://purl.org/rss/1.0/modules/content/""#),
            "content namespace must be declared"
        );

        // A real parser, not contains() (#38): rejects unescaped `&`,
        // undeclared prefixes, and early-terminated CDATA.
        let mut reader = quick_xml::Reader::from_str(&rss);
        reader.config_mut().check_end_names = true;
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => buf.clear(),
                Err(e) => panic!("feed is not well-formed XML: {e}\n{rss}"),
            }
        }

        // The escaped URL round-trips through the parser to the original.
        assert!(rss.contains("<link>https://example.com/tom-jerry/?a=1&amp;b=2</link>"));
    }

    /// #38: `]]>` inside content must not terminate the CDATA section
    /// early — the parser above would reject the dangling tail, and the
    /// extracted text must contain the original sequence.
    #[test]
    fn cdata_closer_in_content_is_neutralised() {
        let config = FeedConfig {
            title: "T".to_string(),
            base_url: "https://example.com".to_string(),
            full_content: true,
            ..Default::default()
        };

        let entries = vec![FeedEntry {
            title: "Arrays".to_string(),
            url: "https://example.com/arrays/".to_string(),
            summary: "s".to_string(),
            content: Some("<pre>a[i]]>0 is the guard</pre>".to_string()),
            date: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            updated: None,
            author: None,
            author_email: None,
            tags: vec![],
        }];

        let rss = generate_rss_feed(&entries, &config).unwrap();

        let mut reader = quick_xml::Reader::from_str(&rss);
        let mut buf = Vec::new();
        let mut text = String::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(quick_xml::events::Event::CData(d)) => {
                    text.push_str(d.as_ref());
                }
                Ok(_) => {}
                Err(e) => panic!("feed is not well-formed XML: {e}\n{rss}"),
            }
            buf.clear();
        }
        assert!(
            text.contains("a[i]]>0"),
            "CDATA content must round-trip; got {text:?}"
        );
    }

    /// #38: `lastBuildDate` is the newest entry date, so an unchanged
    /// build produces an unchanged feed.
    #[test]
    fn last_build_date_is_newest_entry_date() {
        let config = FeedConfig {
            base_url: "https://example.com".to_string(),
            ..Default::default()
        };

        let mut older = create_test_entry("Older");
        older.date = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let mut newer = create_test_entry("Newer");
        newer.date = Utc.with_ymd_and_hms(2026, 3, 15, 9, 30, 0).unwrap();

        let rss = generate_rss_feed(&[older, newer], &config).unwrap();

        assert!(rss.contains("Sun, 15 Mar 2026 09:30:00 +0000"));
    }

    #[test]
    fn empty_feed_falls_back_to_now_for_last_build_date() {
        let config = FeedConfig {
            base_url: "https://example.com".to_string(),
            ..Default::default()
        };

        let rss = generate_rss_feed(&[], &config).unwrap();
        assert!(rss.contains("<lastBuildDate>"));
    }
}
