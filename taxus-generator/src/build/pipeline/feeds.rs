// taxus-generator/src/build/pipeline/generate_feeds.rs

use crate::build::ProcessedPage;
use crate::config::SiteConfig;
use crate::error::{GeneratorError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use taxus_domain::derivation::recent;
use taxus_domain::{NodePath, PageNode, SiteTree};
use tracing::{debug, info, warn};

/// Generated feed file.
#[derive(Debug, Clone)]
pub struct GeneratedFeed {
    /// Feed filename (e.g., "feed.xml", "atom.xml")
    pub filename: String,
    /// Feed content (XML)
    pub content: String,
}

/// The pages a feed syndicates, newest first.
///
/// Feeds are for dated content (#44): the candidates are
/// [`recent`] — every non-draft page, section indexes excluded — narrowed
/// to pages that carry a `date`, and to pages under one of `sections`
/// when that scope is configured. An undated page has no publication
/// date to syndicate; the feed generator would stamp it with the build
/// time, which re-announces it to subscribers on every build.
///
/// A `sections` entry that is not a valid path or names no section is
/// skipped with a warning.
pub fn feed_pages<'a>(tree: &'a SiteTree, sections: &[String]) -> Vec<&'a PageNode> {
    let scopes: Vec<NodePath> = sections
        .iter()
        .filter_map(|raw| match NodePath::parse(raw) {
            Ok(path) if tree.get_section(&path).is_some() => Some(path),
            Ok(_) => {
                warn!(section = %raw, "[feed] sections names a section that does not exist");
                None
            }
            Err(e) => {
                warn!(section = %raw, error = %e, "[feed] sections entry is not a valid path");
                None
            }
        })
        .collect();

    recent(tree, false)
        .into_iter()
        .filter(|p| p.meta.date.is_some())
        .filter(|p| {
            sections.is_empty()
                || scopes
                    .iter()
                    .any(|scope| p.path.segments().starts_with(scope.segments()))
        })
        .collect()
}

/// Generate RSS and Atom feeds.
///
/// Membership and order come from the tree via [`feed_pages`]: dated,
/// non-draft pages, newest first, optionally scoped to `[feed] sections`.
/// Each is joined to its rendered `ProcessedPage` by content file, and the
/// feed generator applies the configured limit.
pub fn generate_feeds(
    tree: &SiteTree,
    processed: &[ProcessedPage],
    config: &SiteConfig,
) -> Result<Vec<GeneratedFeed>> {
    use crate::feed::{FeedConfig as FeedGenConfig, FeedGenerator};

    let mut feeds = Vec::new();

    // Skip if both feeds are disabled
    if !config.feed.rss_enabled && !config.feed.atom_enabled {
        return Ok(feeds);
    }

    let processed_by_file: HashMap<&Path, &ProcessedPage> = processed
        .iter()
        .map(|p| (p.route.content_file.as_path(), p))
        .collect();

    // Collect pages for feed generation
    let pages: Vec<crate::content::Page> = feed_pages(tree, &config.feed.sections)
        .iter()
        .filter_map(|n| processed_by_file.get(n.content_file.as_path()))
        .map(|p| {
            let mut page = p.page.clone();
            // Update the page path to the effective URL (custom slug
            // overrides the discovered route path) so the feed entry
            // links where the page actually lives.
            let url_path = p.effective_url_path();
            page.path = url_path;
            // Set content for full-content feeds
            if config.feed.full_content {
                page.content = Some(p.html_content.clone());
            }
            page
        })
        .collect();

    // Build feed generator config
    let feed_gen_config = FeedGenConfig {
        title: config
            .feed
            .title
            .clone()
            .unwrap_or_else(|| config.site.name.clone()),
        base_url: config.site.base_url.clone(),
        description: config.site.description.clone().unwrap_or_default(),
        author: config.site.author.clone(),
        limit: if config.feed.limit > 0 {
            config.feed.limit
        } else {
            20
        },
        full_content: config.feed.full_content,
        ..Default::default()
    };

    let generator = FeedGenerator::new(feed_gen_config);

    // Generate RSS feed if enabled
    if config.feed.rss_enabled {
        let rss_content = generator.generate_rss(&pages)?;
        let filename = config
            .feed
            .rss_path
            .clone()
            .unwrap_or_else(|| generator.rss_filename());
        feeds.push(GeneratedFeed {
            filename,
            content: rss_content,
        });
        info!("Generated RSS feed");
    }

    // Generate Atom feed if enabled
    if config.feed.atom_enabled {
        let atom_content = generator.generate_atom(&pages)?;
        let filename = config
            .feed
            .atom_path
            .clone()
            .unwrap_or_else(|| generator.atom_filename());
        feeds.push(GeneratedFeed {
            filename,
            content: atom_content,
        });
        info!("Generated Atom feed");
    }

    Ok(feeds)
}

/// Write feed files to output directory.
pub fn write_feeds(feeds: &[GeneratedFeed], output_dir: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        debug!("Dry run - skipping feed writes");
        return Ok(());
    }

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir).map_err(|e| GeneratorError::Io {
        path: output_dir.to_path_buf(),
        source: e,
    })?;

    for feed in feeds {
        let output_path = output_dir.join(&feed.filename);

        // Write the feed file
        fs::write(&output_path, &feed.content).map_err(|e| GeneratorError::Io {
            path: output_path.clone(),
            source: e,
        })?;

        debug!(
            path = %output_path.display(),
            "Written feed file"
        );
    }

    if !feeds.is_empty() {
        info!("Wrote {} feed files", feeds.len());
    }

    Ok(())
}

#[test]
fn test_write_feeds_creates_output_dir() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    // Use a non-existent output directory
    let output_dir = temp_dir.path().join("dist");

    // Ensure output directory doesn't exist
    assert!(!output_dir.exists());

    let feeds = vec![GeneratedFeed {
        filename: "feed.xml".to_string(),
        content: r#"<?xml version="1.0" encoding="UTF-8"?><rss></rss>"#.to_string(),
    }];

    // Write feeds - should create the directory
    let result = write_feeds(&feeds, &output_dir, false);
    assert!(result.is_ok());

    // Verify directory was created
    assert!(output_dir.exists());

    // Verify file was written
    assert!(output_dir.join("feed.xml").exists());
}

#[test]
fn test_write_feeds_dry_run() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("dist");

    let feeds = vec![GeneratedFeed {
        filename: "feed.xml".to_string(),
        content: r#"<?xml version="1.0" encoding="UTF-8"?><rss></rss>"#.to_string(),
    }];

    // Dry run should not write anything
    let result = write_feeds(&feeds, &output_dir, true);
    assert!(result.is_ok());

    // Verify directory was NOT created
    assert!(!output_dir.exists());
}

#[test]
fn test_write_feeds_empty() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().join("dist");

    // Empty feeds list should succeed
    let result = write_feeds(&[], &output_dir, false);
    assert!(result.is_ok());
}

#[cfg(test)]
mod feed_membership_tests {
    use super::*;
    use crate::build::pipeline::test_support::tree_of;
    use crate::content::Page;
    use crate::routes::{RouteInfo, RouteKind};
    use chrono::NaiveDate;
    use std::path::PathBuf;
    use taxus_domain::Frontmatter;

    fn node(path: &str, file: &str, kind: RouteKind, fm: Frontmatter) -> ProcessedPage {
        let output = if path == "/" {
            "index.html".to_string()
        } else {
            format!("{}/index.html", path.trim_matches('/'))
        };
        ProcessedPage {
            toc: Vec::new(),
            route: RouteInfo::new(
                path.to_string(),
                PathBuf::from(file),
                PathBuf::from(output),
                kind,
            )
            .unwrap(),
            page: Page {
                frontmatter: fm,
                path: path.to_string(),
                source: PathBuf::from(file),
                raw_content: String::new(),
                content: None,
            },
            html_content: String::new(),
            hero_image: None,
        }
    }

    fn section(path: &str, file: &str, date: Option<&str>) -> ProcessedPage {
        node(
            path,
            file,
            RouteKind::Section,
            Frontmatter {
                title: path.to_string(),
                date: date.map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap()),
                ..Frontmatter::default()
            },
        )
    }

    fn page(path: &str, file: &str, title: &str, date: Option<&str>) -> ProcessedPage {
        node(
            path,
            file,
            RouteKind::Page,
            Frontmatter {
                title: title.to_string(),
                date: date.map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap()),
                ..Frontmatter::default()
            },
        )
    }

    /// A site with a dated section index, an undated static page, a draft,
    /// and dated posts in two sections — everything #44 wants excluded or
    /// ordered.
    fn site() -> Vec<ProcessedPage> {
        let mut draft = page("/blog/wip/", "blog/wip.md", "WIP", Some("2026-05-01"));
        draft.page.frontmatter.draft = true;
        vec![
            section("/", "_index.md", None),
            section("/blog/", "blog/_index.md", Some("2024-01-01")),
            section("/notes/", "notes/_index.md", None),
            page("/about/", "about.md", "About", None),
            page("/blog/old/", "blog/old.md", "Old", Some("2026-01-01")),
            page("/blog/new/", "blog/new.md", "New", Some("2026-03-01")),
            page("/notes/mid/", "notes/mid.md", "Mid", Some("2026-02-01")),
            draft,
        ]
    }

    fn titles(pages: &[&PageNode]) -> Vec<String> {
        pages.iter().map(|p| p.meta.title.clone()).collect()
    }

    #[test]
    fn feed_pages_are_dated_non_draft_pages_newest_first() {
        let processed = site();
        let tree = tree_of(&processed);
        assert_eq!(titles(&feed_pages(&tree, &[])), ["New", "Mid", "Old"]);
    }

    #[test]
    fn feed_pages_scoped_to_configured_sections() {
        let processed = site();
        let tree = tree_of(&processed);
        assert_eq!(
            titles(&feed_pages(&tree, &["blog".to_string()])),
            ["New", "Old"]
        );
        // Unknown and invalid entries are skipped, not fatal.
        assert_eq!(
            titles(&feed_pages(
                &tree,
                &["missing".to_string(), "..".to_string(), "notes".to_string()]
            )),
            ["Mid"]
        );
    }

    #[test]
    fn generated_rss_lists_only_feed_pages() {
        let processed = site();
        let tree = tree_of(&processed);
        let mut config = SiteConfig::new("Test", "https://example.com");
        config.feed.rss_enabled = true;
        config.feed.atom_enabled = false;
        let feeds = generate_feeds(&tree, &processed, &config).unwrap();
        assert_eq!(feeds.len(), 1);
        let rss = &feeds[0].content;
        let links: Vec<&str> = rss
            .split("<link>")
            .skip(1)
            .map(|rest| rest.split("</link>").next().unwrap())
            .collect();
        // The channel link, then the entries newest first.
        assert_eq!(
            links,
            [
                "https://example.com",
                "https://example.com/blog/new/",
                "https://example.com/notes/mid/",
                "https://example.com/blog/old/",
            ]
        );
        assert!(!rss.contains("/about/"), "undated page in feed: {rss}");
        assert!(!rss.contains("/blog/wip/"), "draft in feed: {rss}");
    }
}
